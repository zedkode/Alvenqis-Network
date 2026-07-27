use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const FAILURE_THRESHOLD: u32 = 3;
const BASE_OPEN_SECONDS: u64 = 5;
const MAX_OPEN_SECONDS: u64 = 60;

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionAvailability {
    pub status: String,
    pub circuit: String,
    pub endpoint: String,
    pub checked_at_unix_seconds: u64,
    pub last_success_at_unix_seconds: Option<u64>,
    pub next_retry_at_unix_seconds: Option<u64>,
    pub consecutive_failures: u32,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

impl ConnectionAvailability {
    pub fn idle(endpoint: impl Into<String>) -> Self {
        Self {
            status: "idle".into(),
            circuit: "closed".into(),
            endpoint: endpoint.into(),
            checked_at_unix_seconds: unix_now(),
            last_success_at_unix_seconds: None,
            next_retry_at_unix_seconds: None,
            consecutive_failures: 0,
            latency_ms: None,
            error: None,
        }
    }

    pub fn unavailable(endpoint: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            status: "offline".into(),
            circuit: "closed".into(),
            endpoint: endpoint.into(),
            checked_at_unix_seconds: unix_now(),
            last_success_at_unix_seconds: None,
            next_retry_at_unix_seconds: None,
            consecutive_failures: 0,
            latency_ms: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitPhase {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitPhase {
    fn label(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::HalfOpen => "half_open",
        }
    }
}

#[derive(Debug, Clone)]
struct Circuit {
    endpoint: String,
    phase: CircuitPhase,
    consecutive_failures: u32,
    last_success_at: Option<u64>,
    next_retry_at: Option<u64>,
    probe_in_flight: bool,
}

impl Circuit {
    fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.into(),
            phase: CircuitPhase::Closed,
            consecutive_failures: 0,
            last_success_at: None,
            next_retry_at: None,
            probe_in_flight: false,
        }
    }

    fn availability(
        &self,
        status: &str,
        latency_ms: Option<u64>,
        error: Option<String>,
    ) -> ConnectionAvailability {
        ConnectionAvailability {
            status: status.into(),
            circuit: self.phase.label().into(),
            endpoint: self.endpoint.clone(),
            checked_at_unix_seconds: unix_now(),
            last_success_at_unix_seconds: self.last_success_at,
            next_retry_at_unix_seconds: self.next_retry_at,
            consecutive_failures: self.consecutive_failures,
            latency_ms,
            error,
        }
    }
}

fn circuits() -> &'static Mutex<HashMap<String, Circuit>> {
    static CIRCUITS: OnceLock<Mutex<HashMap<String, Circuit>>> = OnceLock::new();
    CIRCUITS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn begin(key: &str, endpoint: &str) -> Result<(), Box<ConnectionAvailability>> {
    begin_at(key, endpoint, unix_now())
}

fn begin_at(key: &str, endpoint: &str, now: u64) -> Result<(), Box<ConnectionAvailability>> {
    let mut states = circuits().lock();
    let circuit = states
        .entry(key.into())
        .or_insert_with(|| Circuit::new(endpoint));
    circuit.endpoint = endpoint.into();

    if circuit.phase == CircuitPhase::Open {
        if circuit
            .next_retry_at
            .is_some_and(|retry_at| now >= retry_at)
        {
            circuit.phase = CircuitPhase::HalfOpen;
            circuit.probe_in_flight = true;
            return Ok(());
        }
        let mut availability = circuit.availability(
            "offline",
            None,
            Some("Circuit is open after repeated connection failures.".into()),
        );
        availability.checked_at_unix_seconds = now;
        return Err(Box::new(availability));
    }

    if circuit.phase == CircuitPhase::HalfOpen && circuit.probe_in_flight {
        let mut availability = circuit.availability(
            "offline",
            None,
            Some("Circuit recovery probe is already in progress.".into()),
        );
        availability.checked_at_unix_seconds = now;
        return Err(Box::new(availability));
    }

    if circuit.phase == CircuitPhase::HalfOpen {
        circuit.probe_in_flight = true;
    }
    Ok(())
}

pub fn success(key: &str, endpoint: &str, latency_ms: u64) -> ConnectionAvailability {
    success_at(key, endpoint, latency_ms, unix_now())
}

fn success_at(key: &str, endpoint: &str, latency_ms: u64, now: u64) -> ConnectionAvailability {
    let mut states = circuits().lock();
    let circuit = states
        .entry(key.into())
        .or_insert_with(|| Circuit::new(endpoint));
    circuit.endpoint = endpoint.into();
    circuit.phase = CircuitPhase::Closed;
    circuit.consecutive_failures = 0;
    circuit.last_success_at = Some(now);
    circuit.next_retry_at = None;
    circuit.probe_in_flight = false;
    let mut availability = circuit.availability("online", Some(latency_ms), None);
    availability.checked_at_unix_seconds = now;
    availability
}

pub fn failure(
    key: &str,
    endpoint: &str,
    latency_ms: u64,
    error: impl Into<String>,
) -> ConnectionAvailability {
    failure_at(key, endpoint, latency_ms, error.into(), unix_now())
}

fn failure_at(
    key: &str,
    endpoint: &str,
    latency_ms: u64,
    error: String,
    now: u64,
) -> ConnectionAvailability {
    let mut states = circuits().lock();
    let circuit = states
        .entry(key.into())
        .or_insert_with(|| Circuit::new(endpoint));
    circuit.endpoint = endpoint.into();
    circuit.consecutive_failures = circuit.consecutive_failures.saturating_add(1);
    circuit.probe_in_flight = false;

    if circuit.phase == CircuitPhase::HalfOpen || circuit.consecutive_failures >= FAILURE_THRESHOLD
    {
        circuit.phase = CircuitPhase::Open;
        let exponent = circuit
            .consecutive_failures
            .saturating_sub(FAILURE_THRESHOLD)
            .min(4);
        let open_seconds = BASE_OPEN_SECONDS
            .saturating_mul(1u64 << exponent)
            .min(MAX_OPEN_SECONDS);
        circuit.next_retry_at = Some(now.saturating_add(open_seconds));
    } else {
        circuit.phase = CircuitPhase::Closed;
        circuit.next_retry_at = None;
    }

    let mut availability = circuit.availability("offline", Some(latency_ms), Some(error));
    availability.checked_at_unix_seconds = now;
    availability
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset(key: &str) {
        circuits().lock().remove(key);
    }

    #[test]
    fn opens_after_threshold_and_recovers_half_open() {
        let key = "test:rpc:open";
        reset(key);
        assert!(begin_at(key, "https://rpc/status", 100).is_ok());
        failure_at(key, "https://rpc/status", 10, "one".into(), 100);
        failure_at(key, "https://rpc/status", 10, "two".into(), 101);
        let third = failure_at(key, "https://rpc/status", 10, "three".into(), 102);
        assert_eq!(third.circuit, "open");
        assert_eq!(third.next_retry_at_unix_seconds, Some(107));

        let blocked = begin_at(key, "https://rpc/status", 106).unwrap_err();
        assert_eq!(blocked.circuit, "open");
        assert!(begin_at(key, "https://rpc/status", 107).is_ok());

        let recovered = success_at(key, "https://rpc/status", 4, 108);
        assert_eq!(recovered.circuit, "closed");
        assert_eq!(recovered.status, "online");
        assert_eq!(recovered.consecutive_failures, 0);
        reset(key);
    }

    #[test]
    fn failed_recovery_probe_reopens_with_longer_delay() {
        let key = "test:pool:reopen";
        reset(key);
        failure_at(key, "https://pool/status", 5, "one".into(), 10);
        failure_at(key, "https://pool/status", 5, "two".into(), 11);
        failure_at(key, "https://pool/status", 5, "three".into(), 12);
        assert!(begin_at(key, "https://pool/status", 17).is_ok());
        let reopened = failure_at(key, "https://pool/status", 5, "again".into(), 17);
        assert_eq!(reopened.circuit, "open");
        assert_eq!(reopened.next_retry_at_unix_seconds, Some(27));
        reset(key);
    }
}
