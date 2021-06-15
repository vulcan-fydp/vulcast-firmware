use backend_schema::{schema, signal_schema};
use clap::{AppSettings, Clap};
use graphql_client::{GraphQLQuery, Response};
use graphql_ws::GraphQLWebSocket;
use http::Uri;
use native_tls::TlsConnector;
use reqwest;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::Connector;

#[derive(Serialize)]
struct SessionToken {
    token: String,
}

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    /// Listening address for signal endpoint (domain required).
    #[clap(long, default_value = "http://192.168.0.180:4000")]
    pub backend_addr: String,
    /// Listening address for signal endpoint (domain required).
    #[clap(long, default_value = "wss://192.168.0.180:8443")]
    pub signal_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(env_logger::Env::default());
    let opts: Opts = Opts::parse();

    let register_query =
        schema::AssignVulcastToRelay::build_query(schema::assign_vulcast_to_relay::Variables {});
    let client = reqwest::Client::new();
    let uri = opts.backend_addr + "/graphql";
    let res = client.post(&uri).json(&register_query).send().await?;
    let response_body: Response<schema::assign_vulcast_to_relay::ResponseData> = res.json().await?;
    let response_data: schema::assign_vulcast_to_relay::ResponseData = response_body.data.unwrap();
    let token = match response_data.assign_vulcast_to_relay {
        schema::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::RelayAssignment(assignment) => {
            Some(assignment.relay_access_token)
        }
        schema::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::AuthenticationError(error) => {
            log::error!("{}", error.message);
            None
        }
        schema::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::VulcastAssignedToRelayError(error) => {
            log::error!("{}", error.message);
            None
        }
    };
    log::info!("{:?}", token);

    let connector = TlsConnector::builder()
        .danger_accept_invalid_hostnames(true)
        .danger_accept_invalid_certs(true)
        .build()?;
    let uri: Uri = opts.signal_addr.parse()?;
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
    ws_client.connect(socket, Some(serde_json::to_value(token)?));

    Ok(())
}
