use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use log::*;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

use std::fmt;

use crate::{broker::BrokerMsg, Receiver, Sender};

type WsSender = SplitSink<WebSocket, Message>;
type WsReceiver = SplitStream<WebSocket>;

pub type PeerSender = mpsc::UnboundedSender<PeerMessage>;

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

impl PeerId {
    fn new() -> PeerId {
        PeerId(human_hash::humanize(&Uuid::new_v4(), 2))
    }
}

impl Clone for PeerId {
    fn clone(&self) -> PeerId {
        PeerId(self.0.clone())
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// P2P / Broker -> Peer Messages
#[derive(Debug)]
pub enum PeerMessage {
    P2P(String),
    Connected(PeerSender, PeerId),
    Ping,
    Close,
}

impl From<&str> for PeerMessage {
    fn from(s: &str) -> Self {
        Self::P2P(s.into())
    }
}

// websocket json protocol
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Protocol {
    Welcome(PeerId),
    Connect(PeerId),
    Connected(PeerId),
    Bye { reason: String },
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = serde_json::to_string(&self)
            .unwrap_or_else(|_| String::from(r#"{"error": "unserializable"}"#));
        write!(f, "{}", msg)
    }
}

pub struct Peer {
    pub my_id: PeerId,
    pub correspondent: Option<PeerSender>,
    pub broker_addr: Sender<BrokerMsg>,
    pub peer_sender: Sender<PeerMessage>,
    pub peer_receiver: Receiver<PeerMessage>,
    pub ws_sender: WsSender,
    pub ws_receiver: WsReceiver,
    retire: bool,
}

impl Peer {
    pub fn new(ws: WebSocket, broker_addr: Sender<BrokerMsg>) -> Self {
        let my_id = PeerId::new();
        let (peer_sender, peer_receiver) = mpsc::unbounded_channel::<PeerMessage>();

        let (ws_sender, ws_receiver) = ws.split();

        Peer {
            my_id,
            retire: false,
            correspondent: None,
            broker_addr,
            peer_receiver,
            peer_sender,
            ws_receiver,
            ws_sender,
        }
    }

    pub fn register_at_broker(&mut self) {
        self.send_to_broker(BrokerMsg::Register {
            uuid: self.my_id.clone(),
            peer: self.peer_sender.clone(),
        });
    }

    pub async fn send_welcome(&mut self) {
        self.send_to_remote(Protocol::Welcome(self.my_id.clone()))
            .await;
    }

    pub async fn start(&mut self) {
        loop {
            tokio::select! {
                Some(received) = self.ws_receiver.next() => {
                    trace!("received on ws {:?}", received);
                    if let Ok(raw_content) = received {

                        match (&mut self.correspondent, raw_content.to_str()) {
                            (None, Ok(_)) => {
                                if let Ok(Protocol::Connect(uuid)) = raw_content.to_str().and_then(|s|serde_json::from_str(&s).map_err(|_|())) {
                                    debug!("connecting to {}", uuid);
                                    self.send_to_broker(BrokerMsg::Connect {
                                            from: self.my_id.clone(),
                                            to: uuid,
                                        });
                                } else {
                                    trace!("no corresponded, ignoring");
                                }
                            }
                            (Some(ref mut correspondent), Ok(content)) => {
                                if let Err(e) = correspondent.send(PeerMessage::P2P(content.into())) { // TODO: redundant repacking
                                    debug!("failed to forward {}", e);
                                    break;
                                }
                            }
                            _ => {}

                        }
                    } else {
                        warn!("unhandled message: {:?}", received);
                        break
                    }
                }
                Some(received) = self.peer_receiver.next() => {
                    // Peer::handle_broker_msg(&mut self.correspondent, received, &mut self.ws_sender).await;
                    self.handle_broker_msg(received).await;
                    if self.retire {
                        break;
                    }
                }
            }
        }

        info!("peer quit {}", self.my_id);
    }

    async fn send_to_remote(&mut self, msg: impl ToString) {
        let payload = msg.to_string();
        if let Err(e) = self.ws_sender.send(Message::text(&payload)).await {
            warn!("failed to send message on websocket {} {}", payload, e);
        }
    }

    async fn handle_broker_msg(
        // mut correspondent: &mut Option<PeerSender>,
        &mut self,
        received: PeerMessage,
        // socket_tx: &mut SplitSink<WebSocket, Message>,
    ) {
        match (received, &mut self.correspondent) {
            // from the correspondent, send on socket
            (PeerMessage::P2P(ref content), _) => {
                trace!("peer received P2P");
                self.send_to_remote(content).await;
            }

            (PeerMessage::Connected(..), Some(_)) => warn!("already have a correspondent"),

            (PeerMessage::Connected(other_peer, other_peer_id), None) => {
                self.correspondent.replace(other_peer);
                let hail = Protocol::Connected(other_peer_id.into());
                self.send_to_remote(hail).await;
                info!("set a correspondent");
            }
            (PeerMessage::Close, _) => {
                self.retire = true;
                self.send_to_remote(Protocol::Bye {
                    reason: String::from("kicked"),
                })
                .await;
            }
            (PeerMessage::Ping, _) => {
                // I'm alive
            }
        }
    }

    fn send_to_broker(&self, msg: BrokerMsg) {
        self.broker_addr.send(msg).unwrap();
    }
}
