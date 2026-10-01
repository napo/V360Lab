//! Live preview: RTSP/RTP H.264 from the camera, forwarded to the UI.
//!
//! ```text
//! camera --RTSP (TCP 554)--> RtspClient: DESCRIBE, SETUP, PLAY, keep-alive
//!        --RTP (UDP)-------> Depacketizer --> access units --> sink (UI channel)
//! ```
//!
//! The UI decodes the H.264 access units with WebCodecs. Messages sent to
//! the sink are binary, first byte = message type:
//! - `0` config: UTF-8 JSON `{"codec": "avc1.640028"}`, sent before the
//!   first frame and whenever the codec changes;
//! - `1` frame: `[1, keyframe (0/1), rtp timestamp (u32 BE)]` + Annex B data;
//! - `2` ended: UTF-8 JSON, `null` when stopped normally, else an error.

pub mod h264;
pub mod rtsp;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Instant};

use crate::error::AppError;
use h264::Depacketizer;
use rtsp::{RtspClient, RtspError};

/// No video at all within this time after PLAY: likely blocked by a firewall.
const FIRST_PACKET_TIMEOUT: Duration = Duration::from_secs(6);
/// Video stopped arriving (camera changed mode, went to sleep, …).
const STALL_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_KEEP_ALIVE: Duration = Duration::from_secs(20);
const TEARDOWN_TIMEOUT: Duration = Duration::from_secs(1);
/// Kernel receive buffer requested for RTP. A 360° keyframe arrives as a
/// burst of hundreds of packets; the default buffer (about 200 KB on Linux)
/// can overflow, and every lost packet freezes the image until the next
/// keyframe. The system may grant less (Linux caps it at `rmem_max`).
const RTP_RECEIVE_BUFFER: usize = 4 * 1024 * 1024;
/// Packets buffered between the socket reader and the depacketizer.
const PACKET_QUEUE: usize = 4096;
/// How often losses are logged.
const STATS_INTERVAL: Duration = Duration::from_secs(10);

pub const MSG_CONFIG: u8 = 0;
pub const MSG_FRAME: u8 = 1;
pub const MSG_ENDED: u8 = 2;

/// Receives preview messages; returns `false` when nobody listens anymore.
pub type Sink = Box<dyn Fn(Vec<u8>) -> bool + Send + Sync>;

/// Asks the camera for a keyframe (VIRB `enableIDR`). Must not block.
pub type KeyframeRequest = Box<dyn Fn() + Send + Sync>;

/// Minimum time between two keyframe requests.
const KEYFRAME_REQUEST_INTERVAL: Duration = Duration::from_secs(1);

/// Why a preview could not start or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewFailure {
    /// The camera refused or does not support the live preview.
    Unsupported,
    /// RTSP negotiation failed.
    Negotiation,
    /// PLAY succeeded but no video arrived (firewall?).
    NoVideo,
    /// Video arrived, then stopped.
    Stalled,
}

impl PreviewFailure {
    pub fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Negotiation => "negotiation",
            Self::NoVideo => "noVideo",
            Self::Stalled => "stalled",
        }
    }
}

impl AppError {
    fn preview(reason: PreviewFailure, detail: impl ToString) -> Self {
        AppError::Preview {
            reason,
            detail: detail.to_string(),
        }
    }
}

impl From<RtspError> for AppError {
    fn from(e: RtspError) -> Self {
        AppError::preview(PreviewFailure::Negotiation, e)
    }
}

/// An RTSP session that is playing, ready to receive video.
struct Session {
    rtsp: RtspClient,
    socket: Arc<UdpSocket>,
    depacketizer: Depacketizer,
    keep_alive: Duration,
}

impl Session {
    async fn open(url: &str) -> Result<Self, AppError> {
        let mut rtsp = RtspClient::connect(url).await?;
        let track = rtsp.describe().await?;
        let socket =
            bind_rtp_socket().map_err(|e| AppError::preview(PreviewFailure::Negotiation, e))?;
        let port = socket
            .local_addr()
            .map_err(|e| AppError::preview(PreviewFailure::Negotiation, e))?
            .port();
        let transport = rtsp.setup(&track, port).await?;

        // Send one datagram towards the camera's RTP port so that stateful
        // firewalls (e.g. ufw on Linux) let its video packets back in.
        if let Some(server_port) = transport.server_rtp_port {
            let _ = socket.send_to(&[0], (rtsp.host(), server_port)).await;
        }
        rtsp.play().await?;

        let keep_alive = transport
            .timeout_secs
            .map(|t| Duration::from_secs((t / 2).max(5)))
            .unwrap_or(DEFAULT_KEEP_ALIVE);
        log::info!(
            "Live preview playing: session {}, UDP port {port}, keep-alive {} s",
            transport.session,
            keep_alive.as_secs()
        );
        Ok(Self {
            rtsp,
            socket: Arc::new(socket),
            depacketizer: Depacketizer::new(track.payload_type),
            keep_alive,
        })
    }

    /// Forwards video to `sink` until stopped, the sink goes away, or the
    /// stream fails.
    async fn run(
        &mut self,
        sink: &Sink,
        request_keyframe: &KeyframeRequest,
        mut stop: oneshot::Receiver<()>,
    ) -> Result<(), AppError> {
        // The socket is drained by its own task, so that sending frames to
        // the UI or waiting for an RTSP keep-alive answer never leaves
        // packets waiting in the kernel buffer.
        let (packet_tx, mut packets) = mpsc::channel::<std::io::Result<Vec<u8>>>(PACKET_QUEUE);
        let _reader = AbortOnDrop(tokio::spawn(read_packets(self.socket.clone(), packet_tx)));
        let mut keep_alive = interval(self.keep_alive);
        keep_alive.tick().await;
        let started = Instant::now();
        let mut last_packet: Option<Instant> = None;
        let mut codec: Option<String> = None;
        let mut watchdog = interval(Duration::from_secs(1));
        let mut stats = interval(STATS_INTERVAL);
        stats.tick().await;
        let mut keyframe_requested: Option<Instant> = None;

        loop {
            tokio::select! {
                _ = &mut stop => return Ok(()),
                received = packets.recv() => {
                    let packet = received
                        .unwrap_or_else(|| Err(std::io::Error::other("RTP reader stopped")))
                        .map_err(|e| AppError::preview(PreviewFailure::Stalled, e))?;
                    last_packet = Some(Instant::now());
                    let units = self.depacketizer.push(&packet);
                    // Without a keyframe the image stays frozen: ask for
                    // one instead of waiting for the camera's next one.
                    if self.depacketizer.waiting_for_keyframe()
                        && keyframe_requested
                            .is_none_or(|at| at.elapsed() >= KEYFRAME_REQUEST_INTERVAL)
                    {
                        keyframe_requested = Some(Instant::now());
                        request_keyframe();
                    }
                    for unit in units {
                        let current = self.depacketizer.codec();
                        if current.is_some() && current != codec {
                            codec = current;
                            let config = serde_json::json!({ "codec": codec }).to_string();
                            if !sink(message(MSG_CONFIG, config.as_bytes())) {
                                return Ok(());
                            }
                        }
                        if codec.is_none() {
                            continue;
                        }
                        let mut frame = Vec::with_capacity(6 + unit.data.len());
                        frame.push(MSG_FRAME);
                        frame.push(unit.keyframe as u8);
                        frame.extend_from_slice(&unit.rtp_timestamp.to_be_bytes());
                        frame.extend_from_slice(&unit.data);
                        if !sink(frame) {
                            return Ok(());
                        }
                    }
                }
                _ = stats.tick() => log_stats(&mut self.depacketizer),
                _ = keep_alive.tick() => {
                    if let Err(e) = self.rtsp.keep_alive().await {
                        log::warn!("Live preview keep-alive failed: {e}");
                    }
                }
                _ = watchdog.tick() => {
                    match last_packet {
                        None if started.elapsed() > FIRST_PACKET_TIMEOUT => {
                            return Err(AppError::preview(
                                PreviewFailure::NoVideo,
                                "no RTP packets received after PLAY",
                            ));
                        }
                        Some(at) if at.elapsed() > STALL_TIMEOUT => {
                            return Err(AppError::preview(
                                PreviewFailure::Stalled,
                                format!("no RTP packets for {} s", STALL_TIMEOUT.as_secs()),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn close(mut self) {
        log_stats(&mut self.depacketizer);
        let _ = tokio::time::timeout(TEARDOWN_TIMEOUT, self.rtsp.teardown()).await;
    }
}

fn message(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + payload.len());
    out.push(kind);
    out.extend_from_slice(payload);
    out
}

/// Reads RTP packets until the receiver goes away or the socket fails.
async fn read_packets(socket: Arc<UdpSocket>, packets: mpsc::Sender<std::io::Result<Vec<u8>>>) {
    let mut buffer = vec![0u8; 65_536];
    loop {
        let received = socket.recv_from(&mut buffer).await;
        let failed = received.is_err();
        let packet = received.map(|(len, _)| buffer[..len].to_vec());
        if packets.send(packet).await.is_err() || failed {
            return;
        }
    }
}

struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn log_stats(depacketizer: &mut Depacketizer) {
    let stats = depacketizer.take_stats();
    if stats.lost_packets > 0 || stats.skipped_frames > 0 {
        log::info!(
            "Live preview: {} frames shown, {} skipped waiting for a keyframe, {} RTP packets lost",
            stats.frames,
            stats.skipped_frames,
            stats.lost_packets
        );
    } else if stats.frames > 0 {
        log::debug!("Live preview: {} frames, no losses", stats.frames);
    }
}

/// Binds an even local port (RTP convention; RTCP is on the next one) with
/// a large receive buffer.
fn bind_rtp_socket() -> std::io::Result<UdpSocket> {
    let mut fallback = None;
    for _ in 0..8 {
        let socket = new_rtp_socket()?;
        if socket.local_addr()?.as_socket().map(|a| a.port() % 2 == 0) == Some(true) {
            return into_tokio(socket);
        }
        fallback.get_or_insert(socket);
    }
    match fallback {
        Some(socket) => into_tokio(socket),
        None => into_tokio(new_rtp_socket()?),
    }
}

fn new_rtp_socket() -> std::io::Result<socket2::Socket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    if let Err(e) = socket.set_recv_buffer_size(RTP_RECEIVE_BUFFER) {
        log::debug!("Could not enlarge the RTP receive buffer: {e}");
    }
    socket.bind(&SocketAddr::from(([0, 0, 0, 0], 0)).into())?;
    Ok(socket)
}

fn into_tokio(socket: socket2::Socket) -> std::io::Result<UdpSocket> {
    match socket.recv_buffer_size() {
        Ok(size) => log::debug!("RTP receive buffer: {size} bytes"),
        Err(e) => log::debug!("RTP receive buffer size unknown: {e}"),
    }
    socket.set_nonblocking(true)?;
    UdpSocket::from_std(socket.into())
}

struct Running {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

/// At most one preview at a time.
#[derive(Default)]
pub struct PreviewManager {
    running: Mutex<Option<Running>>,
}

impl PreviewManager {
    /// Opens the stream (errors are returned here), then forwards video to
    /// `sink` in the background. Replaces any running preview.
    pub async fn start(
        &self,
        url: &str,
        sink: Sink,
        request_keyframe: KeyframeRequest,
    ) -> Result<(), AppError> {
        let mut running = self.running.lock().await;
        if let Some(previous) = running.take() {
            stop_running(previous).await;
            // Give the camera a moment to release the previous session.
            sleep(Duration::from_millis(200)).await;
        }
        let mut session = Session::open(url).await?;
        let (stop_tx, stop_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let result = session.run(&sink, &request_keyframe, stop_rx).await;
            session.close().await;
            let ended = match &result {
                Ok(()) => "null".to_string(),
                Err(e) => {
                    log::warn!("Live preview ended: {e}");
                    serde_json::to_string(e).unwrap_or_else(|_| "null".into())
                }
            };
            sink(message(MSG_ENDED, ended.as_bytes()));
        });
        *running = Some(Running {
            stop: stop_tx,
            task,
        });
        Ok(())
    }

    pub async fn stop(&self) {
        if let Some(previous) = self.running.lock().await.take() {
            stop_running(previous).await;
        }
    }
}

async fn stop_running(running: Running) {
    let _ = running.stop.send(());
    let _ = running.task.await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    /// A fake camera: answers RTSP like a VIRB and, after PLAY, sends an
    /// SPS/PPS aggregate and a keyframe to the client's RTP port.
    async fn fake_camera() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = BufReader::new(socket);
            let rtp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let server_port = rtp.local_addr().unwrap().port();
            let mut client_port = 0u16;
            loop {
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    if socket.read_line(&mut line).await.unwrap() == 0 {
                        return;
                    }
                    if line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                let method = head.split(' ').next().unwrap().to_string();
                let cseq = head
                    .lines()
                    .find_map(|l| l.strip_prefix("CSeq: "))
                    .unwrap()
                    .to_string();
                let reply = match method.as_str() {
                    "DESCRIBE" => {
                        let sdp = format!(
                            "v=0\r\nm=video 0 RTP/AVP 96\r\na=control:rtsp://127.0.0.1:{port}/live/video\r\na=rtpmap:96 H264/90000\r\n"
                        );
                        format!("RTSP/1.0 200 OK\r\nCSeq: {cseq}\r\nContent-Length: {}\r\n\r\n{sdp}", sdp.len())
                    }
                    "SETUP" => {
                        client_port = head
                            .lines()
                            .find_map(|l| l.split("client_port=").nth(1))
                            .and_then(|p| p.split('-').next())
                            .and_then(|p| p.trim().parse().ok())
                            .unwrap();
                        format!(
                            "RTSP/1.0 200 OK\r\nCSeq: {cseq}\r\nSession: 7\r\nTransport: RTP/AVP/UDP;unicast;client_port={client_port}-{};server_port={server_port}-{}\r\n\r\n",
                            client_port + 1,
                            server_port + 1
                        )
                    }
                    _ => format!("RTSP/1.0 200 OK\r\nCSeq: {cseq}\r\n\r\n"),
                };
                socket.get_mut().write_all(reply.as_bytes()).await.unwrap();
                if method == "PLAY" {
                    let target = ("127.0.0.1", client_port);
                    let mut stap = vec![24 | 0x60];
                    for nal in [&h264::tests::SPS[..], &h264::tests::PPS[..]] {
                        stap.extend_from_slice(&(nal.len() as u16).to_be_bytes());
                        stap.extend_from_slice(nal);
                    }
                    rtp.send_to(&h264::tests::rtp(1, 0, false, &stap), target).await.unwrap();
                    for packet in h264::tests::idr_fragments(2, 0) {
                        rtp.send_to(&packet, target).await.unwrap();
                    }
                }
            }
        });
        format!("rtsp://127.0.0.1:{port}/live?maxResolutionVertical=0")
    }

    #[tokio::test]
    async fn forwards_config_and_keyframe_then_stops() {
        let url = fake_camera().await;
        let messages = Arc::new(StdMutex::new(Vec::<Vec<u8>>::new()));
        let collected = messages.clone();
        let sink: Sink = Box::new(move |m| {
            collected.lock().unwrap().push(m);
            true
        });

        let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = requests.clone();
        let request_keyframe: KeyframeRequest = Box::new(move || {
            counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        let manager = PreviewManager::default();
        manager.start(&url, sink, request_keyframe).await.unwrap();
        for _ in 0..50 {
            if messages.lock().unwrap().len() >= 2 {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
        manager.stop().await;

        let messages = messages.lock().unwrap();
        assert_eq!(messages[0][0], MSG_CONFIG);
        assert_eq!(&messages[0][1..], br#"{"codec":"avc1.640028"}"#);
        assert_eq!(messages[1][0], MSG_FRAME);
        assert_eq!(messages[1][1], 1, "keyframe flag");
        assert_eq!(&messages[1][6..10], &[0, 0, 0, 1]);
        let last = messages.last().unwrap();
        assert_eq!(last, &message(MSG_ENDED, b"null"));
        // Waiting for the first keyframe: one request, not one per packet.
        assert_eq!(requests.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
