use crate::auth::AdminIdentity;
use crate::config::GatewayConfig;
use crate::limits::{LimitDecision, RateLimiter, RatePermit};
use crate::metrics::GatewayMetrics;
use crate::resolver::DnsResolver;
use crate::routes::{route, AuthPolicy, RouteId, RoutePlan, Upstream};
use crate::tls::{client_certificate, ClientCertificateInfo};
use async_trait::async_trait;
use bytes::Bytes;
use http::header::{
    AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST, STRICT_TRANSPORT_SECURITY, WWW_AUTHENTICATE,
};
use http::{HeaderName, StatusCode, Uri};
use log::{info, warn};
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::modules::http::compression::ResponseCompressionBuilder;
use pingora::modules::http::HttpModules;
use pingora::proxy::{FailToProxy, ProxyHttp, Session};
use pingora::server::configuration::ServerConf;
use pingora::server::ShutdownWatch;
use pingora::services::background::BackgroundService;
use pingora::upstreams::peer::HttpPeer;
use pingora::{Error, ErrorSource, ErrorType, Result as PingoraResult};
use serde_json::json;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const UPSTREAM_IO_TIMEOUT: Duration = Duration::from_secs(90);
const HEALTH_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const HSTS_VALUE: &str = "max-age=31536000; includeSubDomains";
const PROTECTED_REQUEST_HEADERS: [&str; 12] = [
    "forwarded",
    "x-real-ip",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "cf-connecting-ip",
    "x-alvenqis-admin-authenticated",
    "x-alvenqis-admin-user",
    "x-alvenqis-admin-role",
    "x-alvenqis-client-verify",
    "x-alvenqis-client-fingerprint",
    "x-alvenqis-proxy-token",
];

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct RequestContext {
    started: Instant,
    request_id: u64,
    plan: Option<RoutePlan>,
    client_ip: IpAddr,
    forwarded_proto: &'static str,
    admin_identity: Option<AdminIdentity>,
    client_certificate: Option<ClientCertificateInfo>,
    body_received: usize,
    retry_count: u8,
    force_dns_refresh: bool,
    _rate_permit: Option<RatePermit>,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            request_id: REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            plan: None,
            client_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            forwarded_proto: "http",
            admin_identity: None,
            client_certificate: None,
            body_received: 0,
            retry_count: 0,
            force_dns_refresh: false,
            _rate_permit: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct HealthEntry {
    available: Option<bool>,
    consecutive_failures: u8,
}

#[derive(Debug, Default)]
pub struct HealthRegistry {
    entries: Mutex<HashMap<Upstream, HealthEntry>>,
}

impl HealthRegistry {
    fn record(&self, upstream: Upstream, healthy: bool) -> bool {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = entries.entry(upstream).or_default();
        if healthy {
            entry.available = Some(true);
            entry.consecutive_failures = 0;
        } else {
            entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
            if entry.consecutive_failures >= 3 {
                entry.available = Some(false);
            }
        }
        entry.available.unwrap_or(true)
    }

    fn is_available(&self, upstream: Upstream) -> bool {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&upstream)
            .and_then(|entry| entry.available)
            .unwrap_or(true)
    }
}

pub struct GatewayProxy {
    config: Arc<GatewayConfig>,
    resolver: Arc<DnsResolver>,
    limiter: Arc<RateLimiter>,
    metrics: GatewayMetrics,
    health: Arc<HealthRegistry>,
}

impl GatewayProxy {
    pub fn new(config: GatewayConfig) -> (Self, GatewayHealthChecker) {
        let config = Arc::new(config);
        let resolver = DnsResolver::new(config.dns_refresh);
        let health = Arc::new(HealthRegistry::default());
        let metrics = GatewayMetrics::global();
        let proxy = Self {
            limiter: RateLimiter::new(config.limiter_max_keys),
            config,
            resolver: Arc::clone(&resolver),
            metrics: metrics.clone(),
            health: Arc::clone(&health),
        };
        let checker = GatewayHealthChecker {
            resolver,
            health,
            metrics,
        };
        (proxy, checker)
    }

    pub fn server_configuration() -> ServerConf {
        ServerConf {
            threads: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
                .clamp(1, 8),
            listener_tasks_per_fd: 1,
            upstream_keepalive_pool_size: 128,
            max_retries: 1,
            grace_period_seconds: Some(1),
            graceful_shutdown_timeout_seconds: Some(25),
            pid_file: "/tmp/alvenqis-pingora.pid".to_owned(),
            upgrade_sock: "/tmp/alvenqis-pingora-upgrade.sock".to_owned(),
            ..ServerConf::default()
        }
    }

    async fn prepare_request(
        &self,
        session: &mut Session,
        ctx: &mut RequestContext,
    ) -> PingoraResult<bool> {
        let listener_port = session
            .server_addr()
            .and_then(|address| address.as_inet())
            .map(|address| address.port())
            .unwrap_or(self.config.http_bind.port());
        let host = match normalized_host(session.req_header().headers.get(HOST)) {
            Ok(host) => host,
            Err(()) => {
                ctx.plan = Some(error_plan(
                    RouteId::Unknown,
                    StatusCode::BAD_REQUEST,
                    "invalid host",
                ));
                self.metrics.rejection(RouteId::Unknown, "invalid_host");
                respond_json(
                    session,
                    StatusCode::BAD_REQUEST,
                    r#"{"error":"invalid host"}"#,
                    false,
                )
                .await?;
                return Ok(true);
            }
        };
        let path = session.req_header().uri.path();
        let plan = route(
            &self.config.hosts,
            listener_port,
            self.config.mtls_bind.port(),
            &host,
            path,
        );
        ctx.plan = Some(plan.clone());

        let peer_ip = session
            .client_addr()
            .and_then(|address| address.as_inet())
            .map(|address| address.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        let trusted_cloudflare = listener_port == self.config.http_bind.port()
            && self.resolver.trusted_proxy_ip(peer_ip).await;
        ctx.client_ip = if trusted_cloudflare {
            session
                .req_header()
                .headers
                .get("cf-connecting-ip")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse().ok())
                .unwrap_or(peer_ip)
        } else {
            peer_ip
        };
        let forwarded_tls = trusted_cloudflare
            && session
                .req_header()
                .headers
                .get("x-forwarded-proto")
                .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"https"));
        ctx.forwarded_proto = if listener_port == self.config.mtls_bind.port() || forwarded_tls {
            "https"
        } else {
            "http"
        };

        if matches!(
            plan.auth,
            AuthPolicy::Basic | AuthPolicy::BasicWithAdminIdentity
        ) {
            let identity = self
                .config
                .admin
                .authenticate(session.req_header().headers.get(AUTHORIZATION));
            let Some(identity) = identity else {
                self.metrics.rejection(plan.id, "authentication");
                respond_unauthorized(session, ctx.forwarded_proto == "https").await?;
                return Ok(true);
            };
            if plan.auth == AuthPolicy::BasicWithAdminIdentity {
                ctx.admin_identity = Some(identity);
            }
        }

        if plan.inject_client_certificate {
            let Some(certificate) = client_certificate(session).cloned() else {
                self.metrics.rejection(plan.id, "client_certificate");
                respond_json(
                    session,
                    StatusCode::UNAUTHORIZED,
                    r#"{"error":"verified agent client certificate required"}"#,
                    true,
                )
                .await?;
                return Ok(true);
            };
            if certificate.sni.as_deref() != Some(self.config.hosts.fleet_mtls.as_str()) {
                self.metrics.rejection(plan.id, "mtls_sni");
                respond_json(
                    session,
                    StatusCode::MISDIRECTED_REQUEST,
                    r#"{"error":"fleet TLS hostname mismatch"}"#,
                    true,
                )
                .await?;
                return Ok(true);
            }
            ctx.client_certificate = Some(certificate);
        }

        if let Some(limit) = plan.body_limit {
            if session
                .req_header()
                .headers
                .contains_key("transfer-encoding")
            {
                self.metrics.rejection(plan.id, "transfer_encoding");
                respond_json(
                    session,
                    StatusCode::LENGTH_REQUIRED,
                    r#"{"error":"a bounded content length is required"}"#,
                    ctx.forwarded_proto == "https",
                )
                .await?;
                return Ok(true);
            }
            if let Some(value) = session.req_header().headers.get(CONTENT_LENGTH) {
                let length = value
                    .to_str()
                    .ok()
                    .and_then(|value| value.parse::<usize>().ok());
                if length.is_none() {
                    self.metrics.rejection(plan.id, "content_length");
                    respond_json(
                        session,
                        StatusCode::BAD_REQUEST,
                        r#"{"error":"invalid content length"}"#,
                        ctx.forwarded_proto == "https",
                    )
                    .await?;
                    return Ok(true);
                }
                if length.is_some_and(|length| length > limit) {
                    self.metrics.rejection(plan.id, "body_size");
                    respond_json(
                        session,
                        StatusCode::PAYLOAD_TOO_LARGE,
                        r#"{"error":"request body too large"}"#,
                        ctx.forwarded_proto == "https",
                    )
                    .await?;
                    return Ok(true);
                }
            } else if !session.is_body_empty() {
                self.metrics.rejection(plan.id, "content_length_required");
                respond_json(
                    session,
                    StatusCode::LENGTH_REQUIRED,
                    r#"{"error":"a bounded content length is required"}"#,
                    ctx.forwarded_proto == "https",
                )
                .await?;
                return Ok(true);
            }
        }

        if let Some(policy) = plan.rate {
            match self.limiter.check(plan.id, ctx.client_ip, policy) {
                LimitDecision::Allowed(permit) => ctx._rate_permit = Some(permit),
                LimitDecision::RateLimited => {
                    self.metrics.rejection(plan.id, "rate_limit");
                    respond_rate_limited(session, ctx.forwarded_proto == "https").await?;
                    return Ok(true);
                }
                LimitDecision::ConcurrencyLimited => {
                    self.metrics.rejection(plan.id, "concurrency_limit");
                    respond_rate_limited(session, ctx.forwarded_proto == "https").await?;
                    return Ok(true);
                }
            }
        }

        if let Some(local) = &plan.local {
            respond_local(
                session,
                local.status,
                local.content_type,
                local.body,
                ctx.forwarded_proto == "https",
                None,
            )
            .await?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[async_trait]
impl ProxyHttp for GatewayProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::default()
    }

    fn init_downstream_modules(&self, modules: &mut HttpModules) {
        modules.add_module(ResponseCompressionBuilder::enable(5));
    }

    fn request_summary(&self, session: &Session, ctx: &Self::CTX) -> String {
        let route = ctx.plan.as_ref().map_or(RouteId::Unknown, |plan| plan.id);
        format!(
            "request_id={} route={} method={}",
            ctx.request_id,
            route.as_str(),
            session.req_header().method
        )
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<bool> {
        self.prepare_request(session, ctx).await
    }

    async fn request_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<()> {
        let Some(limit) = ctx.plan.as_ref().and_then(|plan| plan.body_limit) else {
            return Ok(());
        };
        let incoming = body.as_ref().map_or(0, Bytes::len);
        ctx.body_received = ctx.body_received.saturating_add(incoming);
        if ctx.body_received > limit {
            return Err(Error::explain(
                ErrorType::HTTPStatus(StatusCode::PAYLOAD_TOO_LARGE.as_u16()),
                "request body exceeded configured route limit",
            ));
        }
        Ok(())
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<Box<HttpPeer>> {
        let upstream = ctx
            .plan
            .as_ref()
            .and_then(|plan| plan.upstream)
            .ok_or_else(|| Error::explain(ErrorType::HTTPStatus(500), "route has no upstream"))?;
        if !self.health.is_available(upstream) {
            return Err(Error::explain(
                ErrorType::HTTPStatus(503),
                "upstream is unavailable",
            ));
        }
        if ctx.force_dns_refresh {
            self.resolver.invalidate_upstream(upstream).await;
            ctx.force_dns_refresh = false;
        }
        let address = self
            .resolver
            .resolve_upstream(upstream)
            .await
            .map_err(|error| Error::explain(ErrorType::ConnectRefused, error))?;
        let mut peer = HttpPeer::new(address, false, upstream.dns_name().to_owned());
        peer.group_key = upstream as u64;
        peer.options.connection_timeout = Some(CONNECT_TIMEOUT);
        peer.options.total_connection_timeout = Some(CONNECT_TIMEOUT);
        peer.options.read_timeout = Some(UPSTREAM_IO_TIMEOUT);
        peer.options.write_timeout = Some(UPSTREAM_IO_TIMEOUT);
        peer.options.idle_timeout = Some(Duration::from_secs(65));
        peer.options.set_http_version(1, 1);
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<()> {
        let plan = ctx
            .plan
            .as_ref()
            .ok_or_else(|| Error::explain(ErrorType::InternalError, "missing route plan"))?;
        for name in PROTECTED_REQUEST_HEADERS {
            request.remove_header(&HeaderName::from_static(name));
        }
        if plan.auth != AuthPolicy::None {
            request.remove_header(&AUTHORIZATION);
        }

        let original_host = normalized_host(session.req_header().headers.get(HOST))
            .map_err(|()| Error::explain(ErrorType::InvalidHTTPHeader, "invalid host"))?;
        request.insert_header(HOST, original_host.as_str())?;
        request.insert_header("x-real-ip", ctx.client_ip.to_string())?;
        request.insert_header("x-forwarded-for", ctx.client_ip.to_string())?;
        request.insert_header("x-forwarded-host", original_host)?;
        request.insert_header("x-forwarded-proto", ctx.forwarded_proto)?;

        if let Some(identity) = &ctx.admin_identity {
            request.insert_header("x-alvenqis-admin-authenticated", "1")?;
            request.insert_header("x-alvenqis-admin-user", identity.username.as_str())?;
            request.insert_header("x-alvenqis-admin-role", identity.role.as_str())?;
        }
        if plan.inject_proxy_token {
            request.insert_header(
                "x-alvenqis-proxy-token",
                self.config.control_proxy_token.as_str(),
            )?;
        }
        if let Some(certificate) = &ctx.client_certificate {
            request.insert_header("x-alvenqis-client-verify", "SUCCESS")?;
            request.insert_header(
                "x-alvenqis-client-fingerprint",
                certificate.fingerprint_sha1.as_str(),
            )?;
        }
        if let Some(rewritten) = &plan.rewrite_path {
            let rewritten = if let Some(query) = session.req_header().uri.query() {
                format!("{rewritten}?{query}")
            } else {
                rewritten.clone()
            };
            let uri = Uri::builder()
                .path_and_query(rewritten)
                .build()
                .map_err(|_| {
                    Error::explain(ErrorType::InvalidHTTPHeader, "invalid rewritten URI")
                })?;
            request.set_uri(uri);
        }
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<()> {
        apply_security_headers(response, ctx.forwarded_proto == "https")?;
        response.remove_header("server");
        response.remove_header("alt-svc");
        Ok(())
    }

    fn fail_to_connect(
        &self,
        session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut error: Box<Error>,
    ) -> Box<Error> {
        if let Some(upstream) = ctx.plan.as_ref().and_then(|plan| plan.upstream) {
            let available = self.health.record(upstream, false);
            self.metrics.upstream_health(upstream, available);
        }
        let idempotent = matches!(session.req_header().method.as_str(), "GET" | "HEAD");
        if idempotent && ctx.retry_count == 0 {
            ctx.retry_count = 1;
            ctx.force_dns_refresh = true;
            error.set_retry(true);
        } else {
            error.set_retry(false);
        }
        error
    }

    fn error_while_proxy(
        &self,
        _peer: &HttpPeer,
        _session: &mut Session,
        mut error: Box<Error>,
        _ctx: &mut Self::CTX,
        _client_reused: bool,
    ) -> Box<Error> {
        error.set_retry(false);
        error
    }

    async fn fail_to_proxy(
        &self,
        session: &mut Session,
        error: &Error,
        ctx: &mut Self::CTX,
    ) -> FailToProxy {
        let status = match error.etype() {
            ErrorType::HTTPStatus(status) => *status,
            _ if error.esource() == &ErrorSource::Downstream => 400,
            _ => 502,
        };
        if status > 0 {
            let body = if status == 503 {
                r#"{"error":"upstream temporarily unavailable"}"#
            } else if status == 413 {
                r#"{"error":"request body too large"}"#
            } else {
                r#"{"error":"gateway request failed"}"#
            };
            if let Err(write_error) = respond_json(
                session,
                StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
                body,
                ctx.forwarded_proto == "https",
            )
            .await
            {
                warn!("failed to write bounded gateway error response: {write_error}");
            }
        }
        FailToProxy {
            error_code: status,
            can_reuse_downstream: false,
        }
    }

    async fn logging(&self, session: &mut Session, error: Option<&Error>, ctx: &mut Self::CTX) {
        let route = ctx.plan.as_ref().map_or(RouteId::Unknown, |plan| plan.id);
        let status = session.response_written().map_or_else(
            || error.map_or(0, |_| 502),
            |response| response.status.as_u16(),
        );
        self.metrics.request(route, status, ctx.started.elapsed());
        let record = json!({
            "event": "gateway_request",
            "request_id": ctx.request_id,
            "route": route.as_str(),
            "method": session.req_header().method.as_str(),
            "status": status,
            "duration_ms": ctx.started.elapsed().as_millis(),
            "error": error.map(|value| format!("{:?}", value.etype())),
        });
        info!("{record}");
    }
}

pub struct GatewayHealthChecker {
    resolver: Arc<DnsResolver>,
    health: Arc<HealthRegistry>,
    metrics: GatewayMetrics,
}

#[async_trait]
impl BackgroundService for GatewayHealthChecker {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = shutdown.changed() => break,
                _ = interval.tick() => {
                    for upstream in ALL_UPSTREAMS {
                        if shutdown.has_changed().unwrap_or(true) {
                            return;
                        }
                        let healthy = probe_upstream(&self.resolver, upstream).await;
                        let available = self.health.record(upstream, healthy);
                        self.metrics.upstream_health(upstream, available);
                    }
                }
            }
        }
    }
}

const ALL_UPSTREAMS: [Upstream; 8] = [
    Upstream::Control,
    Upstream::Ops,
    Upstream::Rpc,
    Upstream::Grafana,
    Upstream::Prometheus,
    Upstream::Pool,
    Upstream::Website,
    Upstream::Explorer,
];

async fn probe_upstream(resolver: &DnsResolver, upstream: Upstream) -> bool {
    let address = match resolver.resolve_upstream(upstream).await {
        Ok(address) => address,
        Err(_) => return false,
    };
    let probe = async {
        let mut stream = TcpStream::connect(address).await?;
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            upstream.health_path(),
            upstream.dns_name()
        );
        stream.write_all(request.as_bytes()).await?;
        let mut response = [0_u8; 512];
        let read = stream.read(&mut response).await?;
        let first_line = std::str::from_utf8(&response[..read])
            .ok()
            .and_then(|response| response.lines().next())
            .unwrap_or_default();
        let status = first_line
            .split_ascii_whitespace()
            .nth(1)
            .and_then(|status| status.parse::<u16>().ok())
            .unwrap_or(0);
        Ok::<bool, std::io::Error>((200..400).contains(&status))
    };
    tokio::time::timeout(HEALTH_PROBE_TIMEOUT, probe)
        .await
        .is_ok_and(|result| result.unwrap_or(false))
}

fn normalized_host(value: Option<&http::HeaderValue>) -> Result<String, ()> {
    let value = value
        .ok_or(())?
        .to_str()
        .map_err(|_| ())?
        .to_ascii_lowercase();
    let authority = http::uri::Authority::from_str(&value).map_err(|_| ())?;
    let host = authority.host();
    if host.is_empty() || host.len() > 253 || host.ends_with('.') {
        return Err(());
    }
    Ok(host.to_owned())
}

fn error_plan(route: RouteId, status: StatusCode, message: &'static str) -> RoutePlan {
    RoutePlan {
        id: route,
        upstream: None,
        rewrite_path: None,
        auth: AuthPolicy::None,
        inject_proxy_token: false,
        inject_client_certificate: false,
        body_limit: None,
        rate: None,
        local: Some(crate::routes::LocalResponse {
            status,
            content_type: "application/json",
            body: message,
        }),
    }
}

async fn respond_unauthorized(session: &mut Session, tls: bool) -> PingoraResult<()> {
    respond_local(
        session,
        StatusCode::UNAUTHORIZED,
        "application/json",
        r#"{"error":"authentication required"}"#,
        tls,
        Some((
            WWW_AUTHENTICATE,
            "Basic realm=\"Alvenqis project operations\"",
        )),
    )
    .await
}

async fn respond_rate_limited(session: &mut Session, tls: bool) -> PingoraResult<()> {
    respond_local(
        session,
        StatusCode::TOO_MANY_REQUESTS,
        "application/json",
        r#"{"error":"request rate exceeded"}"#,
        tls,
        Some((HeaderName::from_static("retry-after"), "1")),
    )
    .await
}

async fn respond_json(
    session: &mut Session,
    status: StatusCode,
    body: &'static str,
    tls: bool,
) -> PingoraResult<()> {
    respond_local(session, status, "application/json", body, tls, None).await
}

async fn respond_local(
    session: &mut Session,
    status: StatusCode,
    content_type: &'static str,
    body: &'static str,
    tls: bool,
    extra: Option<(HeaderName, &'static str)>,
) -> PingoraResult<()> {
    let mut response = ResponseHeader::build(status.as_u16(), Some(10))?;
    response.insert_header(CONTENT_TYPE, content_type)?;
    response.insert_header(CONTENT_LENGTH, body.len().to_string())?;
    apply_security_headers(&mut response, tls)?;
    if let Some((name, value)) = extra {
        response.insert_header(name, value)?;
    }
    session
        .write_response_header(Box::new(response), body.is_empty())
        .await?;
    if !body.is_empty() {
        session
            .write_response_body(Some(Bytes::from_static(body.as_bytes())), true)
            .await?;
    }
    Ok(())
}

fn apply_security_headers(response: &mut ResponseHeader, tls: bool) -> PingoraResult<()> {
    response.insert_header("x-content-type-options", "nosniff")?;
    response.insert_header("x-frame-options", "SAMEORIGIN")?;
    response.insert_header("referrer-policy", "strict-origin-when-cross-origin")?;
    response.insert_header(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()",
    )?;
    if tls {
        response.insert_header(STRICT_TRANSPORT_SECURITY, HSTS_VALUE)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_host_and_strips_optional_port() {
        let header = http::HeaderValue::from_static("RPC.Example.Test:8080");
        assert_eq!(normalized_host(Some(&header)).unwrap(), "rpc.example.test");
    }

    #[test]
    fn rejects_host_with_trailing_dot_or_invalid_bytes() {
        let trailing = http::HeaderValue::from_static("rpc.example.test.");
        assert!(normalized_host(Some(&trailing)).is_err());
        assert!(normalized_host(None).is_err());
    }

    #[test]
    fn health_requires_three_consecutive_failures_before_unavailable() {
        let health = HealthRegistry::default();
        assert!(health.record(Upstream::Rpc, false));
        assert!(health.record(Upstream::Rpc, false));
        assert!(!health.record(Upstream::Rpc, false));
        assert!(health.record(Upstream::Rpc, true));
    }
}
