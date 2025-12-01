use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::fmt;

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

impl Default for PeerId {
    fn default() -> PeerId {
        PeerId(human_hash::humanize(&Uuid::new_v4(), 2))
    }
}

impl Clone for PeerId {
    fn clone(&self) -> PeerId {
        PeerId(self.0.clone())
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
