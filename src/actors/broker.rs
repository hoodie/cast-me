use hannibal::{prelude::*, Handler, WeakAddr};

use std::{collections::HashMap, time::Duration};

use crate::PeerId;

use super::{
    peer::Peer,
    protocol::{ConnectedFrom, Register, RequestConnectTo},
};

#[derive(Service, Default)]
pub struct Broker {
    peers: HashMap<PeerId, WeakAddr<Peer>>,
}

impl Actor for Broker {
    async fn started(&mut self, ctx: &mut Context<Self>) -> DynResult {
        tracing::info!("Broker started");
        ctx.interval(GC, Duration::from_secs(2));

        Ok(())
    }

    async fn stopped(&mut self, _ctx: &mut Context<Self>) {
        tracing::info!("Broker stopped");
    }
}

#[derive(Clone, Message)]
struct GC;
/// Remind yourself regularly to clean up.
impl Handler<GC> for Broker {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _: GC) {
        if !self.peers.is_empty() {
            let len_before = self.peers.len();
            self.peers.retain(|_, peer| !peer.stopped());
            let len_after = self.peers.len();
            if len_after != len_before {
                tracing::debug!("retained {len_after}/{len_before} peers");
            }
        }
    }
}

/// Message from a Peer that it registers its addr under a given id.
impl Handler<Register> for Broker {
    async fn handle(&mut self, _ctx: &mut hannibal::Context<Self>, msg: Register) {
        tracing::info!("registering peer {}", msg.id);
        self.peers.insert(msg.id, msg.addr);
    }
}

/// Message from a Peer that it wants to connect to another peer.
impl Handler<RequestConnectTo> for Broker {
    async fn handle(
        &mut self,
        _ctx: &mut hannibal::Context<Self>,
        msg: RequestConnectTo,
    ) -> Result<WeakAddr<Peer>, String> {
        let RequestConnectTo { active, passive } = msg;

        tracing::debug!("{active} is trying to connect to {passive}");

        if active == passive {
            tracing::warn!("attempted to connect to self");
            return Err("attempted to connect to self".to_string());
        }

        let active_addr = self.peers.remove(&active).filter(|other| !other.stopped());

        let Some(passive_addr) = self.peers.remove(&passive) else {
            tracing::warn!("passive peer not found");
            return Err("passive peer not found".to_string());
        };

        let Some(passive_addr) = passive_addr.upgrade() else {
            tracing::warn!("passive peer not running");
            return Err("passive peer not running".to_string());
        };

        let Some(active_addr) = active_addr else {
            tracing::warn!("active peer not found");
            return Err("active peer not found".to_string());
        };

        if let Err(err) = passive_addr
            .send(ConnectedFrom {
                id: active,
                addr: active_addr.clone(),
            })
            .await
        {
            tracing::warn!("failed to connect to peer: {}", err);
            Err("failed to connect to peer".to_string())
        } else {
            if let Err(err) = active_addr
                .upgrade()
                .unwrap()
                .send(ConnectedFrom {
                    id: passive,
                    addr: passive_addr.downgrade(),
                })
                .await
            {
                tracing::warn!("failed to connect to peer: {}", err);
            }
            Ok(passive_addr.downgrade())
        }
    }
}
