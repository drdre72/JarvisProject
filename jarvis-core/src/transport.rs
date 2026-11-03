//! Versioned JSON-lines frame codec — the IPC contract between the Rust
//! supervisor and Python workers. Phase Six wires it to stdio/named pipes;
//! sockets are never used (ADR-0001).
//!
//! Every frame is size-capped *before* parsing and version-checked after.
//! Malformed input produces typed errors, never a panic or a guess.

use crate::events::EventEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;

use crate::ids;

pub const PROTOCOL_VERSION: u16 = 1;
/// Hard frame cap — a single runaway line cannot exhaust memory.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireFrame {
    pub protocol_version: u16,
    pub msg_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub sent_at_utc: OffsetDateTime,
    pub body: FrameBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum FrameBody {
    Event(EventEnvelope),
    Request {
        request_id: String,
        op: String,
        args: Value,
    },
    Response {
        request_id: String,
        ok: bool,
        body: Value,
    },
    Heartbeat {
        worker: String,
    },
    Shutdown {
        reason: Option<String>,
    },
}

impl WireFrame {
    pub fn new(body: FrameBody) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            msg_id: ids::new_id("msg"),
            sent_at_utc: OffsetDateTime::now_utc(),
            body,
        }
    }
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("frame exceeds {0} bytes")]
    Oversized(usize),
    #[error("empty frame")]
    Empty,
    #[error("malformed frame: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("unsupported protocol_version {0}")]
    UnsupportedVersion(u16),
    #[error("msg_id must be 1..=64 chars")]
    BadId,
}

/// Serialize one frame as a single line (JSON-lines: `\n` terminated).
pub fn encode(frame: &WireFrame) -> Result<String, TransportError> {
    let mut s = serde_json::to_string(frame)?;
    if s.len() > MAX_FRAME_BYTES {
        return Err(TransportError::Oversized(MAX_FRAME_BYTES));
    }
    s.push('\n');
    Ok(s)
}

/// Parse one line. Size is enforced before deserialization — untrusted
/// input can never make the parser eat an unbounded blob.
pub fn decode(line: &str, max_bytes: usize) -> Result<WireFrame, TransportError> {
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        return Err(TransportError::Empty);
    }
    if trimmed.len() > max_bytes {
        return Err(TransportError::Oversized(max_bytes));
    }
    let frame: WireFrame = serde_json::from_str(trimmed)?;
    if frame.protocol_version != PROTOCOL_VERSION {
        return Err(TransportError::UnsupportedVersion(frame.protocol_version));
    }
    if frame.msg_id.is_empty() || frame.msg_id.len() > 64 {
        return Err(TransportError::BadId);
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventSource, EventType};
    use serde_json::json;

    #[test]
    fn event_frame_roundtrips() {
        let env = EventEnvelope::create(
            EventSource::Ui,
            EventType::UserRequestReceived,
            None,
            json!({"text": "hello"}),
        );
        let frame = WireFrame::new(FrameBody::Event(env));
        let line = encode(&frame).unwrap();
        assert!(line.ends_with('\n'));
        let decoded = decode(&line, MAX_FRAME_BYTES).unwrap();
        assert_eq!(decoded.protocol_version, PROTOCOL_VERSION);
        match decoded.body {
            FrameBody::Event(e) => assert_eq!(e.event_type, EventType::UserRequestReceived),
            _ => panic!("expected event frame"),
        }
    }

    #[test]
    fn wire_shape_is_versioned_contract() {
        let frame = WireFrame::new(FrameBody::Heartbeat {
            worker: "stt".into(),
        });
        let line = encode(&frame).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["protocol_version"], 1);
        assert_eq!(v["body"]["kind"], "heartbeat");
        assert_eq!(v["body"]["data"]["worker"], "stt");
    }

    #[test]
    fn oversized_line_is_rejected_before_parsing() {
        let huge = "x".repeat(MAX_FRAME_BYTES + 1);
        assert!(matches!(
            decode(&huge, MAX_FRAME_BYTES),
            Err(TransportError::Oversized(_))
        ));
    }

    #[test]
    fn wrong_version_is_rejected() {
        let frame = WireFrame::new(FrameBody::Heartbeat { worker: "w".into() });
        let line = encode(&frame)
            .unwrap()
            .replace("\"protocol_version\":1", "\"protocol_version\":99");
        assert!(matches!(
            decode(&line, MAX_FRAME_BYTES),
            Err(TransportError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn malformed_json_is_typed_error() {
        assert!(matches!(
            decode("{not json", MAX_FRAME_BYTES),
            Err(TransportError::Malformed(_))
        ));
        assert!(matches!(
            decode("", MAX_FRAME_BYTES),
            Err(TransportError::Empty)
        ));
    }
}
