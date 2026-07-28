mod cache;
pub mod config;
pub mod error;
mod middleware;
pub mod models;
mod routes;
mod services;
pub mod state;

pub use cache::{load_chain, load_index_data, LoadedChain};
pub use config::{RpcAccessMode, RpcConfig};
pub use error::{RpcError, RpcResult};
pub use routes::router;
pub use state::RpcState;

pub mod app {
    pub use crate::cache::{load_chain, load_index_data, LoadedChain};
    pub use crate::routes::router;
    pub use crate::state::RpcState;
}
