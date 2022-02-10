use serde::{Deserialize, Serialize};

use graphql_client::GraphQLQuery;
use std::net::IpAddr;
use vulcast_rtc::types::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
    response_derives = "Debug"
)]
pub struct DataProducerAvailable;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
    response_derives = "Debug"
)]
pub struct CreatePlainTransport;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
    response_derives = "Debug"
)]
pub struct ProducePlain;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
)]
pub struct ServerRtpCapabilities;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
)]
pub struct CreateWebrtcTransport;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
)]
pub struct ClientRtpCapabilities;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
)]
pub struct Produce;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
)]
pub struct ConnectWebrtcTransport;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
)]
pub struct ConsumeData;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/signal_query.gql",
)]
pub struct ProduceData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransportTuple {
    #[serde(rename_all = "camelCase")]
    LocalOnly {
        local_ip: IpAddr,
        local_port: u16,
        protocol: TransportProtocol,
    },
    #[serde(rename_all = "camelCase")]
    WithRemote {
        local_ip: IpAddr,
        local_port: u16,
        remote_ip: IpAddr,
        remote_port: u16,
        protocol: TransportProtocol,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportProtocol {
    Tcp,
    Udp,
}

impl TransportTuple {
    pub fn local_ip(&self) -> IpAddr {
        match self {
            TransportTuple::LocalOnly { local_ip, .. }
            | TransportTuple::WithRemote { local_ip, .. } => *local_ip,
        }
    }
    pub fn local_port(&self) -> u16 {
        match self {
            TransportTuple::LocalOnly { local_port, .. }
            | TransportTuple::WithRemote { local_port, .. } => *local_port,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlainTransportOptions {
    pub id: TransportId,
    pub tuple: TransportTuple,
}
