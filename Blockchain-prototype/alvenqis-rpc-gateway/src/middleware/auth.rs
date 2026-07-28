use crate::error::RpcError;
use crate::state::RpcState;
use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use std::net::SocketAddr;

pub fn client_key(headers: &HeaderMap, peer: Option<&ConnectInfo<SocketAddr>>) -> String {
    if let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!("xff:{forwarded}");
    }
    if let Some(ConnectInfo(address)) = peer {
        return format!("ip:{}", address.ip());
    }
    "ip:unknown".to_owned()
}

pub fn require_write_auth(state: &RpcState, headers: &HeaderMap) -> Result<(), RpcError> {
    let Some(expected) = state.config.effective_api_token() else {
        return Ok(());
    };
    let provided = headers
        .get("x-alvenqis-api-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            headers
                .get(http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| {
                    let value = value.trim();
                    value
                        .strip_prefix("Bearer ")
                        .or_else(|| value.strip_prefix("bearer "))
                        .map(str::trim)
                        .map(str::to_owned)
                })
        });
    match provided {
        Some(token) if token == expected => Ok(()),
        _ => Err(RpcError::Unauthorized(
            "submit/mining requires Authorization: Bearer <token> or X-Alvenqis-Api-Token"
                .to_owned(),
        )),
    }
}
