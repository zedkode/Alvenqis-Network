use crate::config::HostConfig;
use http::{Method, StatusCode};

pub const RPC_BODY_LIMIT: usize = 1024 * 1024;
pub const FLEET_MTLS_BODY_LIMIT: usize = 512 * 1024;
pub const DEFAULT_BODY_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Upstream {
    Control,
    Ops,
    Rpc,
    Grafana,
    Prometheus,
    Pool,
    Website,
    Explorer,
}

impl Upstream {
    pub const fn dns_name(self) -> &'static str {
        match self {
            Self::Control => "alvenqis-control",
            Self::Ops => "alvenqis-ops",
            Self::Rpc => "alvenqis-rpc",
            Self::Grafana => "grafana",
            Self::Prometheus => "prometheus",
            Self::Pool => "alvenqis-pool",
            Self::Website => "alvenqis-website",
            Self::Explorer => "alvenqis-explorer",
        }
    }

    pub const fn port(self) -> u16 {
        match self {
            Self::Control => 10_788,
            Self::Ops | Self::Website | Self::Explorer => 8_080,
            Self::Rpc => 10_787,
            Self::Grafana => 3_000,
            Self::Prometheus => 9_090,
            Self::Pool => 30_787,
        }
    }

    pub const fn health_path(self) -> &'static str {
        match self {
            Self::Control | Self::Ops | Self::Rpc | Self::Pool => "/health",
            Self::Website | Self::Explorer => "/healthz",
            Self::Grafana => "/api/health",
            Self::Prometheus => "/-/healthy",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RouteId {
    GatewayHealth,
    Unknown,
    ControlUi,
    ControlOps,
    ControlApi,
    FleetStatus,
    FleetEnroll,
    FleetUpgradeRequired,
    FleetRoot,
    FleetMtlsReport,
    FleetMtlsRotate,
    RpcPoolHealth,
    RpcPool,
    RpcMiningDenied,
    RpcSubmit,
    Rpc,
    Grafana,
    Prometheus,
    Pool,
    Website,
    Explorer,
}

impl RouteId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GatewayHealth => "gateway_health",
            Self::Unknown => "unknown",
            Self::ControlUi => "control_ui",
            Self::ControlOps => "control_ops",
            Self::ControlApi => "control_api",
            Self::FleetStatus => "fleet_status",
            Self::FleetEnroll => "fleet_enroll",
            Self::FleetUpgradeRequired => "fleet_upgrade_required",
            Self::FleetRoot => "fleet_root",
            Self::FleetMtlsReport => "fleet_mtls_report",
            Self::FleetMtlsRotate => "fleet_mtls_rotate",
            Self::RpcPoolHealth => "rpc_pool_health",
            Self::RpcPool => "rpc_pool",
            Self::RpcMiningDenied => "rpc_mining_denied",
            Self::RpcSubmit => "rpc_submit",
            Self::Rpc => "rpc",
            Self::Grafana => "grafana",
            Self::Prometheus => "prometheus",
            Self::Pool => "pool",
            Self::Website => "website",
            Self::Explorer => "explorer",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthPolicy {
    None,
    Basic,
    BasicWithAdminIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RatePolicy {
    pub requests_per_second: f64,
    pub burst: u32,
    pub concurrent: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalResponse {
    pub status: StatusCode,
    pub content_type: &'static str,
    pub body: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutePlan {
    pub id: RouteId,
    pub upstream: Option<Upstream>,
    pub rewrite_path: Option<String>,
    pub auth: AuthPolicy,
    pub inject_proxy_token: bool,
    pub inject_client_certificate: bool,
    pub body_limit: Option<usize>,
    pub rate: Option<RatePolicy>,
    pub local: Option<LocalResponse>,
}

impl RoutePlan {
    fn proxy(id: RouteId, upstream: Upstream) -> Self {
        Self {
            id,
            upstream: Some(upstream),
            rewrite_path: None,
            auth: AuthPolicy::None,
            inject_proxy_token: false,
            inject_client_certificate: false,
            body_limit: Some(DEFAULT_BODY_LIMIT),
            rate: None,
            local: None,
        }
    }

    fn local(
        id: RouteId,
        status: StatusCode,
        content_type: &'static str,
        body: &'static str,
    ) -> Self {
        Self {
            id,
            upstream: None,
            rewrite_path: None,
            auth: AuthPolicy::None,
            inject_proxy_token: false,
            inject_client_certificate: false,
            body_limit: None,
            rate: None,
            local: Some(LocalResponse {
                status,
                content_type,
                body,
            }),
        }
    }
}

pub fn route(
    hosts: &HostConfig,
    listener_port: u16,
    mtls_listener_port: u16,
    method: &Method,
    host: &str,
    path: &str,
) -> RoutePlan {
    if listener_port == mtls_listener_port {
        return route_mtls(hosts, host, path);
    }
    route_http(hosts, method, host, path)
}

fn route_mtls(hosts: &HostConfig, host: &str, path: &str) -> RoutePlan {
    if host != hosts.fleet_mtls {
        return not_found();
    }
    match path {
        "/fleet/report" => {
            let mut plan = RoutePlan::proxy(RouteId::FleetMtlsReport, Upstream::Control);
            plan.inject_proxy_token = true;
            plan.inject_client_certificate = true;
            plan.body_limit = Some(FLEET_MTLS_BODY_LIMIT);
            plan.rate = Some(RatePolicy {
                requests_per_second: 5.0,
                burst: 20,
                concurrent: 20,
            });
            plan
        }
        "/fleet/certificate/rotate" => {
            let mut plan = RoutePlan::proxy(RouteId::FleetMtlsRotate, Upstream::Control);
            plan.inject_proxy_token = true;
            plan.inject_client_certificate = true;
            plan.body_limit = Some(FLEET_MTLS_BODY_LIMIT);
            plan.rate = Some(RatePolicy {
                requests_per_second: 5.0,
                burst: 5,
                concurrent: 20,
            });
            plan
        }
        _ => not_found(),
    }
}

fn route_http(hosts: &HostConfig, method: &Method, host: &str, path: &str) -> RoutePlan {
    if host == hosts.control {
        if let Some(rewritten) = strip_prefix(path, "/setup/") {
            let mut plan = RoutePlan::proxy(RouteId::ControlUi, Upstream::Ops);
            plan.auth = AuthPolicy::Basic;
            plan.rewrite_path = Some(rewritten);
            return plan;
        }
        if let Some(rewritten) = strip_prefix(path, "/ops/") {
            let mut plan = RoutePlan::proxy(RouteId::ControlOps, Upstream::Ops);
            plan.auth = AuthPolicy::Basic;
            plan.rewrite_path = Some(rewritten);
            return plan;
        }
        let mut plan = RoutePlan::proxy(RouteId::ControlApi, Upstream::Control);
        plan.auth = AuthPolicy::BasicWithAdminIdentity;
        plan.inject_proxy_token = true;
        return plan;
    }

    if host == hosts.fleet {
        return match path {
            "/fleet/status" => {
                let mut plan = RoutePlan::proxy(RouteId::FleetStatus, Upstream::Control);
                plan.rewrite_path = Some("/public/topology".to_owned());
                plan.inject_proxy_token = true;
                plan
            }
            "/fleet/enroll" => {
                let mut plan = RoutePlan::proxy(RouteId::FleetEnroll, Upstream::Control);
                plan.inject_proxy_token = true;
                plan.rate = Some(RatePolicy {
                    requests_per_second: 5.0,
                    burst: 5,
                    concurrent: 5,
                });
                plan
            }
            "/fleet/report" => RoutePlan::local(
                RouteId::FleetUpgradeRequired,
                StatusCode::UPGRADE_REQUIRED,
                "application/json",
                r#"{"error":"agent reports require the dedicated mTLS endpoint"}"#,
            ),
            "/fleet/certificate/rotate" => RoutePlan::local(
                RouteId::FleetUpgradeRequired,
                StatusCode::UPGRADE_REQUIRED,
                "application/json",
                r#"{"error":"certificate rotation requires the dedicated mTLS endpoint"}"#,
            ),
            _ => RoutePlan::local(
                RouteId::FleetRoot,
                StatusCode::OK,
                "text/plain; charset=utf-8",
                "Alvenqis fleet endpoint\n",
            ),
        };
    }

    if host == hosts.rpc {
        if path == "/pool" {
            let mut plan = RoutePlan::proxy(RouteId::RpcPoolHealth, Upstream::Pool);
            plan.rewrite_path = Some("/health".to_owned());
            plan.body_limit = Some(RPC_BODY_LIMIT);
            return plan;
        }
        if let Some(rewritten) = strip_prefix(path, "/pool/") {
            let mut plan = RoutePlan::proxy(RouteId::RpcPool, Upstream::Pool);
            plan.rewrite_path = Some(rewritten);
            plan.body_limit = Some(RPC_BODY_LIMIT);
            return plan;
        }
        if path == "/mining" || path.starts_with("/mining/") {
            return RoutePlan::local(
                RouteId::RpcMiningDenied,
                StatusCode::GONE,
                "application/json",
                r#"{"error":"public mining routes are unavailable"}"#,
            );
        }
        let route_id = if path == "/transactions" && method == Method::POST {
            RouteId::RpcSubmit
        } else {
            RouteId::Rpc
        };
        let mut plan = RoutePlan::proxy(route_id, Upstream::Rpc);
        plan.body_limit = Some(RPC_BODY_LIMIT);
        return plan;
    }

    if host == hosts.grafana {
        return RoutePlan::proxy(RouteId::Grafana, Upstream::Grafana);
    }
    if host == hosts.prometheus {
        let mut plan = RoutePlan::proxy(RouteId::Prometheus, Upstream::Prometheus);
        plan.auth = AuthPolicy::Basic;
        return plan;
    }
    if host == hosts.pool {
        return RoutePlan::proxy(RouteId::Pool, Upstream::Pool);
    }
    if host == hosts.website || host == hosts.www {
        return RoutePlan::proxy(RouteId::Website, Upstream::Website);
    }
    if host == hosts.explorer {
        return RoutePlan::proxy(RouteId::Explorer, Upstream::Explorer);
    }
    if path == "/gateway-health" {
        return RoutePlan::local(
            RouteId::GatewayHealth,
            StatusCode::OK,
            "application/json",
            r#"{"ok":true,"service":"alvenqis-pingora-gateway"}"#,
        );
    }
    not_found()
}

fn strip_prefix(path: &str, prefix: &str) -> Option<String> {
    path.strip_prefix(prefix).map(|suffix| format!("/{suffix}"))
}

fn not_found() -> RoutePlan {
    RoutePlan::local(
        RouteId::Unknown,
        StatusCode::NOT_FOUND,
        "application/json",
        r#"{"error":"not found"}"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hosts() -> HostConfig {
        HostConfig {
            control: "control.example.test".into(),
            rpc: "rpc.example.test".into(),
            fleet: "fleet.example.test".into(),
            fleet_mtls: "fleet-mtls.example.test".into(),
            grafana: "grafana.example.test".into(),
            prometheus: "prometheus.example.test".into(),
            pool: "pool.example.test".into(),
            website: "www.example.test".into(),
            www: "www.example.test".into(),
            explorer: "explorer.example.test".into(),
        }
    }

    #[test]
    fn public_rpc_mining_paths_are_always_gone() {
        for path in ["/mining", "/mining/template", "/mining/submit"] {
            let plan = route(&hosts(), 8080, 10_443, "rpc.example.test", path);
            assert_eq!(plan.id, RouteId::RpcMiningDenied);
            assert_eq!(plan.local.unwrap().status, StatusCode::GONE);
        }
    }

    #[test]
    fn control_routes_preserve_auth_and_role_boundaries() {
        let ui = route(
            &hosts(),
            8080,
            10_443,
            "control.example.test",
            "/setup/health",
        );
        assert_eq!(ui.auth, AuthPolicy::Basic);
        assert_eq!(ui.rewrite_path.as_deref(), Some("/health"));

        let api = route(&hosts(), 8080, 10_443, "control.example.test", "/nodes");
        assert_eq!(api.auth, AuthPolicy::BasicWithAdminIdentity);
        assert!(api.inject_proxy_token);
    }

    #[test]
    fn fleet_reports_require_the_direct_mtls_listener() {
        let public = route(
            &hosts(),
            8080,
            10_443,
            "fleet.example.test",
            "/fleet/report",
        );
        assert_eq!(public.local.unwrap().status, StatusCode::UPGRADE_REQUIRED);

        let direct = route(
            &hosts(),
            10_443,
            10_443,
            "fleet-mtls.example.test",
            "/fleet/report",
        );
        assert_eq!(direct.upstream, Some(Upstream::Control));
        assert!(direct.inject_client_certificate);
        assert_eq!(direct.body_limit, Some(FLEET_MTLS_BODY_LIMIT));

        let alternate_port = route(
            &hosts(),
            11_443,
            11_443,
            "fleet-mtls.example.test",
            "/fleet/report",
        );
        assert_eq!(alternate_port.id, RouteId::FleetMtlsReport);
    }

    #[test]
    fn unknown_hosts_never_receive_a_default_upstream() {
        let plan = route(&hosts(), 8080, 10_443, "attacker.example", "/");
        assert_eq!(plan.id, RouteId::Unknown);
        assert!(plan.upstream.is_none());
        assert_eq!(plan.local.unwrap().status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn every_proxied_route_has_a_bounded_request_body() {
        let cases = [
            ("control.example.test", "/"),
            ("control.example.test", "/setup/health"),
            ("fleet.example.test", "/fleet/enroll"),
            ("rpc.example.test", "/status"),
            ("grafana.example.test", "/"),
            ("prometheus.example.test", "/"),
            ("pool.example.test", "/"),
            ("www.example.test", "/"),
            ("explorer.example.test", "/"),
        ];
        for (host, path) in cases {
            let plan = route(&hosts(), 8080, 10_443, host, path);
            assert!(plan.upstream.is_some(), "{host}{path} must be proxied");
            assert_eq!(plan.body_limit, Some(DEFAULT_BODY_LIMIT));
        }
    }
}
