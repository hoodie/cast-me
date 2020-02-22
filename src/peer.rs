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

use crate::{broker::BrokerMsg, Receiver, Sender};

type WsSender = SplitSink<WebSocket, Message>;
type WsReceiver = SplitStream<WebSocket>;

pub type PeerSender = mpsc::UnboundedSender<PeerMessage>;

// P2P / Broker -> Peer Messages
#[derive(Debug)]
pub enum PeerMessage {
    P2P(String),
    Connected(PeerSender),
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
    Welcome(Uuid),
    Connect(Uuid),
}

pub struct Peer {
    pub my_uuid: Uuid,
    pub correspondent: Option<PeerSender>,
    pub broker_addr: Sender<BrokerMsg>,
    pub peer_sender: Sender<PeerMessage>,
    pub peer_receiver: Receiver<PeerMessage>,
    pub ws_sender: WsSender,
    pub ws_receiver: WsReceiver,
}

impl Peer {
    pub fn new(ws: WebSocket, broker_addr: Sender<BrokerMsg>) -> Self {
        let my_uuid = Uuid::new_v4();
        let (peer_sender, peer_receiver) = mpsc::unbounded_channel::<PeerMessage>();

        let (ws_sender, ws_receiver) = ws.split();

        Peer {
            my_uuid,
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
            uuid: self.my_uuid,
            peer: self.peer_sender.clone(),
        });
    }

    pub async fn send_welcome(&mut self) {
        self.send_to_remote(Protocol::Welcome(self.my_uuid)).await;
    }

    pub async fn start(&mut self) {
        loop {
            tokio::select! {
                received = self.ws_receiver.next() => {
                    if let Some(received) = received {
                        debug!("received on ws {:?}", received);
                        if let Ok(raw_content) = received {

                            match (&mut self.correspondent, raw_content.to_str()) {
                                (None, Ok(_)) => {
                                    if let Ok(Protocol::Connect(uuid)) = raw_content.to_str().and_then(|s|serde_json::from_str(&s).map_err(|_|())) {
                                        debug!("connecting to {}", uuid);
                                        self.send_to_broker(BrokerMsg::Connect {
                                                from: self.my_uuid,
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
                            debug!("unhandled message",);
                        }
                    } else {
                        break
                    }
                }
                Some(received) = self.peer_receiver.next() => {
                    Peer::handle_broker_msg(&mut self.correspondent, received, &mut self.ws_sender).await;
                }
            }
        }

        info!("peer quit {}", self.my_uuid);
    }

    async fn handle_broker_msg(
        mut correspondent: &mut Option<PeerSender>,
        received: PeerMessage,
        socket_tx: &mut SplitSink<WebSocket, Message>,
    ) {
        debug!("peer received from broker {:?}", received);
        match (received, &mut correspondent) {
            // from the correspondent, send on socket
            (PeerMessage::P2P(ref content), _) => {
                if let Err(e) = socket_tx.send(Message::text(content)).await {
                    warn!("failed to send on websocket {:?}", e);
                }
            }

            (PeerMessage::Connected(_), Some(_)) => warn!("already have a correspondent"),

            (PeerMessage::Connected(other_peer), None) => {
                correspondent.replace(other_peer);
                info!("set a correspondent");
            }
        }
    }


    async fn send_to_remote(&mut self, msg: Protocol) {
        if let Err(e) = self
            .ws_sender
            .send(Message::text(serde_json::to_string(&msg).unwrap()))
            .await
        {
            warn!("failed to send on websocket {:?}", e);
        }
    }

    fn send_to_broker(&self, msg: BrokerMsg) {
        self.broker_addr.send(msg).unwrap();
    }
}
