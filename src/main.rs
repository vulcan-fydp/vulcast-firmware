use std::io::Read;
use std::sync::{Arc, Mutex};

use graphql::backend_query;
use graphql::signal_query;

use anyhow::{anyhow, Result};
use backend_query::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::{
    AuthenticationError, RelayAssignment, VulcastAssignedToRelayError,
};
use backend_query::log_in_as_vulcast::LogInAsVulcastLogInAsVulcast::{
    AuthenticationError as LoginAuthenticationError, VulcastAuthentication,
};
use clap::Parser;
use controllers::Controllers;
use controllers::NsProcons;
use futures::StreamExt;
use graphql_client::{GraphQLQuery, Response};
use graphql_ws::GraphQLWebSocket;
use http::Uri;
use ini::Ini;
use serde::Serialize;
use std::convert::TryInto;
use tokio::net::TcpStream;
use tokio_tungstenite::Connector;
use vulcast_rtc::broadcaster::Broadcaster;
use vulcast_rtc::types::*;

use crate::graphql_signaller::GraphQLSignaller;

mod cmdline;
mod controllers;
mod data_streamer;
mod graphql;
mod graphql_signaller;

use cmdline::Opts;

#[derive(Serialize)]
struct SessionToken {
    token: String,
}

async fn login(conf: &Ini, client: &reqwest::Client) -> Result<String> {
    log::info!("Logging in");

    let guid = conf
        .get_from(Some("auth"), "guid")
        .expect("GUID missing from config")
        .to_owned();
    let secret = conf
        .get_from(Some("auth"), "secret")
        .expect("Secret missing from config")
        .to_owned();
    let uri = conf
        .get_from(Some("network"), "backend_addr")
        .expect("No backend address specified")
        .to_owned()
        + "/graphql";

    let login_query =
        backend_query::LogInAsVulcast::build_query(backend_query::log_in_as_vulcast::Variables {
            vulcast_id: guid,
            secret,
        });
    let auth = client.post(&uri).json(&login_query).send().await?;
    let response_body: Response<backend_query::log_in_as_vulcast::ResponseData> =
        auth.json().await?;
    if let Some(errors) = response_body.errors {
        errors.iter().for_each(|error| log::error!("{:?}", error))
    }
    let response_data: backend_query::log_in_as_vulcast::ResponseData = response_body
        .data
        .ok_or_else(|| anyhow!("Request returned no data"))?;
    match response_data.log_in_as_vulcast {
        VulcastAuthentication(auth) => Ok(auth.vulcast_access_token),
        LoginAuthenticationError(error) => Err(anyhow!("Authentication error: {}", error.message)),
    }
}

fn write_relay_assignment(hostname: &str, token: &str, opts: &Opts) -> Result<()> {
    log::info!("Writing relay assignment...");
    let mut assigned = Ini::new();
    assigned
        .with_section(Some("relay"))
        .set("hostname", hostname)
        .set("token", token);
    assigned.write_to_file(opts.config_dir.clone() + "/assigned_relay")?;
    Ok(())
}

fn read_relay_assignment(opts: &Opts) -> Result<(String, String)> {
    log::info!("Reading relay assignment...");
    let relay_file = Ini::load_from_file(opts.config_dir.clone() + "/assigned_relay")?;
    let host = relay_file
        .get_from(Some("relay"), "hostname")
        .ok_or_else(|| anyhow!("Could not load relay hostname from file"))?;
    let token = relay_file
        .get_from(Some("relay"), "token")
        .ok_or_else(|| anyhow!("Could not load relay token from file"))?;
    Ok((host.to_owned(), token.to_owned()))
}

async fn assign_relay(
    conf: &Ini,
    opts: &Opts,
    client: &reqwest::Client,
    auth_token: &str,
) -> Result<(String, String)> {
    log::info!("Requesting relay assignment");

    let uri = conf
        .get_from(Some("network"), "backend_addr")
        .expect("No backend address specified")
        .to_owned()
        + "/graphql";

    let register_query = backend_query::AssignVulcastToRelay::build_query(
        backend_query::assign_vulcast_to_relay::Variables {},
    );
    let res = client
        .post(&uri)
        .bearer_auth("vulcast_".to_owned() + auth_token)
        .json(&register_query)
        .send()
        .await?;

    let response_body: Response<backend_query::assign_vulcast_to_relay::ResponseData> =
        res.json().await?;
    if let Some(errors) = response_body.errors {
        errors.iter().for_each(|error| log::error!("{:?}", error))
    }
    let response_data: backend_query::assign_vulcast_to_relay::ResponseData = response_body
        .data
        .ok_or_else(|| anyhow!("Request returned no data"))?;
    match response_data.assign_vulcast_to_relay {
        RelayAssignment(assignment) => {
            let _ = write_relay_assignment(
                &assignment.relay.host_name,
                &assignment.relay_access_token,
                &opts,
            );
            Ok((assignment.relay.host_name, assignment.relay_access_token))
        }
        AuthenticationError(error) => Err(anyhow!("Authentication error: {}", error.message)),
        VulcastAssignedToRelayError(error) => {
            Err(anyhow!("Vulcast already assigned error: {}", error.message))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default());

    let opts: Opts = Opts::parse();

    let controllers = {
        if !opts.no_controller {
            log::info!("Setting up controller emulator...");
            let mut controllers = NsProcons::new("procons");
            controllers.initialize()?;
            Some(Arc::new(Mutex::new(controllers)))
        } else {
            None
        }
    };

    log::info!("Loading config from {}", opts.config_dir);
    let conf = Ini::load_from_file(opts.config_dir.clone() + "/vulcast.conf").expect(&format!(
        "Couldn't open config file: {}/vulcast.conf",
        &opts.config_dir
    ));
    let client = reqwest::Client::new();

    let access_token = login(&conf, &client).await?;
    let (relay_host, relay_token) = assign_relay(&conf, &opts, &client, &access_token)
        .await
        .or_else(|_| read_relay_assignment(&opts))?;

    log::info!("Assigned to relay {:?}", relay_host);

    let port: u16 = conf
        .get_from(Some("network"), "signal_port")
        .expect("Signal port not specified")
        .parse()
        .expect("Signal port could not be parsed as an int");
    let relay_uri: Uri = format!("wss://{}:{}", relay_host, port).parse().unwrap();

    log::info!("Connecting to relay at {:?}", relay_uri);

    let stream = TcpStream::connect((relay_host.clone(), port)).await?;
    let req = http::Request::builder()
        .uri(relay_uri)
        .header("Sec-WebSocket-Protocol", "graphql-ws")
        .body(())?;

    struct PromiscuousServerVerifier;
    impl rustls::client::ServerCertVerifier for PromiscuousServerVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::Certificate,
            _intermediates: &[rustls::Certificate],
            _server_name: &rustls::ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: std::time::SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
            // here be dragons
            Ok(rustls::client::ServerCertVerified::assertion())
        }
    }
    let client_config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(PromiscuousServerVerifier))
        .with_no_client_auth();
    let (socket, _response) = tokio_tungstenite::client_async_tls_with_config(
        req,
        stream,
        None,
        Some(Connector::Rustls(Arc::new(client_config))),
    )
    .await?;

    log::info!("Strarting graphql client");
    let ws_client = GraphQLWebSocket::new(
        socket,
        Some(serde_json::to_value(SessionToken { token: relay_token })?),
    );

    let signaller = Arc::new(GraphQLSignaller::new(ws_client.clone()));
    let broadcaster = Broadcaster::new(signaller.clone()).await;

    let data_producer_available = ws_client.subscribe::<signal_query::DataProducerAvailable>(
        signal_query::data_producer_available::Variables,
    );
    let mut data_producer_available_stream = data_producer_available.execute();
    tokio::spawn(async move {
        let _vcm_capturer = broadcaster
            .produce_video_from_vcm_capturer(Some(-1), 1280, 720, 30)
            .await;
        let _alsa_capturer = broadcaster.produce_audio_from_default_alsa().await;
        let mut shutdown = signaller.shutdown();
        loop {
            tokio::select! {
                Some(Ok(response)) = data_producer_available_stream.next() => {
                    let data_producer_id = response.data.unwrap().data_producer_available;
                    log::debug!("data producer available: {:?}", &data_producer_id);
                    let mut data_consumer = broadcaster.consume_data(data_producer_id.clone()).await.unwrap();
                    let cont_mutex = controllers.clone();
                    tokio::spawn(async move {
                        while let Some(message) = data_consumer.next().await {
                            log::trace!("{:?}", message);

                            if let Some(cont_mutex) = &cont_mutex {
                               if  message.len() == 13 {
                                let mut conts = cont_mutex.lock().unwrap();
                                let res = conts.set_state(controllers::NetworkControllerState(message.try_into().unwrap()));
                                match &res {
                                    Err(e) => log::warn!("Error writing input: {:?}", e),
                                    Ok(_) => (),
                                };
                               }
                            }
                        }
                        log::debug!("data producer {:?} is gone", data_producer_id);
                    });
                },
                _ = shutdown.recv() => {break},
                else => {break}
            }
        }
    });

    println!("Press Enter to end session...");
    let _ = std::io::stdin().read(&mut [0u8]).unwrap();

    Ok(())
}
