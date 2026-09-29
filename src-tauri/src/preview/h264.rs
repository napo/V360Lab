//! H.264 over RTP (RFC 6184) depacketization into access units.
//!
//! Handles single NAL unit packets, STAP-A aggregates and FU-A fragments.
//! Access units are emitted in Annex B form (start codes), which WebCodecs'
//! `VideoDecoder` accepts directly. After packet loss, output resumes at the
//! next keyframe so the decoder never sees a corrupted reference chain.

const NAL_IDR: u8 = 5;
const NAL_SPS: u8 = 7;
const NAL_PPS: u8 = 8;
const NAL_STAP_A: u8 = 24;
const NAL_FU_A: u8 = 28;
const START_CODE: [u8; 4] = [0, 0, 0, 1];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessUnit {
    /// RTP timestamp (90 kHz clock).
    pub rtp_timestamp: u32,
    pub keyframe: bool,
    /// Annex B bytestream; keyframes always start with SPS and PPS.
    pub data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Depacketizer {
    payload_type: Option<u8>,
    expected_seq: Option<u16>,
    current_ts: Option<u32>,
    nals: Vec<Vec<u8>>,
    fragment: Option<Vec<u8>>,
    corrupted: bool,
    waiting_for_keyframe: bool,
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
}

impl Depacketizer {
    /// `payload_type`: dynamic RTP payload type from the SDP (e.g. 96).
    pub fn new(payload_type: Option<u8>) -> Self {
        Self {
            payload_type,
            waiting_for_keyframe: true,
            ..Self::default()
        }
    }

    /// WebCodecs codec string (`avc1.PPCCLL`) from the last SPS seen.
    pub fn codec(&self) -> Option<String> {
        let sps = self.sps.as_ref()?;
        (sps.len() >= 4).then(|| format!("avc1.{:02X}{:02X}{:02X}", sps[1], sps[2], sps[3]))
    }

    /// Feeds one RTP packet; returns completed access units (usually 0 or 1).
    pub fn push(&mut self, packet: &[u8]) -> Vec<AccessUnit> {
        let Some(rtp) = RtpPacket::parse(packet) else {
            return Vec::new();
        };
        if self.payload_type.is_some_and(|pt| pt != rtp.payload_type) {
            return Vec::new();
        }

        let mut out = Vec::new();
        if let Some(expected) = self.expected_seq {
            if rtp.sequence != expected {
                log::debug!("RTP loss: expected {expected}, got {}", rtp.sequence);
                self.corrupted = true;
                self.fragment = None;
            }
        }
        self.expected_seq = Some(rtp.sequence.wrapping_add(1));

        if self.current_ts.is_some_and(|ts| ts != rtp.timestamp) {
            out.extend(self.finish());
        }
        self.current_ts = Some(rtp.timestamp);
        self.push_payload(rtp.payload);
        if rtp.marker {
            out.extend(self.finish());
        }
        out
    }

    fn push_payload(&mut self, payload: &[u8]) {
        let Some(&header) = payload.first() else {
            return;
        };
        match header & 0x1f {
            1..=23 => self.nals.push(payload.to_vec()),
            NAL_STAP_A => {
                let mut rest = &payload[1..];
                while rest.len() >= 2 {
                    let size = u16::from_be_bytes([rest[0], rest[1]]) as usize;
                    if size == 0 || rest.len() < 2 + size {
                        self.corrupted = true;
                        break;
                    }
                    self.nals.push(rest[2..2 + size].to_vec());
                    rest = &rest[2 + size..];
                }
            }
            NAL_FU_A => {
                if payload.len() < 2 {
                    self.corrupted = true;
                    return;
                }
                let fu_header = payload[1];
                let start = fu_header & 0x80 != 0;
                let end = fu_header & 0x40 != 0;
                if start {
                    let nal_header = (header & 0xe0) | (fu_header & 0x1f);
                    let mut nal = Vec::with_capacity(payload.len() * 4);
                    nal.push(nal_header);
                    nal.extend_from_slice(&payload[2..]);
                    self.fragment = Some(nal);
                } else if let Some(nal) = self.fragment.as_mut() {
                    nal.extend_from_slice(&payload[2..]);
                } else {
                    // Middle/end fragment without a start: its start was lost.
                    self.corrupted = true;
                    return;
                }
                if end {
                    if let Some(nal) = self.fragment.take() {
                        self.nals.push(nal);
                    }
                }
            }
            _ => {} // STAP-B, MTAP, FU-B: not used by the VIRB.
        }
    }

    fn finish(&mut self) -> Option<AccessUnit> {
        let timestamp = self.current_ts.take()?;
        let nals = std::mem::take(&mut self.nals);
        let corrupted = std::mem::replace(&mut self.corrupted, false);
        self.fragment = None;

        for nal in &nals {
            match nal.first().map(|h| h & 0x1f) {
                Some(NAL_SPS) => self.sps = Some(nal.clone()),
                Some(NAL_PPS) => self.pps = Some(nal.clone()),
                _ => {}
            }
        }
        let keyframe = nals.iter().any(|n| n.first().map(|h| h & 0x1f) == Some(NAL_IDR));

        if corrupted {
            self.waiting_for_keyframe = true;
            return None;
        }
        if nals.iter().all(|n| matches!(n.first().map(|h| h & 0x1f), Some(NAL_SPS | NAL_PPS))) {
            return None; // parameter sets only; kept for the next keyframe
        }
        if self.waiting_for_keyframe {
            if !keyframe || self.sps.is_none() || self.pps.is_none() {
                return None;
            }
            self.waiting_for_keyframe = false;
        }

        let mut data = Vec::new();
        if keyframe {
            let has = |t: u8| nals.iter().any(|n| n.first().map(|h| h & 0x1f) == Some(t));
            if !has(NAL_SPS) {
                push_nal(&mut data, self.sps.as_deref().unwrap_or_default());
            }
            if !has(NAL_PPS) {
                push_nal(&mut data, self.pps.as_deref().unwrap_or_default());
            }
        }
        for nal in &nals {
            push_nal(&mut data, nal);
        }
        Some(AccessUnit {
            rtp_timestamp: timestamp,
            keyframe,
            data,
        })
    }
}

fn push_nal(out: &mut Vec<u8>, nal: &[u8]) {
    if !nal.is_empty() {
        out.extend_from_slice(&START_CODE);
        out.extend_from_slice(nal);
    }
}

struct RtpPacket<'a> {
    payload_type: u8,
    marker: bool,
    sequence: u16,
    timestamp: u32,
    payload: &'a [u8],
}

impl<'a> RtpPacket<'a> {
    fn parse(packet: &'a [u8]) -> Option<Self> {
        if packet.len() < 12 || packet[0] >> 6 != 2 {
            return None;
        }
        let padding = packet[0] & 0x20 != 0;
        let extension = packet[0] & 0x10 != 0;
        let csrc_count = (packet[0] & 0x0f) as usize;
        let mut offset = 12 + 4 * csrc_count;
        if extension {
            let words = u16::from_be_bytes([*packet.get(offset + 2)?, *packet.get(offset + 3)?]) as usize;
            offset += 4 + 4 * words;
        }
        let mut end = packet.len();
        if padding {
            end = end.checked_sub(*packet.last()? as usize)?;
        }
        if offset > end {
            return None;
        }
        Some(Self {
            payload_type: packet[1] & 0x7f,
            marker: packet[1] & 0x80 != 0,
            sequence: u16::from_be_bytes([packet[2], packet[3]]),
            timestamp: u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]),
            payload: &packet[offset..end],
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn rtp(seq: u16, ts: u32, marker: bool, payload: &[u8]) -> Vec<u8> {
        let mut p = vec![0x80, if marker { 0x80 | 96 } else { 96 }];
        p.extend_from_slice(&seq.to_be_bytes());
        p.extend_from_slice(&ts.to_be_bytes());
        p.extend_from_slice(&0x1234_5678u32.to_be_bytes());
        p.extend_from_slice(payload);
        p
    }

    pub const SPS: [u8; 5] = [0x67, 0x64, 0x00, 0x28, 0xac];
    pub const PPS: [u8; 3] = [0x68, 0xee, 0x3c];

    fn stap_a(nals: &[&[u8]]) -> Vec<u8> {
        let mut p = vec![NAL_STAP_A | 0x60];
        for nal in nals {
            p.extend_from_slice(&(nal.len() as u16).to_be_bytes());
            p.extend_from_slice(nal);
        }
        p
    }

    /// An IDR slice split into three FU-A fragments.
    pub fn idr_fragments(seq: u16, ts: u32) -> Vec<Vec<u8>> {
        vec![
            rtp(seq, ts, false, &[0x7c, 0x85, 1, 2]),
            rtp(seq + 1, ts, false, &[0x7c, 0x05, 3, 4]),
            rtp(seq + 2, ts, true, &[0x7c, 0x45, 5]),
        ]
    }

    #[test]
    fn assembles_keyframe_with_parameter_sets() {
        let mut d = Depacketizer::new(Some(96));
        assert!(d.push(&rtp(10, 1000, false, &stap_a(&[&SPS, &PPS]))).is_empty());
        let mut units = Vec::new();
        for p in idr_fragments(11, 1000) {
            units.extend(d.push(&p));
        }
        assert_eq!(units.len(), 1);
        let au = &units[0];
        assert!(au.keyframe);
        let mut expected = Vec::new();
        for nal in [&SPS[..], &PPS[..], &[0x65, 1, 2, 3, 4, 5][..]] {
            expected.extend_from_slice(&START_CODE);
            expected.extend_from_slice(nal);
        }
        assert_eq!(au.data, expected);
        assert_eq!(d.codec().as_deref(), Some("avc1.640028"));
    }

    #[test]
    fn prepends_stored_parameter_sets_to_later_keyframes() {
        let mut d = Depacketizer::new(Some(96));
        d.push(&rtp(1, 0, true, &stap_a(&[&SPS, &PPS])));
        let units: Vec<_> = idr_fragments(2, 3000).iter().flat_map(|p| d.push(p)).collect();
        assert!(units[0].data.starts_with(&[0, 0, 0, 1, 0x67]));
    }

    #[test]
    fn waits_for_keyframe_after_loss() {
        let mut d = Depacketizer::new(Some(96));
        d.push(&rtp(1, 0, false, &stap_a(&[&SPS, &PPS])));
        let first: Vec<_> = idr_fragments(2, 0).iter().flat_map(|p| d.push(p)).collect();
        assert_eq!(first.len(), 1);
        // Delta frame OK.
        assert_eq!(d.push(&rtp(5, 3000, true, &[0x41, 9, 9])).len(), 1);
        // Packet 6 lost: the next delta frame is dropped...
        assert!(d.push(&rtp(7, 6000, true, &[0x41, 8, 8])).is_empty());
        assert!(d.push(&rtp(8, 9000, true, &[0x41, 7, 7])).is_empty());
        // ...until the next keyframe.
        let key: Vec<_> = idr_fragments(9, 12000).iter().flat_map(|p| d.push(p)).collect();
        assert!(key[0].keyframe);
    }

    #[test]
    fn ignores_other_payload_types_and_garbage() {
        let mut d = Depacketizer::new(Some(96));
        let mut audio = rtp(1, 0, true, &[1, 2, 3]);
        audio[1] = 0x80 | 97;
        assert!(d.push(&audio).is_empty());
        assert!(d.push(&[1, 2, 3]).is_empty());
        assert!(d.codec().is_none());
    }
}
