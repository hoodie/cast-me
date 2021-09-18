use std::collections::HashMap;

use tokio::{
    sync::mpsc,
    task::{self, JoinHandle},
};

use crate::{
    peer::{PeerId, PeerMessage, PeerSender},
    Sender,
};

// Peer -> Broker Messages
#[derive(Debug)]
pub enum BrokerMsg {
    Register { uuid: PeerId, peer: PeerSender },
    Connect { from: PeerId, to: PeerId },
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
        loose_channels: &mut HashMap<PeerId, PeerSender>,
        uuid: &PeerId,
        peer: PeerSender,
    ) {
        if let Some(_peer) = loose_channels.insert(uuid.clone(), peer) {
            log::warn!("uuid collision {}", uuid);
        }
        log::info!(
            "registered peer under {} {:#?}",
            uuid,
            loose_channels.keys()
        );
    }

    fn connect_peers(loose_channels: &mut HashMap<PeerId, PeerSender>, from: &PeerId, to: &PeerId) {
        // don't be fooled
        if from == to {
            if let Some(bad_guy) = loose_channels.remove(from) {
                if let Err(e) = bad_guy.send(PeerMessage::Close) {
                    log::warn!("failed to send close to {} {}", from, e);
                }
                return;
            }
        }

        if let (true, Some(peer_b)) = (
            loose_channels.contains_key(from),
            loose_channels.remove(to),
        ) {
            let peer_a = loose_channels.remove(from).unwrap();
            log::info!("connecting peers {} and {}", from, to);
            match (
                peer_a.send(PeerMessage::Connected(peer_b.clone(), to.clone())),
                peer_b.send(PeerMessage::Connected(peer_a, from.clone())),
            ) {
                (Err(err), _) => log::error!("failed to send b to a, reason: {}", err),
                (_, Err(err)) => log::error!("failed to send a to b, reason: {}", err),
                _ => log::info!(
                    "connected {} with {} | inventory={:#?}",
                    from,
                    to,
                    loose_channels.keys()
                ),
            }
        } else {
            log::warn!("no uuid match {} {}", from, to);
        }
    }

    fn clean_out_dead_peers(loose_channels: &mut HashMap<PeerId, PeerSender>) {
        loose_channels.retain(|uuid, peer| {
            if let Err(e) = peer.send(PeerMessage::Ping) {
                log::debug!("removing peer {}, {}", uuid, e);
                return false;
            };
            true
        });
    }

    pub fn create() -> (Broker, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::unbounded_channel();

        // only those that don't have a partner yet
        let mut loose_channels: HashMap<PeerId, PeerSender> = HashMap::new();

        let broker_loop = task::spawn(async move {
            log::debug!("broker loop");
            while let Some(res) = rx.recv().await {
                log::debug!("broker received {:?}", res);
                match res {
                    BrokerMsg::Register { uuid, peer } => {
                        Self::clean_out_dead_peers(&mut loose_channels);
                        Self::register_peer(&mut loose_channels, &uuid, peer);
                    }

                    BrokerMsg::Connect { from, to } => {
                        Self::connect_peers(&mut loose_channels, &from, &to);
                    }
                }
            }
        });

        (Broker { to_broker: tx }, broker_loop)
    }
}
