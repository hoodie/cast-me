use env_logger::Env;
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream},
    FutureExt, StreamExt,
};
use log::*;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
};
use uuid::Uuid;
use warp::{
    ws::{Message, WebSocket},
    Filter,
};

use super::*;

type WsSender = SplitSink<WebSocket, Message>;
type WsReceiver = SplitStream<WebSocket>;

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
    pub async fn handle_broker_msg(
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

    pub fn register_at_broker(&mut self) {
        // register at broker
        self.broker_addr
            .send(BrokerMsg::Register {
                uuid: self.my_uuid,
                peer: self.peer_sender.clone(),
            })
            .unwrap();
    }

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

    pub async fn send_welcome(&mut self) {
        // send uuid to user
        if let Err(e) = self
            .ws_sender
            .send(Message::text(
                serde_json::to_string(&Protocol::Welcome(self.my_uuid)).unwrap(),
            ))
            .await
        {
            warn!("failed to send on websocket {:?}", e);
        }
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
                                        self.broker_addr
                                            .send(BrokerMsg::Connect {
                                                from: self.my_uuid,
                                                to: uuid,
                                            })
                                            .unwrap();
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

        // info!("peer quit {}", my_uuid);
    }
}
