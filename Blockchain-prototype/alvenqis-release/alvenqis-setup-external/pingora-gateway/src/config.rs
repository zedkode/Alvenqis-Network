use crate::auth::AdminAuthenticator;
use http::uri::Authority;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;
use std::time::Duration;

const DEFAULT_SECRET_ROOT: &str = "/run/secrets";
const DEFAULT_PKI_ROOT: &str = "/run/alvenqis-pki";

#[derive(Clone, Debug)]
pub struct HostConfig {
    pub control: String,
    pub rpc: String,
    pub fleet: String,
    pub fleet_mtls: String,
    pub grafana: String,
    pub prometheus: String,
    pub pool: String,
    pub website: String,
    pub www: String,
    pub explorer: String,
}

#[derive(Clone, Debug)]
pub struct PkiConfig {
    pub ca_certificate: PathBuf,
    pub server_certificate: PathBuf,
    pub server_private_key: PathBuf,
    pub forbidden_ca_private_key: PathBuf,
}

#[derive(Clone)]
pub struct GatewayConfig {
    pub hosts: HostConfig,
    pub admin: AdminAuthenticator,
    pub control_proxy_token: String,
    pub http_bind: SocketAddr,
    pub mtls_bind: SocketAddr,
    pub metrics_bind: SocketAddr,
    pub pki: PkiConfig,
    pub dns_refresh: Duration,
    pub limiter_max_keys: usize,
    pub connection_rate_per_second: u32,
    pub connection_burst: u32,
}

impl fmt::Debug for GatewayConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GatewayConfig")
            .field("hosts", &self.hosts)
            .field("admin", &self.admin)
            .field("control_proxy_token", &"[REDACTED]")
            .field("http_bind", &self.http_bind)
            .field("mtls_bind", &self.mtls_bind)
            .field("metrics_bind", &self.metrics_bind)
            .field("pki", &self.pki)
            .field("dns_refresh", &self.dns_refresh)
            .field("limiter_max_keys", &self.limiter_max_keys)
            .field(
                "connection_rate_per_second",
                &self.connection_rate_per_second,
            )
            .field("connection_burst", &self.connection_burst)
            .finish()
    }
}

impl GatewayConfig {
    pub fn load() -> Result<Self, String> {
        Self::load_from(
            |name| env::var(name).ok(),
            |path| fs::read(path).map_err(|error| error.to_string()),
        )
    }

    pub fn load_from<E, R>(env_value: E, read_file: R) -> Result<Self, String>
    where
        E: Fn(&str) -> Option<String>,
        R: Fn(&Path) -> Result<Vec<u8>, String>,
    {
        let base_domain = env_value("BASE_DOMAIN");
        let fleet_mtls_fallback = base_domain.map(|domain| format!("fleet-mtls.{domain}"));
        let hosts = HostConfig {
            control: required_host(&env_value, "CONTROL_HOST", None)?,
            rpc: required_host(&env_value, "RPC_HOST", None)?,
            fleet: required_host(&env_value, "FLEET_HOST", None)?,
            fleet_mtls: required_host(&env_value, "FLEET_MTLS_HOST", fleet_mtls_fallback)?,
            grafana: required_host(&env_value, "GRAFANA_HOST", None)?,
            prometheus: required_host(&env_value, "PROMETHEUS_HOST", None)?,
            pool: required_host(&env_value, "POOL_HOST", None)?,
            website: required_host(&env_value, "WEBSITE_HOST", None)?,
            www: required_host(&env_value, "WWW_HOST", None)?,
            explorer: required_host(&env_value, "EXPLORER_HOST", None)?,
        };
        validate_host_uniqueness(&hosts)?;

        let secret_root = PathBuf::from(
            env_value("GATEWAY_SECRET_ROOT").unwrap_or_else(|| DEFAULT_SECRET_ROOT.to_owned()),
        );
        let operator_username = env_value("ADMIN_OPERATOR_USER")
            .or_else(|| env_value("ADMIN_USER"))
            .ok_or_else(|| "ADMIN_OPERATOR_USER is required".to_owned())?;
        let viewer_username =
            env_value("ADMIN_VIEWER_USER").unwrap_or_else(|| "alvenqis-viewer".to_owned());
        let operator_password = read_secret(
            &read_file,
            &secret_root.join("admin_password"),
            "operator password",
        )?;
        let viewer_password = read_secret(
            &read_file,
            &secret_root.join("admin_viewer_password"),
            "viewer password",
        )?;
        let admin = AdminAuthenticator::new(
            viewer_username,
            &viewer_password,
            operator_username,
            &operator_password,
        )?;

        let proxy_token_bytes = read_secret(
            &read_file,
            &secret_root.join("control_proxy_token"),
            "control proxy token",
        )?;
        let control_proxy_token = String::from_utf8(proxy_token_bytes)
            .map_err(|_| "control proxy token must be UTF-8".to_owned())?;
        if control_proxy_token.len() != 64
            || !control_proxy_token
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(
                "control proxy token must contain exactly 64 hexadecimal characters".to_owned(),
            );
        }

        let pki_root = PathBuf::from(
            env_value("GATEWAY_PKI_ROOT").unwrap_or_else(|| DEFAULT_PKI_ROOT.to_owned()),
        );
        let pki = PkiConfig {
            ca_certificate: pki_root.join("fleet-ca.crt.pem"),
            server_certificate: pki_root.join("fleet-server.crt.pem"),
            server_private_key: pki_root.join("fleet-server.key.pem"),
            forbidden_ca_private_key: pki_root.join("fleet-ca.key.pem"),
        };

        let http_bind = parse_socket(
            env_value("GATEWAY_HTTP_BIND")
                .as_deref()
                .unwrap_or("0.0.0.0:8080"),
            "GATEWAY_HTTP_BIND",
        )?;
        let mtls_bind = parse_socket(
            env_value("GATEWAY_MTLS_BIND")
                .as_deref()
                .unwrap_or("0.0.0.0:10443"),
            "GATEWAY_MTLS_BIND",
        )?;
        let metrics_bind = parse_socket(
            env_value("GATEWAY_METRICS_BIND")
                .as_deref()
                .unwrap_or("0.0.0.0:9091"),
            "GATEWAY_METRICS_BIND",
        )?;
        if [http_bind.port(), mtls_bind.port(), metrics_bind.port()]
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != 3
        {
            return Err("gateway listener ports must be distinct".to_owned());
        }

        let dns_refresh = Duration::from_secs(parse_bounded_usize(
            env_value("GATEWAY_DNS_REFRESH_SECONDS").as_deref(),
            10,
            1,
            300,
            "GATEWAY_DNS_REFRESH_SECONDS",
        )? as u64);
        let limiter_max_keys = parse_bounded_usize(
            env_value("GATEWAY_RATE_LIMIT_MAX_KEYS").as_deref(),
            8_192,
            128,
            65_536,
            "GATEWAY_RATE_LIMIT_MAX_KEYS",
        )?;
        let connection_rate_per_second = parse_bounded_usize(
            env_value("GATEWAY_CONNECTION_RATE_PER_SECOND").as_deref(),
            200,
            10,
            10_000,
            "GATEWAY_CONNECTION_RATE_PER_SECOND",
        )? as u32;
        let connection_burst = parse_bounded_usize(
            env_value("GATEWAY_CONNECTION_BURST").as_deref(),
            400,
            20,
            20_000,
            "GATEWAY_CONNECTION_BURST",
        )? as u32;

        Ok(Self {
            hosts,
            admin,
            control_proxy_token,
            http_bind,
            mtls_bind,
            metrics_bind,
            pki,
            dns_refresh,
            limiter_max_keys,
            connection_rate_per_second,
            connection_burst,
        })
    }
}

fn required_host<E>(env_value: &E, name: &str, fallback: Option<String>) -> Result<String, String>
where
    E: Fn(&str) -> Option<String>,
{
    let value = env_value(name)
        .or(fallback)
        .ok_or_else(|| format!("{name} is required"))?
        .to_ascii_lowercase();
    let authority = Authority::from_str(&value).map_err(|_| format!("{name} is invalid"))?;
    if authority.port_u16().is_some()
        || authority.host() != value
        || value.len() > 253
        || value.ends_with('.')
    {
        return Err(format!(
            "{name} must be a lowercase DNS hostname without a port"
        ));
    }
    Ok(value)
}

fn validate_host_uniqueness(hosts: &HostConfig) -> Result<(), String> {
    let values = [
        ("CONTROL_HOST", &hosts.control),
        ("RPC_HOST", &hosts.rpc),
        ("FLEET_HOST", &hosts.fleet),
        ("FLEET_MTLS_HOST", &hosts.fleet_mtls),
        ("GRAFANA_HOST", &hosts.grafana),
        ("PROMETHEUS_HOST", &hosts.prometheus),
        ("POOL_HOST", &hosts.pool),
        ("WEBSITE_HOST", &hosts.website),
        ("EXPLORER_HOST", &hosts.explorer),
    ];
    let mut seen = HashMap::new();
    for (name, value) in values {
        if let Some(previous) = seen.insert(value, name) {
            return Err(format!("{name} must be distinct from {previous}"));
        }
    }
    if hosts.www != hosts.website && seen.contains_key(&hosts.www) {
        return Err("WWW_HOST must be distinct or equal to WEBSITE_HOST".to_owned());
    }
    Ok(())
}

fn read_secret<R>(read_file: &R, path: &Path, label: &str) -> Result<Vec<u8>, String>
where
    R: Fn(&Path) -> Result<Vec<u8>, String>,
{
    let mut value = read_file(path).map_err(|error| format!("{label} is unavailable: {error}"))?;
    while value
        .last()
        .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
    {
        value.pop();
    }
    if value.is_empty() || value.len() > 256 {
        return Err(format!("{label} must contain 1-256 bytes"));
    }
    Ok(value)
}

fn parse_socket(value: &str, name: &str) -> Result<SocketAddr, String> {
    value
        .parse()
        .map_err(|_| format!("{name} must be an IP socket address"))
}

fn parse_bounded_usize(
    value: Option<&str>,
    default: usize,
    minimum: usize,
    maximum: usize,
    name: &str,
) -> Result<usize, String> {
    let parsed = value
        .map(str::parse)
        .transpose()
        .map_err(|_| format!("{name} must be an integer"))?
        .unwrap_or(default);
    if !(minimum..=maximum).contains(&parsed) {
        return Err(format!("{name} must be between {minimum} and {maximum}"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> HashMap<String, String> {
        HashMap::from([
            ("CONTROL_HOST".into(), "control.example.test".into()),
            ("RPC_HOST".into(), "rpc.example.test".into()),
            ("FLEET_HOST".into(), "fleet.example.test".into()),
            ("FLEET_MTLS_HOST".into(), "fleet-mtls.example.test".into()),
            ("GRAFANA_HOST".into(), "grafana.example.test".into()),
            ("PROMETHEUS_HOST".into(), "prometheus.example.test".into()),
            ("POOL_HOST".into(), "pool.example.test".into()),
            ("WEBSITE_HOST".into(), "www.example.test".into()),
            ("WWW_HOST".into(), "www.example.test".into()),
            ("EXPLORER_HOST".into(), "explorer.example.test".into()),
            ("ADMIN_OPERATOR_USER".into(), "operator".into()),
            ("ADMIN_VIEWER_USER".into(), "viewer".into()),
        ])
    }

    fn secret(path: &Path) -> Result<Vec<u8>, String> {
        match path.file_name().and_then(|name| name.to_str()) {
            Some("admin_password") => Ok(b"operator-secret\n".to_vec()),
            Some("admin_viewer_password") => Ok(b"viewer-secret\n".to_vec()),
            Some("control_proxy_token") => Ok(vec![b'a'; 64]),
            _ => Err("not found".into()),
        }
    }

    #[test]
    fn loads_valid_typed_configuration_without_retaining_passwords_in_debug() {
        let environment = environment();
        let config = GatewayConfig::load_from(|name| environment.get(name).cloned(), secret)
            .expect("valid fixture");

        assert_eq!(config.http_bind.port(), 8080);
        assert_eq!(config.mtls_bind.port(), 10443);
        assert_eq!(config.metrics_bind.port(), 9091);
        assert_eq!(config.connection_rate_per_second, 200);
        assert_eq!(config.connection_burst, 400);
        let debug = format!("{config:?}");
        assert!(!debug.contains("operator-secret"));
        assert!(!debug.contains(&"a".repeat(64)));
    }

    #[test]
    fn rejects_duplicate_trust_boundaries() {
        let mut environment = environment();
        environment.insert("RPC_HOST".into(), "control.example.test".into());
        let error = GatewayConfig::load_from(|name| environment.get(name).cloned(), secret)
            .expect_err("duplicate hosts must fail closed");
        assert!(error.contains("distinct"));
    }

    #[test]
    fn rejects_malformed_proxy_token() {
        let environment = environment();
        let error = GatewayConfig::load_from(
            |name| environment.get(name).cloned(),
            |path| {
                if path.ends_with("control_proxy_token") {
                    Ok(b"not-hex".to_vec())
                } else {
                    secret(path)
                }
            },
        )
        .expect_err("token must fail closed");
        assert!(error.contains("64 hexadecimal"));
    }

    #[test]
    fn rejects_unbounded_connection_admission_values() {
        let mut environment = environment();
        environment.insert("GATEWAY_CONNECTION_BURST".into(), "999999".into());
        let error = GatewayConfig::load_from(|name| environment.get(name).cloned(), secret)
            .expect_err("connection admission must be bounded");
        assert!(error.contains("GATEWAY_CONNECTION_BURST"));
    }
}
