pub const PROTOCOL_VERSION: u32 = 1;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub protocol_version: u32,
    pub timestamp_ms: u64,
    pub payload: T,
}

impl<T> Envelope<T> {
    pub fn new(timestamp_ms: u64, payload: T) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            timestamp_ms,
            payload,
        }
    }
}
