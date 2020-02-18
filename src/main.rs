#![allow(unused_imports)]
use std::{collections::BTreeMap, env, error::Error, sync::Arc};

use env_logger::Env;
use futures::{sink::SinkExt, stream::SplitSink, FutureExt, StreamExt};
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

const LOG_VAR: &str = "CASTME_LOG";

#[derive(Default)]
struct InnerState {}

// type State = Arc<Mutex<InnerState>>;

type Sender<T> = mpsc::UnboundedSender<T>;
type PeerSender = mpsc::UnboundedSender<PeerMessage>;

// Peer -> Broker Messages
#[derive(Debug)]
enum BrokerMsg {
    Register { uuid: Uuid, peer: PeerSender },
    Connect { from: Uuid, to: Uuid },
}

// P2P / Broker -> Peer Messages
#[derive(Debug)]
enum PeerMessage {
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
    fn create() -> (Broker, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::unbounded_channel();

        // only those that don't have a parnter yet
        let mut loose_channels: BTreeMap<Uuid, PeerSender> = BTreeMap::new();

        let broker_loop = task::spawn(async move {
            debug!("broker loop");
            while let Some(res) = rx.next().await {
                debug!("broker received {:?}", res);
                match res {
                    BrokerMsg::Register { uuid, peer } => {
                        if let Some(_peer) = loose_channels.insert(uuid, peer) {
                            warn!("uuid collision {}", uuid);
                        }
                        info!(
                            "registered peer under {} {:#?}",
                            uuid,
                            loose_channels.keys()
                        );
                    }

                    BrokerMsg::Connect { from, to } => {
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
                }
            }
        });

        (Broker { to_broker: tx }, broker_loop)
    }
}

async fn peer_connected(ws: WebSocket, broker: Broker) {
    debug!("user connected{:#?}", ws);

    let (mut socket_tx, mut socket_rx) = ws.split();
    let (broker_tx, mut broker_rx) = mpsc::unbounded_channel::<PeerMessage>();

    let my_uuid = Uuid::new_v4();

    // register at broker
    broker
        .to_broker
        .send(BrokerMsg::Register {
            uuid: my_uuid,
            peer: broker_tx.clone(),
        })
        .unwrap();

    
    // send uuid to user
    if let Err(e) = socket_tx
        .send(Message::text(
            serde_json::to_string(&Protocol::Welcome(my_uuid)).unwrap(),
        ))
        .await
    {
        warn!("failed to send on websocket {:?}", e);
    }

    // wait for correspondent
    let mut correspondent: Option<PeerSender> = None;

    loop {
        tokio::select! {
            received = socket_rx.next() => {
                if let Some(received) = received {
                    debug!("received on ws {:?}", received);
                    if let Ok(raw_content) = received {

                        match (&mut correspondent, raw_content.to_str()) {
                            (None, Ok(_)) => {
                                if let Ok(Protocol::Connect(uuid)) = raw_content.to_str().and_then(|s|serde_json::from_str(&s).map_err(|_|())) {
                                    debug!("connecting to {}", uuid);
                                    broker
                                        .to_broker
                                        .send(BrokerMsg::Connect {
                                            from: my_uuid,
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
            Some(received) = broker_rx.next() => {
                debug!("peer received from broker {:?}", received);
                match (received, &mut correspondent) {


                    // from the correspondent, send on socket
                    (PeerMessage::P2P( ref content ), _) => {
                        if let Err (e) = socket_tx.send(Message::text(content)).await {
                            warn!("failed to send on websocket {:?}", e);
                        }
                    }

                    (PeerMessage::Connected(_), Some(_)) => {
                        warn!("already have a correspondent");
                    }

                    (PeerMessage::Connected(other_peer), None) => {
                        correspondent.replace(other_peer);
                        info!("set a correspondent");
                    }

                }
            }
        }
    }

    info!("peer quit {}", my_uuid);
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
