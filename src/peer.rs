use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

use std::fmt;

use crate::{broker::BrokerMsg, Receiver, Sender};

type WsSender = SplitSink<WebSocket, Message>;
type WsReceiver = SplitStream<WebSocket>;

pub type PeerSender = Sender<PeerMessage>;
pub type PeerReceiver = Receiver<PeerMessage>;

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

// P2P: Broker -> Peer Messages
#[derive(Debug)]
pub enum PeerMessage {
    P2P(String),
    Connected(PeerSender, PeerId),
    Disconnected,
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

#[derive(Debug)]
pub enum SendError {
    NoCorrespondant,
    FailedToSendOnWebsocket(mpsc::error::SendError<PeerMessage>),
}

pub struct Peer {
    pub my_id: PeerId,

    /// sender to other participating peer
    pub correspondent: Option<PeerSender>,

    /// sender to broker
    pub broker_addr: Sender<BrokerMsg>,

    /// sender to this peer
    pub peer_sender: PeerSender,

    /// receiver for messages from correspondent
    pub peer_receiver: PeerReceiver,

    /// sender to websocket
    pub ws_sender: WsSender,

    /// receiver on websocket
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

    pub async fn start(&mut self) {
        loop {
            tokio::select! {
                Some(received) = self.ws_receiver.next() => {
                    log::trace!("received on ws {:?}", received);
                    if let Ok(ws_message) = received {

                        if ws_message.is_close() { // TODO: I wish I could just match on the message itself
                            self.retire = true;
                            log::debug!("{:?} websocket disconnected", self.my_id);
                            if let Err(error) = self.send_to_correspondent(PeerMessage::Disconnected).await {
                                log::debug!("{:?}", error);
                            }
                            break
                        }

                        match (&mut self.correspondent, ws_message.to_str()) {
                            (None, Ok(_)) => {
                                if let Ok(Protocol::Connect(uuid)) = ws_message.to_str().and_then(|s|serde_json::from_str(s).map_err(|_|())) {
                                    log::debug!("connecting to {}", uuid);
                                    self.send_to_broker(BrokerMsg::Connect {
                                            from: self.my_id.clone(),
                                            to: uuid,
                                        });
                                } else {
                                    log::trace!("no corresponded, ignoring");
                                }
                            }
                            (Some(ref mut correspondent), Ok(content)) => {
                                if let Err(e) = correspondent.send(PeerMessage::P2P(content.into())) { // TODO: redundant repacking
                                    log::debug!("failed to forward {}", e);
                                    break;
                                }
                            }
                            _ => {}

                        }
                    } else {
                        log::warn!("unhandled message: {:?}", received);
                        break
                    }
                }
                Some(received) = self.peer_receiver.recv() => {
                    // Peer::handle_broker_msg(&mut self.correspondent, received, &mut self.ws_sender).await;
                    self.handle_broker_msg(received).await;
                    if self.retire {
                        break;
                    }
                }
            }
        }

        log::info!("peer quit {}", self.my_id);
    }

    pub async fn send_welcome(&mut self) {
        self.send_to_remote(Protocol::Welcome(self.my_id.clone()))
            .await;
    }

    async fn send_to_correspondent(&mut self, msg: PeerMessage) -> Result<(), SendError> {
        if let Some(ref mut correspondent) = self.correspondent {
            if let Err(e) = correspondent.send(msg) {
                Err(SendError::FailedToSendOnWebsocket(e))
            } else {
                Ok(())
            }
        } else {
            Err(SendError::NoCorrespondant)
        }
    }

    async fn send_to_remote(&mut self, msg: impl ToString) {
        let payload = msg.to_string();
        if let Err(e) = self.ws_sender.send(Message::text(&payload)).await {
            log::warn!("failed to send message on websocket {} {}", payload, e);
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
                log::trace!("peer received P2P");
                self.send_to_remote(content).await;
            }

            (PeerMessage::Connected(..), Some(_)) => log::warn!("already have a correspondent"),

            (PeerMessage::Connected(other_peer, other_peer_id), None) => {
                self.correspondent.replace(other_peer);
                let hail = Protocol::Connected(other_peer_id);
                self.send_to_remote(hail).await;
                log::info!("set a correspondent");
            }
            (PeerMessage::Close, _) => {
                self.retire = true;
                self.send_to_remote(Protocol::Bye {
                    reason: String::from("kicked"),
                })
                .await;
            }
            (PeerMessage::Disconnected, _) => {
                log::debug!("{:?} peer left, retiring", self.my_id);
                self.retire = true;
                self.send_to_remote(Protocol::Bye {
                    reason: String::from("disconnected"),
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
