//! Minimal RTSP client for the VIRB live preview (RFC 2326 subset).
//!
//! Only what the camera needs: DESCRIBE, SETUP (RTP over UDP), PLAY,
//! keep-alive and TEARDOWN. VIRB firmware 4.20 quirks:
//! - the SDP advertises the video control URL with the query string of the
//!   stream URL, but SETUP on it fails with `451 Invalid Parameter`; the
//!   same URL without the query works;
//! - interleaved RTP over TCP is rejected (`461 Unsupported transport`).

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use url::Url;

const DEFAULT_PORT: u16 = 554;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BODY_BYTES: usize = 64 * 1024;
const USER_AGENT: &str = concat!("V360Lab/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, thiserror::Error)]
pub enum RtspError {
    #[error("invalid RTSP URL {0}")]
    InvalidUrl(String),
    #[error("RTSP connection failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("RTSP {method} timed out")]
    Timeout { method: &'static str },
    #[error("malformed RTSP response: {0}")]
    Malformed(String),
    #[error("RTSP {method} failed: {status} {reason}")]
    Status {
        method: &'static str,
        status: u16,
        reason: String,
    },
    #[error("no H.264 video track in the stream description")]
    NoVideoTrack,
}

#[derive(Debug, Clone)]
pub struct RtspResponse {
    pub status: u16,
    pub reason: String,
    headers: Vec<(String, String)>,
    pub body: String,
}

impl RtspResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Video track found in the SDP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTrack {
    pub control_url: String,
    pub payload_type: Option<u8>,
}

/// Result of a successful SETUP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transport {
    pub session: String,
    /// Session timeout announced by the server, if any.
    pub timeout_secs: Option<u64>,
    pub server_rtp_port: Option<u16>,
}

pub struct RtspClient {
    stream: BufReader<TcpStream>,
    url: Url,
    cseq: u32,
    session: Option<String>,
}

impl RtspClient {
    pub async fn connect(url: &str) -> Result<Self, RtspError> {
        let parsed = Url::parse(url).map_err(|_| RtspError::InvalidUrl(url.to_string()))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| RtspError::InvalidUrl(url.to_string()))?
            .to_string();
        let port = parsed.port().unwrap_or(DEFAULT_PORT);
        let stream = timeout(REQUEST_TIMEOUT, TcpStream::connect((host.as_str(), port)))
            .await
            .map_err(|_| RtspError::Timeout { method: "CONNECT" })??;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream: BufReader::new(stream),
            url: parsed,
            cseq: 0,
            session: None,
        })
    }

    pub fn host(&self) -> &str {
        self.url.host_str().unwrap_or_default()
    }

    pub async fn describe(&mut self) -> Result<VideoTrack, RtspError> {
        let url = self.url.to_string();
        let response = self
            .request("DESCRIBE", &url, &[("Accept", "application/sdp")])
            .await?;
        parse_sdp_video(&response.body, &self.url).ok_or(RtspError::NoVideoTrack)
    }

    /// SETUP with RTP over UDP to `client_rtp_port` (RTCP on the next port).
    pub async fn setup(
        &mut self,
        track: &VideoTrack,
        client_rtp_port: u16,
    ) -> Result<Transport, RtspError> {
        let transport = format!(
            "RTP/AVP;unicast;client_port={}-{}",
            client_rtp_port,
            client_rtp_port.wrapping_add(1)
        );
        let mut last_error = None;
        for url in setup_candidates(&track.control_url) {
            match self.request("SETUP", &url, &[("Transport", &transport)]).await {
                Ok(response) => {
                    let session_header = response
                        .header("Session")
                        .ok_or_else(|| RtspError::Malformed("SETUP without Session".into()))?;
                    let (session, timeout_secs) = parse_session(session_header);
                    self.session = Some(session.clone());
                    return Ok(Transport {
                        session,
                        timeout_secs,
                        server_rtp_port: response.header("Transport").and_then(server_rtp_port),
                    });
                }
                Err(RtspError::Status { status: 451 | 461, .. }) if last_error.is_none() => {
                    log::debug!("RTSP SETUP rejected for {url}, retrying without query");
                    last_error = Some(());
                }
                Err(e) => return Err(e),
            }
        }
        Err(RtspError::Status {
            method: "SETUP",
            status: 451,
            reason: "Invalid Parameter".into(),
        })
    }

    pub async fn play(&mut self) -> Result<(), RtspError> {
        let url = self.url.to_string();
        self.request("PLAY", &url, &[("Range", "npt=0.000-")]).await?;
        Ok(())
    }

    /// Keeps the session alive (the VIRB answers OPTIONS).
    pub async fn keep_alive(&mut self) -> Result<(), RtspError> {
        let url = self.url.to_string();
        self.request("OPTIONS", &url, &[]).await?;
        Ok(())
    }

    pub async fn teardown(&mut self) -> Result<(), RtspError> {
        if self.session.is_none() {
            return Ok(());
        }
        let url = self.url.to_string();
        self.request("TEARDOWN", &url, &[]).await?;
        self.session = None;
        Ok(())
    }

    async fn request(
        &mut self,
        method: &'static str,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<RtspResponse, RtspError> {
        self.cseq += 1;
        let mut request = format!(
            "{method} {url} RTSP/1.0\r\nCSeq: {}\r\nUser-Agent: {USER_AGENT}\r\n",
            self.cseq
        );
        if let Some(session) = &self.session {
            request.push_str(&format!("Session: {session}\r\n"));
        }
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        log::debug!("RTSP -> {method} {url}");

        let response = timeout(REQUEST_TIMEOUT, async {
            self.stream.get_mut().write_all(request.as_bytes()).await?;
            read_response(&mut self.stream).await
        })
        .await
        .map_err(|_| RtspError::Timeout { method })??;

        log::debug!("RTSP <- {method}: {} {}", response.status, response.reason);
        if !(200..300).contains(&response.status) {
            return Err(RtspError::Status {
                method,
                status: response.status,
                reason: response.reason,
            });
        }
        Ok(response)
    }
}

async fn read_response<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
) -> Result<RtspResponse, RtspError> {
    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        return Err(RtspError::Malformed("connection closed".into()));
    }
    let mut parts = line.trim_end().splitn(3, ' ');
    let (Some(version), Some(code)) = (parts.next(), parts.next()) else {
        return Err(RtspError::Malformed(line.trim_end().to_string()));
    };
    if !version.starts_with("RTSP/") {
        return Err(RtspError::Malformed(line.trim_end().to_string()));
    }
    let status = code
        .parse()
        .map_err(|_| RtspError::Malformed(line.trim_end().to_string()))?;
    let reason = parts.next().unwrap_or_default().to_string();

    let mut headers = Vec::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            return Err(RtspError::Malformed("truncated headers".into()));
        }
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    let length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Content-Length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err(RtspError::Malformed(format!("body of {length} bytes")));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await?;

    Ok(RtspResponse {
        status,
        reason,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

/// The control URL as given, then without its query string.
fn setup_candidates(control_url: &str) -> Vec<String> {
    let mut urls = vec![control_url.to_string()];
    if let Some((base, _)) = control_url.split_once('?') {
        urls.push(base.to_string());
    }
    urls
}

/// Finds the first H.264 video track of an SDP description.
pub fn parse_sdp_video(sdp: &str, base: &Url) -> Option<VideoTrack> {
    let mut in_video = false;
    let mut control = None;
    let mut payload_type = None;
    let mut is_h264 = false;
    for line in sdp.lines().map(str::trim) {
        if let Some(media) = line.strip_prefix("m=") {
            if in_video {
                break;
            }
            in_video = media.starts_with("video");
            if in_video {
                payload_type = media.split_whitespace().nth(3).and_then(|p| p.parse().ok());
            }
            continue;
        }
        if !in_video {
            continue;
        }
        if let Some(value) = line.strip_prefix("a=control:") {
            control = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("a=rtpmap:") {
            is_h264 |= value.to_ascii_uppercase().contains("H264");
        }
    }
    if !is_h264 {
        return None;
    }
    let control = control?;
    let control_url = if control.contains("://") {
        control
    } else {
        let base = base.as_str().trim_end_matches('/');
        format!("{base}/{control}")
    };
    Some(VideoTrack {
        control_url,
        payload_type,
    })
}

/// `Session: 1085377743;timeout=60` → id and timeout.
fn parse_session(header: &str) -> (String, Option<u64>) {
    let mut parts = header.split(';');
    let id = parts.next().unwrap_or_default().trim().to_string();
    let timeout = parts
        .filter_map(|p| p.trim().strip_prefix("timeout="))
        .find_map(|t| t.parse().ok());
    (id, timeout)
}

fn server_rtp_port(transport: &str) -> Option<u16> {
    transport
        .split(';')
        .find_map(|p| p.trim().strip_prefix("server_port="))
        .and_then(|ports| ports.split('-').next())
        .and_then(|p| p.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SDP returned by a VIRB 360 on firmware 4.20.
    const VIRB_SDP: &str = "v=0\r\n\
o=- 7276421 7276421 IN IP4 192.168.1.200\r\n\
s=Unnamed\r\n\
c=IN IP4 0.0.0.0\r\n\
a=control:rtsp://192.168.1.200/livePreviewStream\r\n\
m=video 0 RTP/AVP 96\r\n\
a=control:rtsp://192.168.1.200/livePreviewStream/video?maxResolutionVertical=0\r\n\
a=rtpmap:96 H264/90000\r\n\
a=fmtp:96 packetization-mode=1\r\n\
m=audio 0 RTP/AVP 96\r\n\
a=control:rtsp://192.168.1.200/livePreviewStream/audio48wyzx\r\n\
a=rtpmap:96 mpeg4-generic/48000/4\r\n";

    #[test]
    fn finds_the_virb_video_track() {
        let base = Url::parse("rtsp://192.168.1.200/livePreviewStream?maxResolutionVertical=0").unwrap();
        let track = parse_sdp_video(VIRB_SDP, &base).unwrap();
        assert_eq!(
            track.control_url,
            "rtsp://192.168.1.200/livePreviewStream/video?maxResolutionVertical=0"
        );
        assert_eq!(track.payload_type, Some(96));
        assert_eq!(
            setup_candidates(&track.control_url),
            vec![
                "rtsp://192.168.1.200/livePreviewStream/video?maxResolutionVertical=0",
                "rtsp://192.168.1.200/livePreviewStream/video",
            ]
        );
    }

    #[test]
    fn resolves_relative_control_urls() {
        let base = Url::parse("rtsp://cam/stream").unwrap();
        let sdp = "m=video 0 RTP/AVP 97\na=control:trackID=1\na=rtpmap:97 H264/90000\n";
        let track = parse_sdp_video(sdp, &base).unwrap();
        assert_eq!(track.control_url, "rtsp://cam/stream/trackID=1");
        assert_eq!(track.payload_type, Some(97));
    }

    #[test]
    fn rejects_sdp_without_h264_video() {
        let base = Url::parse("rtsp://cam/stream").unwrap();
        assert!(parse_sdp_video("m=audio 0 RTP/AVP 96\na=control:a\n", &base).is_none());
    }

    #[test]
    fn parses_session_and_transport() {
        assert_eq!(parse_session("1085377743"), ("1085377743".into(), None));
        assert_eq!(parse_session("ABC;timeout=60"), ("ABC".into(), Some(60)));
        assert_eq!(
            server_rtp_port(
                "RTP/AVP/UDP;unicast;client_port=50002-50003;server_port=49154-49155;ssrc=838BB3F7"
            ),
            Some(49154)
        );
    }

    #[tokio::test]
    async fn reads_a_response_with_body() {
        let raw = b"RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Length: 5\r\n\r\nhello";
        let mut reader = BufReader::new(&raw[..]);
        let response = read_response(&mut reader).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.header("cseq"), Some("2"));
        assert_eq!(response.body, "hello");
    }

    #[tokio::test]
    async fn setup_retries_without_query_like_the_virb_needs() {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = BufReader::new(socket);
            let mut seen = Vec::new();
            for _ in 0..2 {
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    socket.read_line(&mut line).await.unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    request.push_str(&line);
                }
                let first = request.lines().next().unwrap().to_string();
                let reply = if first.contains('?') {
                    "RTSP/1.0 451 Invalid Parameter\r\nCSeq: 1\r\n\r\n".to_string()
                } else {
                    "RTSP/1.0 200 OK\r\nCSeq: 2\r\nSession: 42;timeout=60\r\n\
                     Transport: RTP/AVP/UDP;unicast;client_port=5000-5001;server_port=6000-6001\r\n\r\n"
                        .to_string()
                };
                seen.push(first);
                socket.get_mut().write_all(reply.as_bytes()).await.unwrap();
            }
            seen
        });

        let url = format!("rtsp://127.0.0.1:{port}/live?x=0");
        let mut client = RtspClient::connect(&url).await.unwrap();
        let track = VideoTrack {
            control_url: format!("rtsp://127.0.0.1:{port}/live/video?x=0"),
            payload_type: Some(96),
        };
        let transport = client.setup(&track, 5000).await.unwrap();
        assert_eq!(transport.session, "42");
        assert_eq!(transport.timeout_secs, Some(60));
        assert_eq!(transport.server_rtp_port, Some(6000));
        let seen = server.await.unwrap();
        assert!(seen[1].starts_with(&format!("SETUP rtsp://127.0.0.1:{port}/live/video RTSP/1.0")));
    }
}
