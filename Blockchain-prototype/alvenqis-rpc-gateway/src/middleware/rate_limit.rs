use crate::error::RpcError;
use crate::middleware::auth::client_key;
use crate::state::RpcState;
use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct RateBucket {
    pub(crate) window_started_at: u64,
    pub(crate) write_count: u32,
    pub(crate) template_count: u32,
}

#[derive(Clone, Copy)]
pub enum WriteKind {
    Submit,
    MiningTemplate,
    MiningSubmit,
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn enforce_write_rate_limit(
    state: &RpcState,
    headers: &HeaderMap,
    peer: Option<&ConnectInfo<SocketAddr>>,
    kind: WriteKind,
) -> Result<(), RpcError> {
    let write_limit = state.config.write_rate_limit_per_minute;
    let template_limit = state.config.mining_template_rate_limit_per_minute;
    if write_limit == 0 && template_limit == 0 {
        return Ok(());
    }
    let key = client_key(headers, peer);
    let now = unix_seconds();
    let mut map = state
        .rate_limits
        .lock()
        .map_err(|_| RpcError::Config("rate limit lock poisoned".to_owned()))?;
    let bucket = map.entry(key).or_default();
    if now.saturating_sub(bucket.window_started_at) >= 60 {
        bucket.window_started_at = now;
        bucket.write_count = 0;
        bucket.template_count = 0;
    }
    match kind {
        WriteKind::Submit | WriteKind::MiningSubmit => {
            if write_limit > 0 {
                bucket.write_count = bucket.write_count.saturating_add(1);
                if bucket.write_count > write_limit {
                    return Err(RpcError::RateLimited(format!(
                        "write rate limit exceeded ({write_limit}/min)"
                    )));
                }
            }
        }
        WriteKind::MiningTemplate => {
            if template_limit > 0 {
                bucket.template_count = bucket.template_count.saturating_add(1);
                if bucket.template_count > template_limit {
                    return Err(RpcError::RateLimited(format!(
                        "mining template rate limit exceeded ({template_limit}/min)"
                    )));
                }
            }
            if write_limit > 0 {
                bucket.write_count = bucket.write_count.saturating_add(1);
                if bucket.write_count > write_limit {
                    return Err(RpcError::RateLimited(format!(
                        "write rate limit exceeded ({write_limit}/min)"
                    )));
                }
            }
        }
    }
    Ok(())
}
