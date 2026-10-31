use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum RpcErrorKind {
    TimedOut,
    ConnectionClosed,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RpcRequest<P> {
    payload: P,
    id: u32,
}

impl<P> RpcRequest<P> {
    #[cfg(feature = "std")]
    pub fn new(payload: P) -> Self {
        Self {
            payload,
            id: rand::random::<u32>(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RpcResponse<P> {
    payload: P,
    id: u32,
}

impl<P> RpcResponse<P> {
    pub fn new<R>(request: &RpcRequest<R>, payload: P) -> Self {
        Self {
            payload,
            id: request.id,
        }
    }
}
