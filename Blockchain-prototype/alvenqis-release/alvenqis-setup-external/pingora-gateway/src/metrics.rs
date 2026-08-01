use crate::routes::{RouteId, Upstream};
use pingora_prometheus::prometheus::{
    self, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts,
};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone)]
pub struct GatewayMetrics {
    requests: IntCounterVec,
    rejections: IntCounterVec,
    latency: HistogramVec,
    upstream_health: IntGaugeVec,
}

impl GatewayMetrics {
    pub fn global() -> Self {
        static METRICS: OnceLock<GatewayMetrics> = OnceLock::new();
        METRICS
            .get_or_init(|| {
                let requests = IntCounterVec::new(
                    Opts::new(
                        "alvenqis_gateway_requests_total",
                        "Completed gateway requests",
                    ),
                    &["route", "status"],
                )
                .expect("static metric definition");
                let rejections = IntCounterVec::new(
                    Opts::new(
                        "alvenqis_gateway_rejections_total",
                        "Gateway policy rejections",
                    ),
                    &["route", "reason"],
                )
                .expect("static metric definition");
                let latency = HistogramVec::new(
                    HistogramOpts::new(
                        "alvenqis_gateway_request_duration_seconds",
                        "Gateway request duration",
                    ),
                    &["route"],
                )
                .expect("static metric definition");
                let upstream_health = IntGaugeVec::new(
                    Opts::new(
                        "alvenqis_gateway_upstream_health",
                        "Last active upstream health result (1 healthy, 0 unhealthy)",
                    ),
                    &["upstream"],
                )
                .expect("static metric definition");
                let registry = prometheus::default_registry();
                registry
                    .register(Box::new(requests.clone()))
                    .expect("register request metric once");
                registry
                    .register(Box::new(rejections.clone()))
                    .expect("register rejection metric once");
                registry
                    .register(Box::new(latency.clone()))
                    .expect("register latency metric once");
                registry
                    .register(Box::new(upstream_health.clone()))
                    .expect("register upstream metric once");
                GatewayMetrics {
                    requests,
                    rejections,
                    latency,
                    upstream_health,
                }
            })
            .clone()
    }

    pub fn request(&self, route: RouteId, status: u16, elapsed: Duration) {
        let status = status.to_string();
        self.requests
            .with_label_values(&[route.as_str(), &status])
            .inc();
        self.latency
            .with_label_values(&[route.as_str()])
            .observe(elapsed.as_secs_f64());
    }

    pub fn rejection(&self, route: RouteId, reason: &'static str) {
        self.rejections
            .with_label_values(&[route.as_str(), reason])
            .inc();
    }

    pub fn upstream_health(&self, upstream: Upstream, healthy: bool) {
        self.upstream_health
            .with_label_values(&[upstream.dns_name()])
            .set(i64::from(healthy));
    }
}
