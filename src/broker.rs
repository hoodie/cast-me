use std::{collections::BTreeMap, env, error::Error, sync::Arc};

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

use crate::{
    peer::{PeerMessage, PeerSender},
    Receiver, Sender,
};

// Peer -> Broker Messages
#[derive(Debug)]
pub enum BrokerMsg {
    Register { uuid: Uuid, peer: PeerSender },
    Connect { from: Uuid, to: Uuid },
}

#[derive(Clone)]
pub struct Broker {
    to_broker: Sender<BrokerMsg>,
}

impl Broker {
    pub fn addr(self) -> Sender<BrokerMsg> {
        self.to_broker
    }

    fn register_peer(
        loose_channels: &mut BTreeMap<Uuid, PeerSender>,
        uuid: Uuid,
        peer: PeerSender,
    ) {
        if let Some(_peer) = loose_channels.insert(uuid, peer) {
            warn!("uuid collision {}", uuid);
        }
        info!(
            "registered peer under {} {:#?}",
            uuid,
            loose_channels.keys()
        );
    }

    fn connect_peers(loose_channels: &mut BTreeMap<Uuid, PeerSender>, from: Uuid, to: Uuid) {
        if let (Some(peer_a), Some(peer_b)) =
            (loose_channels.remove(&from), loose_channels.remove(&to))
        {
            info!("connecting peers {} and {}", from, to);
            match (
                peer_a.send(PeerMessage::Connected(peer_b.clone())),
                peer_b.send(PeerMessage::Connected(peer_a)),
            ) {
                (Err(err), _) => error!("failed to send b to a, reason: {}", err),
                (_, Err(err)) => error!("failed to send a to b, reason: {}", err),
                _ => info!(
                    "connected {} with {} | inventory={:#?}",
                    from,
                    to,
                    loose_channels.keys()
                ),
            }
        } else {
            warn!("no uuid match {} {}", from, to);
        }
    }

    pub fn create() -> (Broker, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::unbounded_channel();

        // only those that don't have a partner yet
        let mut loose_channels: BTreeMap<Uuid, PeerSender> = BTreeMap::new();

        let broker_loop = task::spawn(async move {
            debug!("broker loop");
            while let Some(res) = rx.next().await {
                debug!("broker received {:?}", res);
                match res {
                    BrokerMsg::Register { uuid, peer } => {
                        Self::register_peer(&mut loose_channels, uuid, peer)
                    }

                    BrokerMsg::Connect { from, to } => {
                        Self::connect_peers(&mut loose_channels, from, to)
                    }
                }
            }
        });

        (Broker { to_broker: tx }, broker_loop)
    }
}
