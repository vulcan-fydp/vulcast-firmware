use serde::{Deserialize, Serialize};

use graphql_client::GraphQLQuery;
use mediasoup::{
    data_producer::DataProducerId, data_structures::TransportTuple, producer::ProducerId,
    rtp_parameters::MediaKind, rtp_parameters::RtpParameters, transport::TransportId,
};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlainTransportOptions {
    pub id: TransportId,
    pub tuple: TransportTuple,
}
