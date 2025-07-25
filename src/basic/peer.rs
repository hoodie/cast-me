#![allow(clippy::suspicious_else_formatting)]

use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};

use crate::{PeerId, WsProtocol};

use super::{BrokerMsg, Receiver, Sender};

type WsSender = SplitSink<WebSocket, Message>;
type WsReceiver = SplitStream<WebSocket>;

pub type PeerSender = Sender<PeerMessage>;
pub type PeerReceiver = Receiver<PeerMessage>;

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

mod protocol {}

#[derive(Debug)]
pub enum SendError {
    NoCorrespondant,
    FailedToSendOnWebsocket(#[allow(dead_code)] mpsc::error::SendError<PeerMessage>),
}

#[derive(Debug)]
pub struct Peer {
    pub id: PeerId,

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
            id: my_id,
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
            uuid: self.id.clone(),
            peer: self.peer_sender.clone(),
        });
    }

    #[tracing::instrument]
    pub async fn start(&mut self) {
        loop {
            tokio::select! {
                Some(received) = self.ws_receiver.next() => {
                    tracing::trace!("received on ws {:?}", received);
                    if let Ok(ws_message) = received {

                        if ws_message.is_close() { // TODO: I wish I could just match on the message itself
                            self.retire = true;
                            tracing::debug!("{:?} websocket disconnected", self.id);
                            if let Err(error) = self.send_to_correspondent(PeerMessage::Disconnected).await {
                                tracing::debug!("{:?}", error);
                            }
                            break
                        }

                        match (&mut self.correspondent, ws_message.to_str()) {
                            (None, Ok(_)) => {
                                if let Ok(WsProtocol::Connect(uuid)) = ws_message.to_str().and_then(|s|serde_json::from_str(s).map_err(|_|())) {
                                    tracing::debug!("connecting to {}", uuid);
                                    self.send_to_broker(BrokerMsg::Connect {
                                            from: self.id.clone(),
                                            to: uuid,
                                        });
                                } else {
                                    tracing::trace!("no corresponded, ignoring");
                                }
                            }
                            (Some(ref mut correspondent), Ok(content)) => {
                                if let Err(e) = correspondent.send(PeerMessage::P2P(content.into())) { // TODO: redundant repacking
                                    tracing::debug!("failed to forward {}", e);
                                    break;
                                }
                            }
                            _ => {}

                        }
                    } else {
                        tracing::warn!("unhandled message: {:?}", received);
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

        tracing::info!("peer quit {}", self.id);
    }

    #[tracing::instrument]
    pub async fn send_welcome(&mut self) {
        self.send_to_remote(&WsProtocol::Welcome(self.id.clone()).to_string())
            .await;
    }

    #[tracing::instrument]
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

    #[tracing::instrument]
    async fn send_to_remote(&mut self, msg: &str) {
        let payload = msg.to_string();
        if let Err(e) = self.ws_sender.send(Message::text(&payload)).await {
            tracing::warn!("failed to send message on websocket {} {}", payload, e);
        }
    }

    #[tracing::instrument]
    async fn handle_broker_msg(
        // mut correspondent: &mut Option<PeerSender>,
        &mut self,
        received: PeerMessage,
        // socket_tx: &mut SplitSink<WebSocket, Message>,
    ) {
        match (received, &mut self.correspondent) {
            // from the correspondent, send on socket
            (PeerMessage::P2P(ref content), _) => {
                tracing::trace!("peer received P2P");
                self.send_to_remote(content).await;
            }

            (PeerMessage::Connected(..), Some(_)) => tracing::warn!("already have a correspondent"),

            (PeerMessage::Connected(other_peer, other_peer_id), None) => {
                self.correspondent.replace(other_peer);
                let hail = WsProtocol::Connected(other_peer_id);
                self.send_to_remote(&hail.to_string()).await;
                tracing::info!("set a correspondent");
            }
            (PeerMessage::Close, _) => {
                self.retire = true;
                self.send_to_remote(
                    &WsProtocol::Bye {
                        reason: String::from("kicked"),
                    }
                    .to_string(),
                )
                .await;
            }
            (PeerMessage::Disconnected, _) => {
                tracing::debug!("{:?} peer left, retiring", self.id);
                self.retire = true;
                self.send_to_remote(
                    &WsProtocol::Bye {
                        reason: String::from("disconnected"),
                    }
                    .to_string(),
                )
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
