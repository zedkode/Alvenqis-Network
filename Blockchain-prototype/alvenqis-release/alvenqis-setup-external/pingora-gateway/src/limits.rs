use crate::routes::{RatePolicy, RouteId};
use lru::LruCache;
use std::net::IpAddr;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct LimitKey {
    route: RouteId,
    client: IpAddr,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last_refill: Instant,
    in_flight: u32,
}

#[derive(Debug)]
pub enum LimitDecision {
    Allowed(RatePermit),
    RateLimited,
    ConcurrencyLimited,
}

#[derive(Debug)]
pub struct RateLimiter {
    state: Mutex<LruCache<LimitKey, Bucket>>,
}

impl RateLimiter {
    pub fn new(max_keys: usize) -> Arc<Self> {
        let capacity = NonZeroUsize::new(max_keys).expect("validated nonzero limiter capacity");
        Arc::new(Self {
            state: Mutex::new(LruCache::new(capacity)),
        })
    }

    pub fn check(
        self: &Arc<Self>,
        route: RouteId,
        client: IpAddr,
        policy: RatePolicy,
    ) -> LimitDecision {
        let key = LimitKey { route, client };
        let now = Instant::now();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.contains(&key) {
            state.put(
                key.clone(),
                Bucket {
                    tokens: f64::from(policy.burst),
                    last_refill: now,
                    in_flight: 0,
                },
            );
        }
        let bucket = state.get_mut(&key).expect("bucket inserted above");
        let elapsed = now
            .saturating_duration_since(bucket.last_refill)
            .as_secs_f64();
        bucket.tokens =
            (bucket.tokens + elapsed * policy.requests_per_second).min(f64::from(policy.burst));
        bucket.last_refill = now;

        if bucket.in_flight >= policy.concurrent {
            return LimitDecision::ConcurrencyLimited;
        }
        if bucket.tokens < 1.0 {
            return LimitDecision::RateLimited;
        }
        bucket.tokens -= 1.0;
        bucket.in_flight += 1;
        LimitDecision::Allowed(RatePermit {
            limiter: Arc::clone(self),
            key: Some(key),
        })
    }
}

#[derive(Debug)]
pub struct RatePermit {
    limiter: Arc<RateLimiter>,
    key: Option<LimitKey>,
}

impl Drop for RatePermit {
    fn drop(&mut self) {
        let Some(key) = self.key.take() else {
            return;
        };
        let mut state = self
            .limiter
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(bucket) = state.get_mut(&key) {
            bucket.in_flight = bucket.in_flight.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_concurrency_and_releases_permit_on_drop() {
        let limiter = RateLimiter::new(16);
        let policy = RatePolicy {
            requests_per_second: 100.0,
            burst: 2,
            concurrent: 1,
        };
        let client = "127.0.0.1".parse().unwrap();
        let first = match limiter.check(RouteId::FleetEnroll, client, policy) {
            LimitDecision::Allowed(permit) => permit,
            decision => panic!("unexpected decision: {decision:?}"),
        };
        assert!(matches!(
            limiter.check(RouteId::FleetEnroll, client, policy),
            LimitDecision::ConcurrencyLimited
        ));
        drop(first);
        assert!(matches!(
            limiter.check(RouteId::FleetEnroll, client, policy),
            LimitDecision::Allowed(_)
        ));
    }

    #[test]
    fn bounds_random_source_key_memory_with_lru_capacity() {
        let limiter = RateLimiter::new(2);
        let policy = RatePolicy {
            requests_per_second: 1.0,
            burst: 1,
            concurrent: 1,
        };
        for address in ["192.0.2.1", "192.0.2.2", "192.0.2.3"] {
            let decision = limiter.check(RouteId::FleetEnroll, address.parse().unwrap(), policy);
            drop(decision);
        }
        assert_eq!(limiter.state.lock().unwrap().len(), 2);
    }
}
