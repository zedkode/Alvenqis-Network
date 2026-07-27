use crate::error::{AppError, AppResult};
use crate::workspace::{find_workspace_root, local_root};
use regex::Regex;
use std::fs;

pub fn add_seed(seed: &str) -> AppResult<String> {
    let value = validate_seed(seed)?;
    let workspace = find_workspace_root()?;
    let config_path = local_root(&workspace).join("node.toml");
    let content = fs::read_to_string(&config_path).map_err(|_| {
        AppError::msg(
            "Runtime node configuration is unavailable. Start the local stack once before adding a seed.",
        )
    })?;
    let re = Regex::new(r#"(?m)^seed_nodes\s*=\s*\[[^\]]*\]\s*$"#)
        .map_err(|error| AppError::msg(format!("internal seed_nodes pattern error: {error}")))?;
    let seed_line = re
        .find(&content)
        .ok_or_else(|| AppError::msg("Runtime node configuration does not contain seed_nodes"))?
        .as_str();
    let entry_re = Regex::new(r#""([^"]+)""#)
        .map_err(|error| AppError::msg(format!("internal seed entry pattern error: {error}")))?;
    let mut seeds: Vec<String> = entry_re
        .captures_iter(seed_line)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();
    if seeds.iter().any(|s| s == &value) {
        return Ok(format!("Seed {value} is already configured."));
    }
    seeds.push(value.clone());
    let encoded = format!(
        "seed_nodes = [{}]",
        seeds
            .iter()
            .map(|entry| format!("\"{entry}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let updated = content.replacen(seed_line, &encoded, 1);
    let temporary = config_path.with_extension("toml.tmp");
    fs::write(&temporary, updated)?;
    fs::rename(temporary, config_path)?;
    Ok(format!("Seed {value} saved. Restart the node to connect."))
}

fn validate_seed(seed: &str) -> AppResult<String> {
    let value = seed.trim();
    let host_port = Regex::new(r"(?i)^(\[[0-9a-f:]+\]|[a-z0-9.-]+):([1-9]\d{0,4})$")
        .map_err(|error| AppError::msg(format!("internal host:port pattern error: {error}")))?;
    let multiaddr = Regex::new(
        r#"(?i)^/(?:ip4|ip6|dns4|dns6)/[^\s"']+/tcp/([1-9]\d{0,4})(?:/p2p/[A-Za-z0-9]+)?$"#,
    )
    .map_err(|error| AppError::msg(format!("internal multiaddr pattern error: {error}")))?;
    let host_port_match = host_port.captures(value);
    let multiaddr_match = multiaddr.captures(value);
    let port = host_port_match
        .as_ref()
        .and_then(|c| c.get(2))
        .or_else(|| multiaddr_match.as_ref().and_then(|c| c.get(1)))
        .and_then(|m| m.as_str().parse::<u32>().ok())
        .unwrap_or(0);
    if (host_port_match.is_none() && multiaddr_match.is_none()) || port > 65535 {
        return Err(AppError::msg(
            "Seed must be host:port or a TCP multiaddress, for example 192.168.1.20:20787",
        ));
    }
    let canonical = if let Some(cap) = host_port_match {
        let host = cap
            .get(1)
            .map(|m| m.as_str().trim_matches(|c| c == '[' || c == ']'))
            .unwrap_or_default();
        let port = cap.get(2).map(|m| m.as_str()).unwrap_or("20787");
        if is_ipv4(host) {
            format!("/ip4/{host}/tcp/{port}")
        } else if host.contains(':') {
            format!("/ip6/{host}/tcp/{port}")
        } else {
            format!("/dns4/{host}/tcp/{port}")
        }
    } else {
        let mut v = value.to_string();
        if let Ok(ipv4_dns) = Regex::new(r"^/dns4/(\d{1,3}(?:\.\d{1,3}){3})/") {
            if let Some(cap) = ipv4_dns.captures(&v) {
                v = v.replacen(
                    &format!("/dns4/{}/", &cap[1]),
                    &format!("/ip4/{}/", &cap[1]),
                    1,
                );
            }
        }
        if let Ok(ipv6_dns) = Regex::new(r"(?i)^/dns6/([0-9a-f:]+)/") {
            if let Some(cap) = ipv6_dns.captures(&v) {
                v = v.replacen(
                    &format!("/dns6/{}/", &cap[1]),
                    &format!("/ip6/{}/", &cap[1]),
                    1,
                );
            }
        }
        v
    };
    if canonical == "/ip4/127.0.0.1/tcp/20787" || canonical == "/ip6/::1/tcp/20787" {
        return Err(AppError::msg(
            "127.0.0.1:20787 is this node's own P2P endpoint, not a bootstrap peer. Use the reachable address of another Alvenqis node.",
        ));
    }
    Ok(canonical)
}

fn is_ipv4(host: &str) -> bool {
    let octets: Vec<&str> = host.split('.').collect();
    octets.len() == 4
        && octets.iter().all(|octet| {
            octet.parse::<u32>().map(|n| n <= 255).unwrap_or(false) && !octet.is_empty()
        })
}
