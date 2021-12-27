use async_trait::async_trait;
use graphql_ws::GraphQLWebSocket;
use tokio::sync::broadcast;
use vulcast_rtc::broadcaster::{Signaller, TransportConnectionState};

use crate::graphql::signal_query as schema;

pub struct GraphQLSignaller {
    client: GraphQLWebSocket,
    shutdown_tx: broadcast::Sender<()>,
}
impl GraphQLSignaller {
    pub fn new(client: GraphQLWebSocket) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);
        Self {
            client,
            shutdown_tx,
        }
    }
    pub fn shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
}
#[async_trait]
impl Signaller for GraphQLSignaller {
    async fn server_rtp_capabilities(&self) -> vulcast_rtc::types::RtpCapabilitiesFinalized {
        self.client
            .query_unchecked::<schema::ServerRtpCapabilities>(
                schema::server_rtp_capabilities::Variables,
            )
            .await
            .server_rtp_capabilities
    }

    async fn create_webrtc_transport(&self) -> vulcast_rtc::types::WebRtcTransportOptions {
        self.client
            .query_unchecked::<schema::CreateWebrtcTransport>(
                schema::create_webrtc_transport::Variables,
            )
            .await
            .create_webrtc_transport
    }

    async fn on_rtp_capabilities(&self, rtp_capabilities: vulcast_rtc::types::RtpCapabilities) {
        self.client
            .query_unchecked::<schema::ClientRtpCapabilities>(
                schema::client_rtp_capabilities::Variables { rtp_capabilities },
            )
            .await;
    }

    async fn on_produce(
        &self,
        transport_id: vulcast_rtc::types::TransportId,
        kind: vulcast_rtc::types::MediaKind,
        rtp_parameters: vulcast_rtc::types::RtpParameters,
    ) -> vulcast_rtc::types::ProducerId {
        self.client
            .query_unchecked::<schema::Produce>(schema::produce::Variables {
                transport_id,
                kind,
                rtp_parameters,
            })
            .await
            .produce
    }

    async fn on_connect_webrtc_transport(
        &self,
        transport_id: vulcast_rtc::types::TransportId,
        dtls_parameters: vulcast_rtc::types::DtlsParameters,
    ) {
        self.client
            .query_unchecked::<schema::ConnectWebrtcTransport>(
                schema::connect_webrtc_transport::Variables {
                    transport_id,
                    dtls_parameters,
                },
            )
            .await;
    }

    async fn consume_data(
        &self,
        transport_id: vulcast_rtc::types::TransportId,
        data_producer_id: vulcast_rtc::types::DataProducerId,
    ) -> Result<vulcast_rtc::types::DataConsumerOptions, Box<dyn std::error::Error>> {
        Ok(self
            .client
            .query_unchecked::<schema::ConsumeData>(schema::consume_data::Variables {
                transport_id,
                data_producer_id,
            })
            .await
            .consume_data)
    }

    async fn on_connection_state_changed(
        &self,
        _transport_id: vulcast_rtc::types::TransportId,
        state: vulcast_rtc::broadcaster::TransportConnectionState,
    ) {
        match state {
            TransportConnectionState::Disconnected | TransportConnectionState::Failed => {
                let _ = self.shutdown_tx.send(());
            }
            _ => (),
        }
    }
}
