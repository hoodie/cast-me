#![allow(unused_imports)]
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

mod peer;
use crate::peer::Peer;

const LOG_VAR: &str = "CAST_ME_LOG";

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;
type PeerSender = mpsc::UnboundedSender<PeerMessage>;


// Peer -> Broker Messages
#[derive(Debug)]
pub enum BrokerMsg {
    Register { uuid: Uuid, peer: PeerSender },
    Connect { from: Uuid, to: Uuid },
}

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

#[derive(Clone)]
struct Broker {
    to_broker: Sender<BrokerMsg>,
}

impl Broker {
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

    fn create() -> (Broker, JoinHandle<()>) {
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

#[allow(clippy::cognitive_complexity)]
async fn peer_connected(ws: WebSocket, broker: Broker) {
    debug!("user connected{:#?}", ws);

    let mut peer = Peer::new(ws, broker.to_broker);
    peer.register_at_broker();
    peer.send_welcome().await;
    peer.start().await;
}

#[tokio::main]
async fn main() {
    color_backtrace::install();
    if env::var(LOG_VAR).is_err() {
        env::set_var(LOG_VAR, "cast_me=trace,warp=info");
    }
    env_logger::init_from_env(Env::new().filter(LOG_VAR));

    let (broker, broker_loop) = Broker::create();
    let broker = warp::any().map(move || broker.clone());

    let chat =
        warp::path("ws")
            .and(warp::ws())
            .and(broker)
            .map(|ws: warp::ws::Ws, broker: Broker| {
                ws.on_upgrade(move |socket| peer_connected(socket, broker))
            });

    let index = warp::path::end().map(|| warp::reply::html(include_str!("../index.html")));

    let routes = index.or(chat);

    let listen_on = std::net::SocketAddr::from(([127, 0, 0, 1], 3030));
    info!("listening on {}", listen_on);

    tokio::select! {
        _ = broker_loop => {},
        _ = warp::serve(routes).run(listen_on) => {},
    };
}
