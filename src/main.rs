use anyhow::{anyhow, Result};
use backend_schema::{schema, signal_schema};
use graphql_client::{GraphQLQuery, Response};
use graphql_ws::GraphQLWebSocket;
use http::Uri;
use ini::Ini;
use native_tls::TlsConnector;
use reqwest;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::Connector;

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

    let login_query = schema::LogInAsVulcast::build_query(schema::log_in_as_vulcast::Variables {
        vulcast_guid: guid,
        secret: secret,
    });
    let auth = client.post(&uri).json(&login_query).send().await?;
    let response_body: Response<schema::log_in_as_vulcast::ResponseData> = auth.json().await?;
    if let Some(errors) = response_body.errors {
        errors.iter().for_each(|error| log::error!("{:?}", error))
    }
    let response_data: schema::log_in_as_vulcast::ResponseData = response_body
        .data
        .ok_or(anyhow!("Request returned no data"))?;
    match response_data.log_in_as_vulcast {
        schema::log_in_as_vulcast::LogInAsVulcastLogInAsVulcast::VulcastAuthentication(auth) => {
            Ok(auth.vulcast_access_token)
        }
        schema::log_in_as_vulcast::LogInAsVulcastLogInAsVulcast::AuthenticationError(error) => {
            Err(anyhow!("Authentication error: {}", error.message))
        }
    }
}

async fn assign_relay(conf: &Ini, client: &reqwest::Client, auth_token: &str) -> Result<String> {
    log::info!("Requesting relay assignment");

    let uri = conf
        .get_from(Some("network"), "backend_addr")
        .expect("No backend address specified")
        .to_owned()
        + "/graphql";

    let register_query =
        schema::AssignVulcastToRelay::build_query(schema::assign_vulcast_to_relay::Variables {});
    let res = client
        .post(&uri)
        .bearer_auth("vulcast_".to_owned() + &auth_token)
        .json(&register_query)
        .send()
        .await?;

    let response_body: Response<schema::assign_vulcast_to_relay::ResponseData> = res.json().await?;
    if let Some(errors) = response_body.errors {
        errors.iter().for_each(|error| log::error!("{:?}", error))
    }
    let response_data: schema::assign_vulcast_to_relay::ResponseData = response_body
        .data
        .ok_or(anyhow!("Request returned no data"))?;
    match response_data.assign_vulcast_to_relay {
        schema::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::RelayAssignment(assignment) => {
            Ok(assignment.relay_access_token)
        }
        schema::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::AuthenticationError(error) => {
            Err(anyhow!("Authentication error: {}", error.message))
        }
        schema::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::VulcastAssignedToRelayError(error) => {
            Err(anyhow!("Vulcast already assigned error: {}", error.message))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default());

    log::info!("Loading config from ~/.vulcast/vulcast.conf");
    let conf = Ini::load_from_file("/home/pi/.vulcast/vulcast.conf")?;
    let client = reqwest::Client::new();

    let access_token = login(&conf, &client).await?;
    let relay_token = assign_relay(&conf, &client, &access_token).await?;

    log::info!("{:?}", relay_token);

    let connector = TlsConnector::builder()
        .danger_accept_invalid_hostnames(true)
        .danger_accept_invalid_certs(true)
        .build()?;
    let uri: Uri = conf
        .get_from(Some("network"), "signal_addr")
        .expect("No signal address specified")
        .parse()?;
    log::info!("connecting to {}", &uri);

    let host = uri.host().unwrap();
    let port = uri.port_u16().unwrap();
    let stream = TcpStream::connect((host, port)).await?;

    let req = http::Request::builder()
        .uri(uri)
        .header("Sec-WebSocket-Protocol", "graphql-ws")
        .body(())?;
    let (socket, _response) = tokio_tungstenite::client_async_tls_with_config(
        req,
        stream,
        None,
        Some(Connector::NativeTls(connector)),
    )
    .await?;

    let mut ws_client = GraphQLWebSocket::new();
    ws_client.connect(socket, Some(serde_json::to_value(relay_token)?));

    Ok(())
}
