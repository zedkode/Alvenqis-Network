use serde::Deserialize;
use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminConfig {
    pub bind_host: String,
    pub bind_port: u16,
    pub network_id: String,
    pub status_label: String,
    pub node_name: String,
    pub advertise_host: String,
    pub p2p_port: u16,
    pub local_rpc_url: String,
    pub state_dir: PathBuf,
    pub release_bundle_url: String,
    #[serde(default)]
    pub controller_url: Option<String>,
    #[serde(default)]
    pub public_rpc_url: Option<String>,
    #[serde(default = "default_report_interval")]
    pub report_interval_seconds: u64,
    #[serde(default = "default_invitation_ttl")]
    pub invitation_ttl_seconds: u64,
}

fn default_report_interval() -> u64 {
    15
}

fn default_invitation_ttl() -> u64 {
    900
}

impl AdminConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let config: Self =
            toml::from_str(&raw).map_err(|error| format!("invalid {}: {error}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        let docker_mode = env::var("ALVENQIS_DEPLOYMENT_MODE")
            .is_ok_and(|value| value.eq_ignore_ascii_case("docker"));
        self.validate_for_mode(docker_mode)
    }

    fn validate_for_mode(&self, docker_mode: bool) -> Result<(), String> {
        let bind_ip: IpAddr = self
            .bind_host
            .parse()
            .map_err(|_| "bind_host must be a literal loopback IP".to_owned())?;
        if !bind_ip.is_loopback() && !docker_mode {
            return Err("admin service must bind to a loopback address".to_owned());
        }
        if self.network_id != "alvenqis-mainnet-candidate" {
            return Err("network_id must be alvenqis-mainnet-candidate".to_owned());
        }
        if self.node_name.trim().is_empty() || self.advertise_host.trim().is_empty() {
            return Err("node_name and advertise_host are required".to_owned());
        }
        if !self.local_rpc_url.starts_with("http://127.0.0.1:")
            && !self.local_rpc_url.starts_with("http://[::1]:")
            && !(docker_mode && self.local_rpc_url.starts_with("http://alvenqis-rpc:"))
        {
            return Err(
                "local_rpc_url must use loopback HTTP or Docker-internal alvenqis-rpc".to_owned(),
            );
        }
        if let Some(url) = &self.controller_url {
            if !url.is_empty() && !url.starts_with("https://") {
                return Err("controller_url must use HTTPS".to_owned());
            }
            if url.contains('@') || url.chars().any(char::is_whitespace) {
                return Err("controller_url must not contain credentials or whitespace".to_owned());
            }
        }
        if let Some(url) = &self.public_rpc_url {
            if !url.is_empty() && !url.starts_with("https://") {
                return Err("public_rpc_url must use HTTPS".to_owned());
            }
            if url.contains('@') || url.chars().any(char::is_whitespace) {
                return Err("public_rpc_url must not contain credentials or whitespace".to_owned());
            }
        }
        if !self.release_bundle_url.starts_with("https://")
            || self.release_bundle_url.contains('@')
            || self.release_bundle_url.chars().any(char::is_whitespace)
        {
            return Err(
                "release_bundle_url must use HTTPS without embedded credentials".to_owned(),
            );
        }
        if !(5..=300).contains(&self.report_interval_seconds)
            || !(60..=86_400).contains(&self.invitation_ttl_seconds)
        {
            return Err("report interval or invitation lifetime is outside safe limits".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> AdminConfig {
        AdminConfig {
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 10788,
            network_id: "alvenqis-mainnet-candidate".to_owned(),
            status_label: "Mainnet Candidate / Prototype".to_owned(),
            node_name: "bootstrap-1".to_owned(),
            advertise_host: "node.example.org".to_owned(),
            p2p_port: 20787,
            local_rpc_url: "http://127.0.0.1:10787".to_owned(),
            state_dir: PathBuf::from("state"),
            release_bundle_url: "https://example.org/bundle.tar.gz".to_owned(),
            controller_url: None,
            public_rpc_url: None,
            report_interval_seconds: 15,
            invitation_ttl_seconds: 900,
        }
    }

    #[test]
    fn rejects_public_bind() {
        let mut config = valid();
        config.bind_host = "0.0.0.0".to_owned();
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_insecure_controller() {
        let mut config = valid();
        config.controller_url = Some("http://controller.example.org".to_owned());
        assert!(config.validate().is_err());
    }

    #[test]
    fn accepts_docker_internal_bind_and_rpc_only_in_docker_mode() {
        let mut config = valid();
        config.bind_host = "0.0.0.0".to_owned();
        config.local_rpc_url = "http://alvenqis-rpc:10787".to_owned();

        assert!(config.validate_for_mode(true).is_ok());
        assert!(config.validate_for_mode(false).is_err());
    }
}
