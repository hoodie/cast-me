use serde::{Deserialize, Serialize};
use std::fmt;

use crate::peer_id::PeerId;

// websocket json protocol
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WsProtocol {
    Welcome(PeerId),
    Connect(PeerId),
    Connected(PeerId),
    Subscribed,
    Bye { reason: String },
}

impl fmt::Display for WsProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = serde_json::to_string(&self)
            .unwrap_or_else(|_| String::from(r#"{"error": "unserializable"}"#));
        write!(f, "{msg}")
    }
}

// pub type WsSender = SplitSink<WebSocket, Message>;
// pub type WsReceiver = SplitStream<WebSocket>;
