use crate::graphql::signal_query;

use anyhow::Result;
use std::process::{Child, Command, Stdio};
// use std::thread;

pub trait Streamer {
    fn stream(
        &self,
        audio_transport_options: signal_query::PlainTransportOptions,
        video_transport_options: signal_query::PlainTransportOptions,
    ) -> Result<Child>;
}

pub fn _stream_persistent(
    _streamer: &dyn Streamer,
    _audio_transport_options: signal_query::PlainTransportOptions,
    _video_transport_options: signal_query::PlainTransportOptions,
) {
}

pub struct FfmpegStreamer {}

impl FfmpegStreamer {
    pub fn _new() -> Self {
        Self {}
    }
}

impl Streamer for FfmpegStreamer {
    fn stream(
        &self,
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
}

pub struct GStreamer {}

impl GStreamer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Streamer for GStreamer {
    fn stream(
        &self,
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
}
