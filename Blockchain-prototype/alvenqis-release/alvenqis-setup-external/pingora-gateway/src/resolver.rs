use crate::routes::Upstream;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::lookup_host;
use tokio::sync::Mutex;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DnsKey {
    host: String,
    port: u16,
}

#[derive(Debug)]
struct CacheEntry {
    addresses: Vec<SocketAddr>,
    expires_at: Instant,
    next: usize,
}

#[derive(Debug)]
pub struct DnsResolver {
    refresh: Duration,
    entries: Mutex<HashMap<DnsKey, CacheEntry>>,
}

impl DnsResolver {
    pub fn new(refresh: Duration) -> Arc<Self> {
        Arc::new(Self {
            refresh,
            entries: Mutex::new(HashMap::with_capacity(16)),
        })
    }

    pub async fn resolve_upstream(&self, upstream: Upstream) -> Result<SocketAddr, String> {
        self.resolve_one(upstream.dns_name(), upstream.port()).await
    }

    pub async fn invalidate_upstream(&self, upstream: Upstream) {
        let key = DnsKey {
            host: upstream.dns_name().to_owned(),
            port: upstream.port(),
        };
        if let Some(entry) = self.entries.lock().await.get_mut(&key) {
            entry.expires_at = Instant::now();
        }
    }

    pub async fn trusted_proxy_ip(&self, peer: IpAddr) -> bool {
        self.resolve_all("cloudflared", 8080)
            .await
            .is_ok_and(|addresses| addresses.iter().any(|address| address.ip() == peer))
    }

    async fn resolve_one(&self, host: &str, port: u16) -> Result<SocketAddr, String> {
        let key = DnsKey {
            host: host.to_owned(),
            port,
        };
        let addresses = self.resolve_all(host, port).await?;
        let mut entries = self.entries.lock().await;
        let entry = entries
            .get_mut(&key)
            .ok_or_else(|| format!("DNS cache entry disappeared for {host}"))?;
        let selected = addresses[entry.next % addresses.len()];
        entry.next = entry.next.wrapping_add(1);
        Ok(selected)
    }

    async fn resolve_all(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
        let key = DnsKey {
            host: host.to_owned(),
            port,
        };
        let now = Instant::now();
        {
            let entries = self.entries.lock().await;
            if let Some(entry) = entries.get(&key) {
                if now < entry.expires_at && !entry.addresses.is_empty() {
                    return Ok(entry.addresses.clone());
                }
            }
        }

        let resolved = lookup_host((host, port))
            .await
            .map(|addresses| {
                let mut addresses = addresses.collect::<Vec<_>>();
                addresses.sort_unstable();
                addresses.dedup();
                addresses
            })
            .map_err(|error| format!("DNS resolution failed for {host}: {error}"));

        let mut entries = self.entries.lock().await;
        match resolved {
            Ok(addresses) if !addresses.is_empty() => {
                entries.insert(
                    key,
                    CacheEntry {
                        addresses: addresses.clone(),
                        expires_at: now + self.refresh,
                        next: 0,
                    },
                );
                Ok(addresses)
            }
            Ok(_) | Err(_) => {
                if let Some(entry) = entries.get(&key) {
                    if !entry.addresses.is_empty() {
                        return Ok(entry.addresses.clone());
                    }
                }
                resolved.and_then(|_| Err(format!("DNS resolution returned no address for {host}")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolves_and_reuses_a_bounded_local_dns_entry() {
        let resolver = DnsResolver::new(Duration::from_secs(60));
        let first = resolver.resolve_one("localhost", 8080).await.unwrap();
        let second = resolver.resolve_one("localhost", 8080).await.unwrap();
        assert_eq!(first.port(), 8080);
        assert_eq!(second.port(), 8080);
        assert_eq!(resolver.entries.lock().await.len(), 1);
    }
}
