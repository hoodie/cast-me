use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt as _};
use hannibal::{prelude::*, Actor, StreamHandler, WeakAddr};

use crate::{actors::protocol::Forward, peer_id::PeerId, ws_protocol::WsProtocol};

type WsSender = SplitSink<WebSocket, Message>;

use super::{
    broker::Broker,
    protocol::{ConnectedFrom, RequestConnectTo},
};

pub struct Peer {
    pub id: PeerId,
    /// sender to websocket
    pub ws_sender: WsSender,
    pub correspondent: Option<WeakAddr<Peer>>,
}

impl Peer {
    pub fn new(sender: WsSender) -> Peer {
        Self {
            id: PeerId::new(),
            ws_sender: sender,
            correspondent: None,
        }
    }

    fn correspondent(&self) -> Option<Addr<Peer>> {
        self.correspondent.as_ref().and_then(|o| o.upgrade())
    }

    async fn handle_ws_message(&mut self, message: WsProtocol) -> anyhow::Result<()> {
        if let WsProtocol::Connect(peer_id) = message {
            tracing::debug!("connecting to {}", peer_id);
            let active = self.id.clone();
            let passive = peer_id.clone();
            match Broker::from_registry()
                .await
                .call(RequestConnectTo { active, passive })
                .await?
            {
                Ok(correspondent) => {
                    tracing::debug!("connected to {}", peer_id);
                    self.correspondent.replace(correspondent);
                }
                Err(error) => {
                    tracing::warn!("failed to connect to {} ({})", peer_id, error);
                }
            }
        }
        Ok(())
    }
}

impl Actor for Peer {
    async fn started(&mut self, ctx: &mut hannibal::Context<Self>) -> hannibal::DynResult {
        tracing::info!(peer = ?self.id, "peer started");
        self.ws_sender
            .send(WsProtocol::Welcome(self.id.clone()).to_string().into())
            .await?;
        Broker::from_registry()
            .await
            .send(super::protocol::Register {
                id: self.id.clone(),
                addr: ctx.weak_address().unwrap(),
            })
            .await?;

        Ok(())
    }

    async fn stopped(&mut self, _: &mut hannibal::Context<Self>) {
        tracing::info!("peer stopped");
    }
}

type WsStreamMessage = Result<Message, axum::Error>;

/// Messages from the client
impl StreamHandler<WsStreamMessage> for Peer {
    async fn handle(&mut self, ctx: &mut hannibal::Context<Self>, msg: WsStreamMessage) {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!("peer received text: {text}");

                if let Some(correspondent) = self.correspondent() {
                    if let Err(error) = correspondent.send(Forward(text.to_string())).await {
                        tracing::warn!(peer = ?self.id, "error forwarding message: {error}");
                    }
                } else {
                    match serde_json::from_str::<WsProtocol>(text.as_str()) {
                        Ok(message) => {
                            if let Err(err) = self.handle_ws_message(message).await {
                                tracing::error!("error handling websocket message: {err}");
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                peer = ?self.id,
                                ?text,
                                "peer received invalid message: {error}"
                            );
                        }
                    }
                }
            }

            Ok(Message::Close(_close_frame)) => {
                tracing::info!("websocket terminated by other side");
                if let Err(error) = ctx.stop() {
                    tracing::error!(peer = ?self.id, "error stopping peer actor: {error}");
                }
            }
            _ => {}
        };
    }

    async fn finished(&mut self, _ctx: &mut hannibal::Context<Self>) {
        tracing::info!("websocket stream ended")
    }
}

/// Message from Broker that the active peer has connected to you
impl Handler<ConnectedFrom> for Peer {
    async fn handle(&mut self, _ctx: &mut hannibal::Context<Self>, msg: ConnectedFrom) {
        tracing::debug!(peer = ?self.id, "connected from {}", msg.id);
        self.correspondent.replace(msg.addr);
        if let Err(error) = self
            .ws_sender
            .send(WsProtocol::Connected(msg.id).to_string().into())
            .await
        {
            tracing::warn!("failed to send connected message to client ({error})");
        }
    }
}

/// Message from the other peer for you to forward to the client
impl Handler<Forward> for Peer {
    async fn handle(&mut self, _ctx: &mut Context<Self>, Forward(msg): Forward) {
        tracing::debug!("forwarding message {msg}",);
        if let Err(error) = self.ws_sender.send(msg.into()).await {
            tracing::warn!("error forwarding message: {error}");
        }
    }
}
