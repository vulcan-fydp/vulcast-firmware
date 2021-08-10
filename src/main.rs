use std::io::Read;
use std::sync::Arc;

mod graphql;
use graphql::backend_query;
use graphql::signal_query;

use anyhow::{anyhow, Result};
use backend_query::assign_vulcast_to_relay::AssignVulcastToRelayAssignVulcastToRelay::{
    AuthenticationError, RelayAssignment, VulcastAssignedToRelayError,
};
use backend_query::log_in_as_vulcast::LogInAsVulcastLogInAsVulcast::{
    AuthenticationError as LoginAuthenticationError, VulcastAuthentication,
};
use futures::StreamExt;
use graphql_client::{GraphQLQuery, Response};
use graphql_ws::GraphQLWebSocket;
use http::Uri;
use ini::Ini;
use serde::Serialize;
use serde_json::json;
use std::env;
use std::process::{Child, Command, Stdio};
use tokio::net::TcpStream;
use tokio_tungstenite::Connector;
use vulcast_rtc::types::*;

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
            secret: secret,
        });
    let auth = client.post(&uri).json(&login_query).send().await?;
    let response_body: Response<backend_query::log_in_as_vulcast::ResponseData> =
        auth.json().await?;
    if let Some(errors) = response_body.errors {
        errors.iter().for_each(|error| log::error!("{:?}", error))
    }
    let response_data: backend_query::log_in_as_vulcast::ResponseData = response_body
        .data
        .ok_or(anyhow!("Request returned no data"))?;
    match response_data.log_in_as_vulcast {
        VulcastAuthentication(auth) => Ok(auth.vulcast_access_token),
        LoginAuthenticationError(error) => Err(anyhow!("Authentication error: {}", error.message)),
    }
}

fn write_relay_assignment(hostname: &str, token: &str) -> Result<()> {
    let mut assigned = Ini::new();
    assigned
        .with_section(Some("relay"))
        .set("hostname", hostname)
        .set("token", token);
    assigned.write_to_file(env::var("HOME").unwrap() + "/.vulcast/assigned_relay")?;
    Ok(())
}

fn read_relay_assignment() -> Result<(String, String)> {
    let relay_file = Ini::load_from_file(env::var("HOME").unwrap() + "/.vulcast/assigned_relay")?;
    let host = relay_file
        .get_from(Some("relay"), "hostname")
        .ok_or(anyhow!("Could not load relay hostname from file"))?;
    let token = relay_file
        .get_from(Some("relay"), "token")
        .ok_or(anyhow!("Could not load relay token from file"))?;
    Ok((host.to_owned(), token.to_owned()))
}

async fn assign_relay(
    conf: &Ini,
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
        .bearer_auth("vulcast_".to_owned() + &auth_token)
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
        .ok_or(anyhow!("Request returned no data"))?;
    match response_data.assign_vulcast_to_relay {
        RelayAssignment(assignment) => {
            let _ =
                write_relay_assignment(&assignment.relay.host_name, &assignment.relay_access_token);
            Ok((assignment.relay.host_name, assignment.relay_access_token))
        }
        AuthenticationError(error) => Err(anyhow!("Authentication error: {}", error.message)),
        VulcastAssignedToRelayError(error) => {
            Err(anyhow!("Vulcast already assigned error: {}", error.message))
        }
    }
}

fn _stream_ffmpeg(
    audio_transport_options: signal_query::PlainTransportOptions,
    video_transport_options: signal_query::PlainTransportOptions,
) -> Result<Child> {
    let tee_fmt = format!(
        "[select=a:f=rtp:ssrc=11111111:payload_type=101]rtp://{}:{}|\
         [select=v:f=rtp:ssrc=22222222:payload_type=102]rtp://{}:{}",
        audio_transport_options.tuple.local_ip(),
        audio_transport_options.tuple.local_port(),
        video_transport_options.tuple.local_ip(),
        video_transport_options.tuple.local_port()
    );

    #[rustfmt::skip]
    let ffmpeg = Command::new("ffmpeg")
        // .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&[
            "-fflags", "+genpts",
            "-f", "v4l2", "-thread_queue_size", "1024", "-input_format", "mjpeg",
            "-video_size", "640x480", "-framerate", "30", "-i", "/dev/video0",
            "-f", "alsa", "-thread_queue_size", "1024", "-ac", "2", "-i", "hw:CARD=MS2109,DEV=0",
            // "-re", "-stream_loop", "-1", "-i", "esker.mp4",
            // "-c:v", "copy",
            "-c:v", "libx264", "-preset", "ultrafast", "-maxrate", "300k", "-bufsize", "300k", "-g", "60", "-tune", "zerolatency",
            // "-c:v", "h264_v4l2m2m", "-g", "48",
            // "-c:v", "h264_omx", "-profile:v", "baseline", "-g", "48",
            "-bsf:v", "h264_mp4toannexb,dump_extra",
            "-pix_fmt", "yuv420p",
            "-map", "0:v:0",
            "-map", "1:a:0",
            // "-map", "0:a:0",
            "-c:a", "libopus", "-ab", "256k", "-ac", "2", "-ar", "48000",
            "-f", "tee", &tee_fmt,
        ])
        .spawn()?;
    Ok(ffmpeg)
}

fn stream_gstreamer(
    audio_transport_options: signal_query::PlainTransportOptions,
    video_transport_options: signal_query::PlainTransportOptions,
) -> Result<Child> {
    let video_ip = format!("host={}", video_transport_options.tuple.local_ip());
    let video_port = format!("port={}", video_transport_options.tuple.local_port());
    let audio_ip = format!("host={}", audio_transport_options.tuple.local_ip());
    let audio_port = format!("port={}", audio_transport_options.tuple.local_port());
    #[rustfmt::skip]
    let gstreamer = Command::new("gst-launch-1.0")
        .args(&[
            "rtpbin", "name=rtpbin",
            "v4l2src", "device=/dev/video0",
            "!", "image/jpeg,framerate=30/1,width=640,height=480",
            "!", "queue",
            "!", "decodebin",
            "!", "videoconvert",
            "!", "vp8enc", "end-usage=cbr", "keyframe-max-dist=60", "target-bitrate=30000", "deadline=1", "cpu-used=4",
            "!", "rtpvp8pay", "pt=102", "ssrc=22222222", "picture-id-mode=2",
            // "!", "omxh264enc", "control-rate=constant", "target-bitrate=30000",
            //      "b-frames=0", "interval-intraframes=60", "inline-header=true",
            // "!", "video/x-h264,profile=baseline",
            // "!", "h264parse",
            // "!", "rtph264pay", "pt=102", "ssrc=22222222",
            "!", "rtpbin.send_rtp_sink_0",
            "rtpbin.send_rtp_src_0", "!", "udpsink", &video_ip, &video_port, "bind-port=50000",
            "rtpbin.send_rtcp_src_0", "!", "udpsink", &video_ip, &video_port, "bind-port=50000", "sync=false", "async=false",
            "alsasrc", "device=\"hw:CARD=MS2109,DEV=0\"",
            "!", "queue",
            "!", "decodebin",
            "!", "audioresample",
            "!", "audioconvert",
            "!", "opusenc", "inband-fec=true",
            "!", "rtpopuspay", "pt=101", "ssrc=11111111",
            "!", "rtpbin.send_rtp_sink_1",
            "rtpbin.send_rtp_src_1", "!", "udpsink", &audio_ip, &audio_port, "bind-port=50001",
            "rtpbin.send_rtcp_src_1", "!", "udpsink", &audio_ip, &audio_port, "bind-port=50001", "sync=false", "async=false"
        ])
        .spawn()?;
    Ok(gstreamer)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default());

    log::info!("Loading config from ~/.vulcast/vulcast.conf");
    let conf = Ini::load_from_file(env::var("HOME").unwrap() + "/.vulcast/vulcast.conf")?;
    let client = reqwest::Client::new();

    let access_token = login(&conf, &client).await?;
    let (relay_host, relay_token) = assign_relay(&conf, &client, &access_token)
        .await
        .or(read_relay_assignment())?;

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

    // remove this later
    struct PromiscuousServerVerifier;
    impl rustls::ServerCertVerifier for PromiscuousServerVerifier {
        fn verify_server_cert(
            &self,
            _roots: &rustls::RootCertStore,
            _presented_certs: &[rustls::Certificate],
            _dns_name: webpki::DNSNameRef,
            _ocsp_response: &[u8],
        ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
            // here be dragons
            Ok(rustls::ServerCertVerified::assertion())
        }
    }
    let mut client_config = rustls::ClientConfig::default();
    client_config
        .dangerous()
        .set_certificate_verifier(Arc::new(PromiscuousServerVerifier));
    let (socket, _response) = tokio_tungstenite::client_async_tls_with_config(
        req,
        stream,
        None,
        Some(Connector::Rustls(Arc::new(client_config))),
    )
    .await?;

    let ws_client = GraphQLWebSocket::new();
    ws_client.connect(
        socket,
        Some(serde_json::to_value(SessionToken { token: relay_token })?),
    );

    let audio_transport_options = ws_client
        .query_unchecked::<signal_query::CreatePlainTransport>(
            signal_query::create_plain_transport::Variables,
        )
        .await
        .create_plain_transport;
    log::debug!("Audio transport options: {:?}", audio_transport_options);
    let video_transport_options = ws_client
        .query_unchecked::<signal_query::CreatePlainTransport>(
            signal_query::create_plain_transport::Variables,
        )
        .await
        .create_plain_transport;
    log::debug!("Video transport options: {:?}", video_transport_options);

    let audio_transport_id = audio_transport_options.id.clone();
    let video_transport_id = video_transport_options.id.clone();

    let audio_producer_id = ws_client
        .query_unchecked::<signal_query::ProducePlain>(signal_query::produce_plain::Variables {
            transport_id: audio_transport_id,
            kind: MediaKind::Audio,
            rtp_parameters: RtpParameters::from(json!({
                "codecs": [{
                    "mimeType": "audio/opus",
                    "payloadType": 101,
                    "clockRate": 48000,
                    "channels": 2,
                    "parameters": {"sprop-stereo": 1},
                    "rtcpFeedback": [{"type": "nack", "parameter": ""},
                                     {"type": "nack", "parameter": "pli"},
                                     {"type": "ccm", "parameter": "fir"},
                                     {"type": "goog-remb", "parameter": ""},
                                     {"type": "transport-cc", "parameter": ""},
                                     {"type": "unknown", "parameter": ""},
                    ]
                }],
                "headerExtensions": [],
                "encodings": [{
                    "ssrc": 11111111,
                }],
                "rtcp": {"reducedSize": true}
            })),
        })
        .await
        .produce_plain;
    log::debug!("audio producer: {:?}", audio_producer_id);

    let video_producer_id = ws_client
        .query_unchecked::<signal_query::ProducePlain>(signal_query::produce_plain::Variables {
            transport_id: video_transport_id,
            kind: MediaKind::Video,
            rtp_parameters: RtpParameters::from(json!({
                "codecs": [{
                    // "mimeType": "video/H264",
                    // "payloadType": 102,
                    // "clockRate": 90000,
                    // "parameters": {
                    //     "packetization-mode": 1,
                    //     "level-asymmetry-allowed": 1,
                    //     "profile-level-id": "42e01f"
                    // },
                    "mimeType": "video/VP8",
                    "payloadType": 102,
                    "clockRate": 90000,
                    "parameters": {},
                    "rtcpFeedback": [{"type": "nack", "parameter": ""},
                                    {"type": "nack", "parameter": "pli"},
                                    {"type": "ccm", "parameter": "fir"},
                                    {"type": "goog-remb", "parameter": ""},
                                    {"type": "transport-cc", "parameter": ""},
                                    {"type": "unknown", "parameter": ""},
                    ]
                }],
                "headerExtensions": [],
                "encodings": [{
                    "ssrc": 22222222,
                }],
                "rtcp": {"reducedSize": true}
            })),
        })
        .await
        .produce_plain;
    log::debug!("video producer: {:?}", video_producer_id);

    // println!("Press Enter to start stream...");
    // let _ = std::io::stdin().read(&mut [0u8]).unwrap();

    let mut stream_process = stream_gstreamer(audio_transport_options, video_transport_options)?;

    let data_producer_available = ws_client.subscribe::<signal_query::DataProducerAvailable>(
        signal_query::data_producer_available::Variables,
    );
    let mut data_producer_available_stream = data_producer_available.execute();
    tokio::spawn(async move {
        while let Some(Ok(response)) = data_producer_available_stream.next().await {
            log::debug!(
                "data producer available: {:?}",
                response.data.unwrap().data_producer_available
            )
        }
    });

    println!("Press Enter to end session...");
    let _ = std::io::stdin().read(&mut [0u8]).unwrap();

    stream_process.kill()?;

    Ok(())
}
