use hannibal::{prelude::*, WeakAddr};

use crate::PeerId;

use super::Peer;

/// 1. both peers register themselves with the broker
#[message]
pub struct Register {
    pub id: PeerId,
    pub addr: WeakAddr<Peer>,
}

/// 2. the active peer requests to connect to another peer
#[message(response = Result<WeakAddr<Peer>, String>)]
pub struct RequestConnectTo {
    pub active: PeerId,
    pub passive: PeerId,
}

/// 3. the passive peer receives a notification that it is connected to the active peer
#[message]
pub struct ConnectedFrom {
    pub id: PeerId,
    pub addr: WeakAddr<Peer>,
}

#[message]
pub struct Forward(pub String);
