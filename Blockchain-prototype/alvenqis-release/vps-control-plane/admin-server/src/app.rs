use crate::config::AdminConfig;
use crate::models::{
    AdminOverview, AuditEntry, BanRequest, BootstrapManifest, BootstrapManifestRequest,
    BootstrapPort, DeploymentRole, EnrollmentRequest, EnrollmentResponse, EnrollmentStep,
    FleetTopology, InvitationRequest, InvitationResponse, InvitationView, MutationResponse,
    NodeDetailView, NodeReport, ProbeResult, ReportRequest, ServiceInventoryItem, ServiceStates,
};
use crate::store::{ControlledMutation, FleetStore, MutationContext};
use axum::extract::{Path as PathParam, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use reqwest::Client;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_rustls::TlsConnector;

const INDEX_HTML: &str = include_str!("../static/index.html");
const APP_JS: &str = include_str!("../static/app.js");
const STYLES_CSS: &str = include_str!("../static/styles.css");
const LOGO_PNG: &[u8] = include_bytes!("../static/logo.png");
const LOGO_MARK_PNG: &[u8] = include_bytes!("../static/logo-mark.png");
const MAX_REPORT_BYTES: usize = 512 * 1024;

#[derive(Clone)]
pub struct AdminState {
    pub config: AdminConfig,
    pub store: FleetStore,
    client: Client,
}

impl AdminState {
    pub fn new(config: AdminConfig, store: FleetStore) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            config,
            store,
            client,
        })
    }
}

pub fn router(state: AdminState) -> Router {
    let protected = Router::new()
        .route("/api/overview", get(overview))
        .route("/api/health", get(control_health))
        .route("/api/status", get(control_status))
        .route("/api/topology", get(topology))
        .route("/api/services", get(service_inventory))
        .route("/api/nodes", get(topology).post(create_invitation))
        .route("/api/nodes/:node_id", get(node_detail).delete(remove_node))
        .route("/api/nodes/:node_id/ban", post(ban_node))
        .route("/api/nodes/:node_id/unban", post(unban_node))
        .route(
            "/api/invitations",
            get(list_invitations).post(create_invitation),
        )
        .route("/api/invitations/:invitation_id", delete(revoke_invitation))
        .route(
            "/api/bootstrap/manifests",
            post(generate_bootstrap_manifest),
        )
        .route("/api/bootstrap/roles", get(bootstrap_roles))
        .route("/api/audit", get(audit_log))
        .route("/api/fleet/summary", get(fleet_summary))
        .route_layer(middleware::from_fn(require_proxy_auth));

    Router::new()
        .route("/", get(index))
        .route("/app.js", get(javascript))
        .route("/styles.css", get(styles))
        .route("/logo.png", get(logo_png))
        .route("/logo-mark.png", get(logo_mark_png))
        .route("/health", get(health))
        .route("/public/topology", get(public_topology))
        .route("/fleet/enroll", post(enroll))
        .route("/fleet/report", post(report))
        .merge(protected)
        .with_state(state)
}

async fn require_proxy_auth(headers: HeaderMap, request: Request, next: Next) -> Response {
    if headers
        .get("x-alvenqis-admin-authenticated")
        .and_then(|value| value.to_str().ok())
        == Some("1")
    {
        return next.run(request).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "admin reverse-proxy authentication required"})),
    )
        .into_response()
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn javascript() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        APP_JS,
    )
}

async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES_CSS,
    )
}

async fn logo_png() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], LOGO_PNG)
}

async fn logo_mark_png() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], LOGO_MARK_PNG)
}

async fn health(State(state): State<AdminState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "kind": "liveness",
        "service": "alvenqis-vps-admin",
        "network_id": state.config.network_id,
        "status_label": state.config.status_label,
        "exposure": "loopback-only; authenticate at reverse proxy"
    }))
}

async fn control_health(State(state): State<AdminState>) -> Json<Value> {
    let local = collect_local_report(&state).await;
    let failing: Vec<&String> = local
        .probes
        .iter()
        .filter_map(|(name, probe)| (probe.configured && !probe.healthy).then_some(name))
        .collect();
    Json(json!({
        "ok": failing.is_empty(),
        "kind": "dependency-readiness",
        "checked_at_unix_seconds": local.reported_at_unix_seconds,
        "failing_probes": failing,
        "probes": local.probes,
    }))
}

async fn control_status(State(state): State<AdminState>) -> Json<Value> {
    let local = collect_local_report(&state).await;
    Json(json!({
        "network_id": local.network_id,
        "node_name": local.node_name,
        "p2p": local.p2p,
        "stratum": local.stratum,
        "indexer": local.indexer,
        "sync": local.sync,
        "services": local.services,
        "probes": local.probes,
        "reported_at_unix_seconds": local.reported_at_unix_seconds,
    }))
}

async fn overview(
    State(state): State<AdminState>,
) -> Result<Json<AdminOverview>, (StatusCode, Json<Value>)> {
    let local = collect_local_report(&state).await;
    let topology = build_topology(&state, Some(local.clone())).map_err(internal)?;
    Ok(Json(AdminOverview {
        mode: "Mainnet Candidate / observed VPS fleet",
        status_label: state.config.status_label.clone(),
        local,
        topology,
    }))
}

async fn topology(
    State(state): State<AdminState>,
) -> Result<Json<FleetTopology>, (StatusCode, Json<Value>)> {
    let local = collect_local_report(&state).await;
    Ok(Json(build_topology(&state, Some(local)).map_err(internal)?))
}

async fn public_topology(
    State(state): State<AdminState>,
) -> Result<Json<FleetTopology>, (StatusCode, Json<Value>)> {
    let local = collect_local_report(&state).await;
    Ok(Json(build_topology(&state, Some(local)).map_err(internal)?))
}

async fn service_inventory(
    State(state): State<AdminState>,
) -> Result<Json<Vec<ServiceInventoryItem>>, (StatusCode, Json<Value>)> {
    let local = collect_local_report(&state).await;
    let topology = build_topology(&state, Some(local)).map_err(internal)?;
    let mut services = Vec::new();
    for node in topology.nodes {
        for (service, current_state) in [
            ("node", node.services.node.as_str()),
            ("rpc", node.services.rpc.as_str()),
            ("indexer", node.services.indexer_timer.as_str()),
            ("admin", node.services.admin.as_str()),
            ("stratum", node.services.stratum.as_str()),
            ("explorer", node.services.explorer.as_str()),
            ("validator", node.services.validator.as_str()),
        ] {
            let healthy = match current_state {
                "active" => Some(true),
                "inactive" | "failed" => Some(false),
                _ => None,
            };
            services.push(ServiceInventoryItem {
                node_id: node.node_id.clone(),
                node_name: node.node_name.clone(),
                service: service.to_owned(),
                state: current_state.to_owned(),
                healthy,
                evidence: "reported service state and fixed local dependency probes".to_owned(),
            });
        }
    }
    Ok(Json(services))
}

async fn create_invitation(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(request): Json<InvitationRequest>,
) -> Result<Json<InvitationResponse>, (StatusCode, Json<Value>)> {
    validate_label("node_name", &request.node_name).map_err(bad_request)?;
    validate_host(&request.advertise_host).map_err(bad_request)?;
    if let Some(domain) = &request.admin_domain {
        validate_host(domain).map_err(bad_request)?;
    }
    validate_email(&request.acme_email).map_err(bad_request)?;
    if !docker_mode() {
        return Err(bad_request(
            "VPS enrollment is available only in Docker deployment mode",
        ));
    }
    let fingerprint = request_fingerprint(&json!({
        "node_name": request.node_name,
        "advertise_host": request.advertise_host,
        "admin_domain": request.admin_domain,
        "acme_email": request.acme_email,
        "role": request.role.as_str(),
    }))?;
    let mutation = state
        .store
        .create_invitation_controlled(
            request.node_name.clone(),
            request.advertise_host.clone(),
            state.config.invitation_ttl_seconds,
            mutation_context(&headers, &fingerprint)?,
        )
        .map_err(conflict)?;
    let ControlledMutation::Applied {
        value: invite,
        response: operation,
    } = mutation
    else {
        return Err(conflict(
            "invitation already created for this idempotency key; use the invitations inventory",
        ));
    };
    let controller = request
        .admin_domain
        .as_deref()
        .map(|domain| format!("https://{domain}"))
        .unwrap_or_else(|| format!("https://{}", state.config.advertise_host));
    let seed = format!(
        "/dns4/{}/tcp/{}",
        state.config.advertise_host, state.config.p2p_port
    );
    let install_command = enrollment_command(&state, &request, &controller, &seed);
    let steps = enrollment_steps(
        &request,
        &install_command,
        invite.expires_at_unix_seconds,
        state.config.p2p_port,
    );
    Ok(Json(InvitationResponse {
        operation_id: operation.operation_id,
        replayed: false,
        invitation_id: invite.id,
        enrollment_token: invite.token,
        role: request.role.as_str().to_owned(),
        expires_at_unix_seconds: invite.expires_at_unix_seconds,
        install_command,
        steps,
        seed_multiaddr: seed,
        controller_url: controller,
        node_name: request.node_name,
        advertise_host: request.advertise_host,
        ttl_seconds: state.config.invitation_ttl_seconds,
    }))
}

async fn list_invitations(
    State(state): State<AdminState>,
) -> Result<Json<Vec<InvitationView>>, (StatusCode, Json<Value>)> {
    state
        .store
        .invitation_views(unix_seconds())
        .map(Json)
        .map_err(internal)
}

async fn revoke_invitation(
    State(state): State<AdminState>,
    headers: HeaderMap,
    PathParam(invitation_id): PathParam<String>,
) -> Result<Json<MutationResponse>, (StatusCode, Json<Value>)> {
    validate_identifier("invitation_id", &invitation_id).map_err(bad_request)?;
    let fingerprint = request_fingerprint(&json!({
        "action": "revoke-invitation",
        "invitation_id": invitation_id,
    }))?;
    let mutation = state
        .store
        .revoke_invitation_controlled(&invitation_id, mutation_context(&headers, &fingerprint)?)
        .map_err(operation_error)?;
    Ok(Json(mutation_response(mutation)))
}

async fn remove_node(
    State(state): State<AdminState>,
    headers: HeaderMap,
    PathParam(node_id): PathParam<String>,
) -> Result<Json<MutationResponse>, (StatusCode, Json<Value>)> {
    validate_identifier("node_id", &node_id).map_err(bad_request)?;
    let fingerprint = request_fingerprint(&json!({
        "action": "remove-node",
        "node_id": node_id,
    }))?;
    let mutation = state
        .store
        .remove_node_controlled(&node_id, mutation_context(&headers, &fingerprint)?)
        .map_err(operation_error)?;
    Ok(Json(mutation_response(mutation)))
}

async fn ban_node(
    State(state): State<AdminState>,
    headers: HeaderMap,
    PathParam(node_id): PathParam<String>,
    Json(request): Json<BanRequest>,
) -> Result<Json<MutationResponse>, (StatusCode, Json<Value>)> {
    validate_identifier("node_id", &node_id).map_err(bad_request)?;
    validate_reason(&request.reason).map_err(bad_request)?;
    let fingerprint = request_fingerprint(&json!({
        "action": "ban-node",
        "node_id": node_id,
        "reason": request.reason,
    }))?;
    let mutation = state
        .store
        .set_node_ban_controlled(
            &node_id,
            Some(request.reason),
            mutation_context(&headers, &fingerprint)?,
        )
        .map_err(operation_error)?;
    Ok(Json(mutation_response(mutation)))
}

async fn unban_node(
    State(state): State<AdminState>,
    headers: HeaderMap,
    PathParam(node_id): PathParam<String>,
) -> Result<Json<MutationResponse>, (StatusCode, Json<Value>)> {
    validate_identifier("node_id", &node_id).map_err(bad_request)?;
    let fingerprint = request_fingerprint(&json!({
        "action": "unban-node",
        "node_id": node_id,
    }))?;
    let mutation = state
        .store
        .set_node_ban_controlled(&node_id, None, mutation_context(&headers, &fingerprint)?)
        .map_err(operation_error)?;
    Ok(Json(mutation_response(mutation)))
}

async fn node_detail(
    State(state): State<AdminState>,
    PathParam(node_id): PathParam<String>,
) -> Result<Json<NodeDetailView>, (StatusCode, Json<Value>)> {
    let now = unix_seconds();
    if node_id == "local-controller" {
        let local = collect_local_report(&state).await;
        let node = state
            .store
            .local_node_view(&local, now, state.config.report_interval_seconds)
            .map_err(internal)?;
        return Ok(Json(NodeDetailView {
            node,
            report: local,
            is_local_controller: true,
        }));
    }
    validate_identifier("node_id", &node_id).map_err(bad_request)?;
    let report = state
        .store
        .get_node_report(&node_id)
        .map_err(internal)?
        .ok_or_else(|| not_found("node not found"))?;
    let node = state
        .store
        .node_views(now, state.config.report_interval_seconds)
        .map_err(internal)?
        .into_iter()
        .find(|node| node.node_id == node_id)
        .ok_or_else(|| not_found("node not found"))?;
    Ok(Json(NodeDetailView {
        node,
        report,
        is_local_controller: false,
    }))
}

async fn fleet_summary(
    State(state): State<AdminState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let local = collect_local_report(&state).await;
    let topology = build_topology(&state, Some(local.clone())).map_err(internal)?;
    let invitations = state
        .store
        .invitation_views(unix_seconds())
        .map_err(internal)?;
    let pending = invitations
        .iter()
        .filter(|item| item.status == "pending")
        .count();
    Ok(Json(json!({
        "network_id": state.config.network_id,
        "status_label": state.config.status_label,
        "controller": state.config.node_name,
        "advertise_host": state.config.advertise_host,
        "p2p_port": state.config.p2p_port,
        "release_bundle_url": state.config.release_bundle_url,
        "invitation_ttl_seconds": state.config.invitation_ttl_seconds,
        "topology": topology,
        "pending_invitations": pending,
        "invitations": invitations,
        "local_height": local.status.get("height"),
        "local_tip": local.status.get("tip_hash"),
        "generated_at_unix_seconds": unix_seconds(),
    })))
}

async fn audit_log(
    State(state): State<AdminState>,
) -> Result<Json<Vec<AuditEntry>>, (StatusCode, Json<Value>)> {
    state.store.audit_entries(250).map(Json).map_err(internal)
}

async fn bootstrap_roles() -> Json<Value> {
    Json(json!({
        "roles": ["node", "validator", "indexer", "explorer", "stratum", "full-stack"],
        "validator_semantics": "PoW full-validation node; Alvenqis has no staking validator role",
        "execution": "manifest-only",
        "secrets": "supplied out-of-band at execution time",
    }))
}

async fn generate_bootstrap_manifest(
    State(state): State<AdminState>,
    Json(request): Json<BootstrapManifestRequest>,
) -> Result<Json<BootstrapManifest>, (StatusCode, Json<Value>)> {
    validate_label("node_name", &request.node_name).map_err(bad_request)?;
    validate_host(&request.advertise_host).map_err(bad_request)?;
    validate_email(&request.acme_email).map_err(bad_request)?;
    if let Some(domain) = &request.admin_domain {
        validate_host(domain).map_err(bad_request)?;
    }
    let controller_url = request
        .admin_domain
        .as_ref()
        .map(|domain| format!("https://{domain}"))
        .or_else(|| state.config.controller_url.clone())
        .unwrap_or_else(|| format!("https://{}", state.config.advertise_host));
    let components = components_for_role(&request.role);
    let ports = ports_for_role(&request.role, state.config.p2p_port);
    let mut environment = BTreeMap::new();
    environment.insert(
        "ALVENQIS_NETWORK_ID".to_owned(),
        state.config.network_id.clone(),
    );
    environment.insert(
        "ALVENQIS_DEPLOYMENT_ROLE".to_owned(),
        request.role.as_str().to_owned(),
    );
    environment.insert("NODE_NAME".to_owned(), request.node_name.clone());
    environment.insert("P2P_HOST".to_owned(), request.advertise_host.clone());
    environment.insert("P2P_PORT".to_owned(), state.config.p2p_port.to_string());
    environment.insert("CONTROLLER_URL".to_owned(), controller_url.clone());
    environment.insert("ADMIN_EMAIL".to_owned(), request.acme_email.clone());
    let command = manifest_command_template(
        &state.config.release_bundle_url,
        &request,
        &controller_url,
        &format!(
            "/dns4/{}/tcp/{}",
            state.config.advertise_host, state.config.p2p_port
        ),
    );
    Ok(Json(BootstrapManifest {
        schema_version: "alvenqis.bootstrap/v1",
        network_id: state.config.network_id.clone(),
        role: request.role.as_str().to_owned(),
        node_name: request.node_name,
        advertise_host: request.advertise_host.clone(),
        controller_url,
        immutable_release_bundle_url: state.config.release_bundle_url.clone(),
        components,
        required_ports: ports,
        environment,
        apt_packages: vec![
            "ca-certificates",
            "curl",
            "docker-ce",
            "docker-ce-cli",
            "containerd.io",
            "docker-buildx-plugin",
            "docker-compose-plugin",
            "openssl",
            "python3",
        ],
        install_command_template: command,
        secrets_required: vec!["ALVENQIS_ENROLLMENT_TOKEN"],
        contains_secrets: false,
        post_install_checks: vec![
            "docker compose ps must report required services healthy".to_owned(),
            format!(
                "TCP {} must accept the configured P2P listener",
                state.config.p2p_port
            ),
            "controller /fleet/report must accept the first authenticated report".to_owned(),
            "RPC /p2p/status and /indexer/status must return real JSON without error".to_owned(),
            "Stratum roles require a certificate-valid TLS handshake on the configured port"
                .to_owned(),
        ],
    }))
}

async fn enroll(
    State(state): State<AdminState>,
    Json(mut request): Json<EnrollmentRequest>,
) -> Result<Json<EnrollmentResponse>, (StatusCode, Json<Value>)> {
    validate_report(&state.config, &request.report).map_err(bad_request)?;
    sanitize_report(&mut request.report);
    let (node_id, node_token) = state
        .store
        .enroll(
            &request.invitation_token,
            request.report,
            unix_seconds(),
            state.config.report_interval_seconds,
        )
        .map_err(unauthorized)?;
    Ok(Json(EnrollmentResponse {
        node_id,
        node_token,
    }))
}

async fn report(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(mut request): Json<ReportRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    validate_report(&state.config, &request.report).map_err(bad_request)?;
    sanitize_report(&mut request.report);
    let token = bearer_token(&headers).ok_or_else(|| unauthorized("missing bearer token"))?;
    state
        .store
        .update_report(
            &request.node_id,
            token,
            request.report,
            state.config.report_interval_seconds,
        )
        .map_err(unauthorized)?;
    Ok(StatusCode::NO_CONTENT)
}

fn build_topology(state: &AdminState, local: Option<NodeReport>) -> Result<FleetTopology, String> {
    let now = unix_seconds();
    let mut nodes = state
        .store
        .node_views(now, state.config.report_interval_seconds)?;
    if let Some(report) = local {
        nodes.insert(
            0,
            state
                .store
                .local_node_view(&report, now, state.config.report_interval_seconds)?,
        );
    }
    let online_node_count = nodes.iter().filter(|node| node.online).count();
    let banned_node_count = nodes.iter().filter(|node| node.banned).count();
    Ok(FleetTopology {
        mode: "Observed fleet telemetry; not a global network census",
        network_id: state.config.network_id.clone(),
        generated_at_unix_seconds: now,
        registered_node_count: nodes.len(),
        online_node_count,
        banned_node_count,
        direct_validated_connections: nodes.iter().map(|node| node.validating_peers).sum(),
        observed_miner_count: nodes
            .iter()
            .map(|node| node.mining_peers)
            .max()
            .unwrap_or(0),
        observed_hashrate_hs: nodes
            .iter()
            .map(|node| node.observed_hashrate_hs)
            .max()
            .unwrap_or(0),
        nodes,
    })
}

async fn collect_local_report(state: &AdminState) -> NodeReport {
    let base = state.config.local_rpc_url.trim_end_matches('/');
    let now = unix_seconds();
    let status_url = format!("{base}/status");
    let sync_url = format!("{base}/sync/status");
    let p2p_url = format!("{base}/p2p/status");
    let mempool_url = format!("{base}/mempool/status");
    let indexer_url = format!("{base}/indexer/status");
    let (status, sync, p2p, mempool, indexer, stratum_probe) = tokio::join!(
        get_json_probe(&state.client, &status_url, "status"),
        get_json_probe(&state.client, &sync_url, "sync"),
        get_json_probe(&state.client, &p2p_url, "p2p"),
        get_json_probe(&state.client, &mempool_url, "mempool"),
        get_json_probe(&state.client, &indexer_url, "indexer"),
        probe_stratum_tls(now),
    );
    let mut probes = BTreeMap::new();
    probes.insert("status".to_owned(), status.1);
    probes.insert("sync".to_owned(), sync.1);
    probes.insert("p2p".to_owned(), p2p.1);
    probes.insert("mempool".to_owned(), mempool.1);
    probes.insert("indexer".to_owned(), indexer.1);
    probes.insert("stratum_tls".to_owned(), stratum_probe.clone());
    let services = if docker_mode() {
        ServiceStates {
            node: probe_service_state(probes.get("p2p")),
            rpc: probe_service_state(probes.get("status")),
            indexer_timer: probe_service_state(probes.get("indexer")),
            admin: "active".to_owned(),
            stratum: if stratum_probe.configured {
                probe_service_state(Some(&stratum_probe))
            } else {
                "not-configured".to_owned()
            },
            explorer: "not-observed".to_owned(),
            validator: validator_state(&status.0),
        }
    } else {
        service_states(&stratum_probe, &status.0)
    };
    let mut report = NodeReport {
        network_id: state.config.network_id.clone(),
        node_name: state.config.node_name.clone(),
        advertise_host: state.config.advertise_host.clone(),
        p2p_multiaddr: format!(
            "/dns4/{}/tcp/{}",
            state.config.advertise_host, state.config.p2p_port
        ),
        reported_at_unix_seconds: now,
        services,
        status: status.0,
        sync: sync.0,
        p2p: p2p.0,
        mempool: mempool.0,
        indexer: indexer.0,
        stratum: serde_json::to_value(&stratum_probe).unwrap_or(Value::Null),
        probes,
    };
    sanitize_report(&mut report);
    report
}

async fn get_json_probe(client: &Client, url: &str, name: &str) -> (Value, ProbeResult) {
    let started = Instant::now();
    let now = unix_seconds();
    match client.get(url).send().await {
        Ok(response) if response.status().is_success() => {
            let latency = duration_millis(started.elapsed());
            match response.json::<Value>().await {
                Ok(value) => (
                    value,
                    ProbeResult {
                        configured: true,
                        healthy: true,
                        checked_at_unix_seconds: now,
                        latency_ms: Some(latency),
                        detail: format!("{name} returned valid JSON"),
                        error: None,
                    },
                ),
                Err(error) => {
                    let message = sanitize_error(error);
                    (
                        json!({"error": message}),
                        failed_probe(name, now, latency, "response was not valid JSON", message),
                    )
                }
            }
        }
        Ok(response) => {
            let latency = duration_millis(started.elapsed());
            let message = format!("HTTP {}", response.status());
            (
                json!({"error": message}),
                failed_probe(name, now, latency, "endpoint rejected probe", message),
            )
        }
        Err(error) => {
            let latency = duration_millis(started.elapsed());
            let message = sanitize_error(error);
            (
                json!({"error": message}),
                failed_probe(name, now, latency, "endpoint request failed", message),
            )
        }
    }
}

async fn probe_stratum_tls(now: u64) -> ProbeResult {
    if !env_flag("ENABLE_POOL") {
        return ProbeResult::not_configured("Stratum pool is disabled on this host", now);
    }
    let host = match env::var("STRATUM_HOST") {
        Ok(value) if validate_host(&value).is_ok() => value,
        Ok(_) => {
            return failed_probe(
                "stratum_tls",
                now,
                0,
                "Stratum is enabled but STRATUM_HOST is invalid",
                "invalid STRATUM_HOST".to_owned(),
            )
        }
        Err(_) => {
            return failed_probe(
                "stratum_tls",
                now,
                0,
                "Stratum is enabled but STRATUM_HOST is missing",
                "missing STRATUM_HOST".to_owned(),
            )
        }
    };
    let port = env::var("STRATUM_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3333);
    probe_tls_endpoint(&host, port, now).await
}

async fn probe_tls_endpoint(host: &str, port: u16, now: u64) -> ProbeResult {
    let started = Instant::now();
    let server_name = match ServerName::try_from(host.to_owned()) {
        Ok(value) => value,
        Err(error) => {
            return failed_probe(
                "stratum_tls",
                now,
                0,
                "invalid TLS server name",
                sanitize_error(error),
            )
        }
    };
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let tcp = match timeout_connect((host, port)).await {
        Ok(stream) => stream,
        Err(error) => {
            return failed_probe(
                "stratum_tls",
                now,
                duration_millis(started.elapsed()),
                "TCP connection failed",
                error,
            )
        }
    };
    match tokio::time::timeout(
        Duration::from_secs(4),
        TlsConnector::from(Arc::new(tls)).connect(server_name, tcp),
    )
    .await
    {
        Ok(Ok(_stream)) => ProbeResult {
            configured: true,
            healthy: true,
            checked_at_unix_seconds: now,
            latency_ms: Some(duration_millis(started.elapsed())),
            detail: format!("certificate-valid TLS handshake succeeded on {host}:{port}"),
            error: None,
        },
        Ok(Err(error)) => failed_probe(
            "stratum_tls",
            now,
            duration_millis(started.elapsed()),
            "TLS certificate or handshake validation failed",
            sanitize_error(error),
        ),
        Err(_) => failed_probe(
            "stratum_tls",
            now,
            duration_millis(started.elapsed()),
            "TLS handshake timed out",
            "timeout after 4 seconds".to_owned(),
        ),
    }
}

async fn timeout_connect<A: ToSocketAddrs>(address: A) -> Result<TcpStream, String> {
    tokio::time::timeout(Duration::from_secs(4), TcpStream::connect(address))
        .await
        .map_err(|_| "timeout after 4 seconds".to_owned())?
        .map_err(sanitize_error)
}

fn failed_probe(
    name: &str,
    now: u64,
    latency_ms: u64,
    detail: impl Into<String>,
    error: String,
) -> ProbeResult {
    ProbeResult {
        configured: true,
        healthy: false,
        checked_at_unix_seconds: now,
        latency_ms: Some(latency_ms),
        detail: format!("{name}: {}", detail.into()),
        error: Some(truncate(&error, 320)),
    }
}

fn service_states(stratum_probe: &ProbeResult, status: &Value) -> ServiceStates {
    ServiceStates {
        node: systemd_state("alvenqis-node"),
        rpc: systemd_state("alvenqis-rpc"),
        indexer_timer: systemd_state("alvenqis-indexer-refresh.timer"),
        admin: systemd_state("alvenqis-vps-admin"),
        stratum: if stratum_probe.configured {
            systemd_state("alvenqis-mining-pool")
        } else {
            "not-configured".to_owned()
        },
        explorer: systemd_state("alvenqis-explorer"),
        validator: validator_state(status),
    }
}

fn probe_service_state(probe: Option<&ProbeResult>) -> String {
    match probe {
        Some(probe) if probe.healthy => "active",
        Some(probe) if probe.configured => "inactive",
        _ => "not-configured",
    }
    .to_owned()
}

fn validator_state(status: &Value) -> String {
    if status.get("validator").and_then(Value::as_bool) == Some(true)
        || status
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role.eq_ignore_ascii_case("validator"))
    {
        "active".to_owned()
    } else {
        "not-observed".to_owned()
    }
}

fn systemd_state(unit: &str) -> String {
    if !cfg!(target_os = "linux") {
        return "not-applicable".to_owned();
    }
    Command::new("systemctl")
        .args(["is-active", unit])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

pub async fn run_health_sampler(state: AdminState) {
    loop {
        let report = collect_local_report(&state).await;
        if let Err(error) = state
            .store
            .record_local_observation(&report, state.config.report_interval_seconds)
        {
            eprintln!("local health sample failed: {error}");
        }
        tokio::time::sleep(Duration::from_secs(state.config.report_interval_seconds)).await;
    }
}

pub async fn run_agent_reporter(state: AdminState) {
    let Some(controller) = state
        .config
        .controller_url
        .clone()
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    loop {
        if let Err(error) = report_once(&state, &controller).await {
            eprintln!("fleet report failed: {error}");
        }
        tokio::time::sleep(Duration::from_secs(state.config.report_interval_seconds)).await;
    }
}

async fn report_once(state: &AdminState, controller: &str) -> Result<(), String> {
    let credentials_path = state.config.state_dir.join("agent-credentials.json");
    let report = collect_local_report(state).await;
    if credentials_path.exists() {
        let credentials: Value = serde_json::from_str(
            &fs::read_to_string(&credentials_path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let node_id = credentials["node_id"]
            .as_str()
            .ok_or("agent credentials missing node_id")?;
        let node_token = credentials["node_token"]
            .as_str()
            .ok_or("agent credentials missing node_token")?;
        state
            .client
            .post(format!("{}/fleet/report", controller.trim_end_matches('/')))
            .bearer_auth(node_token)
            .json(&ReportRequestOwned { node_id, report })
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    let invitation_path = state.config.state_dir.join("enrollment.token");
    let token = fs::read_to_string(&invitation_path)
        .map_err(|_| "waiting for enrollment.token".to_owned())?;
    let response: EnrollmentResponse = state
        .client
        .post(format!("{}/fleet/enroll", controller.trim_end_matches('/')))
        .json(&EnrollmentRequestOwned {
            invitation_token: token.trim(),
            report,
        })
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    write_private_json(&credentials_path, &response)?;
    let _ = fs::remove_file(invitation_path);
    Ok(())
}

#[derive(serde::Serialize)]
struct EnrollmentRequestOwned<'a> {
    invitation_token: &'a str,
    report: NodeReport,
}

#[derive(serde::Serialize)]
struct ReportRequestOwned<'a> {
    node_id: &'a str,
    report: NodeReport,
}

fn write_private_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn validate_report(config: &AdminConfig, report: &NodeReport) -> Result<(), String> {
    if report.network_id != config.network_id {
        return Err("report belongs to another network".to_owned());
    }
    validate_label("node_name", &report.node_name)?;
    validate_host(&report.advertise_host)?;
    let expected_multiaddr = format!("/dns4/{}/tcp/{}", report.advertise_host, config.p2p_port);
    if report.p2p_multiaddr != expected_multiaddr {
        return Err("p2p_multiaddr does not match the enrolled host and fleet port".to_owned());
    }
    let now = unix_seconds();
    if report.reported_at_unix_seconds > now.saturating_add(60)
        || now.saturating_sub(report.reported_at_unix_seconds) > 300
    {
        return Err("report timestamp is outside the accepted clock window".to_owned());
    }
    let encoded = serde_json::to_vec(report).map_err(|error| error.to_string())?;
    if encoded.len() > MAX_REPORT_BYTES {
        return Err("report exceeds the 512 KiB limit".to_owned());
    }
    if report.probes.len() > 16 {
        return Err("report contains too many probes".to_owned());
    }
    for (name, probe) in &report.probes {
        validate_label("probe name", name)?;
        if probe.detail.len() > 512
            || probe.error.as_ref().is_some_and(|error| error.len() > 512)
            || probe.checked_at_unix_seconds > now.saturating_add(60)
        {
            return Err("probe evidence is invalid or too large".to_owned());
        }
    }
    Ok(())
}

fn sanitize_report(report: &mut NodeReport) {
    for value in [
        &mut report.status,
        &mut report.sync,
        &mut report.p2p,
        &mut report.mempool,
        &mut report.indexer,
        &mut report.stratum,
    ] {
        redact_sensitive(value);
    }
}

fn redact_sensitive(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                let lower = key.to_ascii_lowercase();
                if ["password", "secret", "token", "private_key", "mnemonic"]
                    .iter()
                    .any(|needle| lower.contains(needle))
                {
                    *item = Value::String("[redacted]".to_owned());
                } else {
                    redact_sensitive(item);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_sensitive),
        _ => {}
    }
}

fn enrollment_command(
    state: &AdminState,
    request: &InvitationRequest,
    controller: &str,
    seed: &str,
) -> String {
    let (profile, targets, stop_targets) = compose_plan(request.role);
    format!(
        "set -Eeuo pipefail\nread -r -s -p 'Enrollment token: ' ALVENQIS_ENROLLMENT_TOKEN\nprintf '\\n'\n{docker_install}\ninstall -d -m 0755 /opt/alvenqis-agent\ntest -z \"$(ls -A /opt/alvenqis-agent)\" || {{ echo 'Refusing to overwrite /opt/alvenqis-agent' >&2; exit 73; }}\ncurl -fsSL {bundle} -o /tmp/alvenqis-docker-control-plane.tar.gz\ncurl -fsSL {bundle}.sha256 -o /tmp/alvenqis-docker-control-plane.tar.gz.sha256\ncd /tmp\nsha256sum -c alvenqis-docker-control-plane.tar.gz.sha256\ntar -xzf alvenqis-docker-control-plane.tar.gz -C /opt/alvenqis-agent\ncd /opt/alvenqis-agent/alvenqis-release/vps-control-plane\nprintf '%s\\n' \"$ALVENQIS_ENROLLMENT_TOKEN\" | ./scripts/enroll-docker-node.sh --node-name {node} --p2p-host {domain} --email {email} --controller-url {controller} --enrollment-token-stdin --seed {seed} --release-bundle-url {bundle}\nunset ALVENQIS_ENROLLMENT_TOKEN\nprintf '\\nALVENQIS_DEPLOYMENT_ROLE=%s\\nENABLE_POOL=%s\\n' {role} {enable_pool} >> .env\n{stop_command}\ndocker compose --env-file .env -f compose.yaml {profile} up -d --build {targets}\ndocker compose --env-file .env -f compose.yaml {profile} ps {targets}\ninstall -m 0600 /dev/null /root/alvenqis-access.txt\n{{ printf 'Node: %s\\nRole: %s\\nController: %s\\nControl URL: %s\\n' {node} {role} {controller} {control_url}; if test -s state/secrets/admin_password; then printf 'Admin password: '; cat state/secrets/admin_password; fi; }} > /root/alvenqis-access.txt\nchmod 0600 /root/alvenqis-access.txt\nprintf 'Credentials saved to /root/alvenqis-access.txt (mode 0600)\\n'\n",
        docker_install = docker_install_commands(),
        bundle = shell_arg(&state.config.release_bundle_url),
        node = shell_arg(&request.node_name),
        domain = shell_arg(&request.advertise_host),
        email = shell_arg(&request.acme_email),
        controller = shell_arg(controller),
        seed = shell_arg(seed),
        role = shell_arg(request.role.as_str()),
        enable_pool = shell_arg(if matches!(request.role, DeploymentRole::Stratum | DeploymentRole::FullStack) { "true" } else { "false" }),
        stop_command = if stop_targets.is_empty() {
            ":".to_owned()
        } else {
            format!("docker compose --env-file .env -f compose.yaml stop {stop_targets} || true")
        },
        profile = profile,
        targets = targets,
        control_url = shell_arg(&format!("https://{}", request.admin_domain.as_deref().unwrap_or(&request.advertise_host))),
    )
}

fn enrollment_steps(
    request: &InvitationRequest,
    install_command: &str,
    expires_at: u64,
    p2p_port: u16,
) -> Vec<EnrollmentStep> {
    vec![
        EnrollmentStep {
            title: "1 - Prepare DNS and firewall".into(),
            detail: format!(
                "Point DNS for {} to the new host and open TCP {} for P2P.",
                request.advertise_host, p2p_port
            ),
            code: None,
        },
        EnrollmentStep {
            title: "2 - SSH to the new host".into(),
            detail: "Use a clean Ubuntu 24.04 machine and run as root.".into(),
            code: Some(format!("ssh root@{}", request.advertise_host)),
        },
        EnrollmentStep {
            title: "3 - Run one-time enrollment".into(),
            detail: format!(
                "The authenticated response contains a single-use token expiring at unix:{expires_at}."
            ),
            code: Some(install_command.to_owned()),
        },
        EnrollmentStep {
            title: "4 - Verify real probes".into(),
            detail:
                "Wait for the first report, then require healthy P2P, RPC and indexer probes."
                    .into(),
            code: None,
        },
    ]
}

fn manifest_command_template(
    bundle: &str,
    request: &BootstrapManifestRequest,
    controller: &str,
    seed: &str,
) -> String {
    let (profile, targets, stop_targets) = compose_plan(request.role);
    format!(
        "set -Eeuo pipefail\nread -r -s -p 'Enrollment token: ' ALVENQIS_ENROLLMENT_TOKEN\nprintf '\\n'\n{docker_install}\ninstall -d -m 0755 /opt/alvenqis-agent\ntest -z \"$(ls -A /opt/alvenqis-agent)\" || {{ echo 'Refusing to overwrite /opt/alvenqis-agent' >&2; exit 73; }}\ncurl -fsSL {bundle} -o /tmp/alvenqis-control-plane.tar.gz\ncurl -fsSL {bundle}.sha256 -o /tmp/alvenqis-control-plane.tar.gz.sha256\ncd /tmp && sha256sum -c alvenqis-control-plane.tar.gz.sha256\ntar -xzf /tmp/alvenqis-control-plane.tar.gz -C /opt/alvenqis-agent\ncd /opt/alvenqis-agent/alvenqis-release/vps-control-plane\nprintf '%s\\n' \"$ALVENQIS_ENROLLMENT_TOKEN\" | ./scripts/enroll-docker-node.sh --node-name {node} --p2p-host {host} --email {email} --controller-url {controller} --enrollment-token-stdin --seed {seed} --release-bundle-url {bundle}\nunset ALVENQIS_ENROLLMENT_TOKEN\nprintf '\\nALVENQIS_DEPLOYMENT_ROLE=%s\\nENABLE_POOL=%s\\n' {role} {enable_pool} >> .env\n{stop_command}\ndocker compose --env-file .env -f compose.yaml {profile} up -d --build {targets}\ndocker compose --env-file .env -f compose.yaml {profile} ps {targets}\ninstall -m 0600 /dev/null /root/alvenqis-access.txt\n{{ printf 'Node: %s\\nRole: %s\\nController: %s\\n' {node} {role} {controller}; if test -s state/secrets/admin_password; then printf 'Admin password: '; cat state/secrets/admin_password; fi; }} > /root/alvenqis-access.txt\nchmod 0600 /root/alvenqis-access.txt\n",
        docker_install = docker_install_commands(),
        bundle = shell_arg(bundle),
        node = shell_arg(&request.node_name),
        host = shell_arg(&request.advertise_host),
        email = shell_arg(&request.acme_email),
        controller = shell_arg(controller),
        seed = shell_arg(seed),
        role = shell_arg(request.role.as_str()),
        enable_pool = shell_arg(if matches!(request.role, DeploymentRole::Stratum | DeploymentRole::FullStack) { "true" } else { "false" }),
        stop_command = if stop_targets.is_empty() { ":".to_owned() } else { format!("docker compose --env-file .env -f compose.yaml stop {stop_targets} || true") },
        profile = profile,
        targets = targets,
    )
}

fn docker_install_commands() -> &'static str {
    "export DEBIAN_FRONTEND=noninteractive\napt-get update\napt-get install -y ca-certificates curl openssl python3\ninstall -m 0755 -d /etc/apt/keyrings\ncurl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc\nchmod a+r /etc/apt/keyrings/docker.asc\n. /etc/os-release\necho \"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${UBUNTU_CODENAME:-$VERSION_CODENAME} stable\" > /etc/apt/sources.list.d/docker.list\napt-get update\napt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin\nsystemctl enable --now docker"
}

fn compose_plan(role: DeploymentRole) -> (&'static str, &'static str, &'static str) {
    match role {
        DeploymentRole::Node | DeploymentRole::Validator => (
            "",
            "alvenqis-node alvenqis-rpc alvenqis-control",
            "alvenqis-indexer",
        ),
        DeploymentRole::Indexer => (
            "",
            "alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control",
            "",
        ),
        DeploymentRole::Explorer => (
            "",
            "alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control alvenqis-explorer gateway",
            "",
        ),
        DeploymentRole::Stratum => (
            "--profile pool",
            "alvenqis-node alvenqis-rpc alvenqis-control stratum-certbot alvenqis-pool",
            "alvenqis-indexer",
        ),
        DeploymentRole::FullStack => (
            "--profile pool",
            "alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control stratum-certbot alvenqis-pool alvenqis-website alvenqis-explorer gateway",
            "",
        ),
    }
}

fn components_for_role(role: &DeploymentRole) -> Vec<&'static str> {
    match role {
        DeploymentRole::Node => vec!["alvenqis-node", "alvenqis-rpc", "alvenqis-control"],
        DeploymentRole::Validator => {
            vec!["alvenqis-node", "alvenqis-rpc", "alvenqis-control"]
        }
        DeploymentRole::Indexer => vec![
            "alvenqis-node",
            "alvenqis-rpc",
            "alvenqis-indexer",
            "alvenqis-control",
        ],
        DeploymentRole::Explorer => vec![
            "alvenqis-node",
            "alvenqis-rpc",
            "alvenqis-indexer",
            "alvenqis-control",
            "alvenqis-explorer",
            "gateway",
        ],
        DeploymentRole::Stratum => vec![
            "alvenqis-node",
            "alvenqis-rpc",
            "alvenqis-pool",
            "stratum-certbot",
            "alvenqis-control",
        ],
        DeploymentRole::FullStack => vec![
            "alvenqis-node",
            "alvenqis-rpc",
            "alvenqis-indexer",
            "alvenqis-pool",
            "stratum-certbot",
            "alvenqis-control",
            "alvenqis-website",
            "alvenqis-explorer",
            "gateway",
        ],
    }
}

fn ports_for_role(role: &DeploymentRole, p2p_port: u16) -> Vec<BootstrapPort> {
    let mut ports = vec![BootstrapPort {
        port: p2p_port,
        transport: "tcp",
        purpose: "p2p",
        public: true,
    }];
    if matches!(role, DeploymentRole::Explorer | DeploymentRole::FullStack) {
        ports.push(BootstrapPort {
            port: 443,
            transport: "tcp",
            purpose: "https",
            public: true,
        });
    }
    if matches!(role, DeploymentRole::Stratum | DeploymentRole::FullStack) {
        ports.push(BootstrapPort {
            port: 3333,
            transport: "tcp+tls",
            purpose: "stratum",
            public: true,
        });
    }
    ports
}

fn mutation_context<'a>(
    headers: &'a HeaderMap,
    request_sha256: &'a str,
) -> Result<MutationContext<'a>, (StatusCode, Json<Value>)> {
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| bad_request("Idempotency-Key header is required"))?;
    validate_idempotency_key(idempotency_key).map_err(bad_request)?;
    let actor = headers
        .get("x-alvenqis-admin-user")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("authenticated-proxy");
    validate_actor(actor).map_err(bad_request)?;
    Ok(MutationContext {
        actor,
        idempotency_key,
        request_sha256,
        now: unix_seconds(),
    })
}

fn mutation_response(mutation: ControlledMutation<()>) -> MutationResponse {
    match mutation {
        ControlledMutation::Applied { response, .. } => response,
        ControlledMutation::Replayed(response) => response,
    }
}

fn request_fingerprint(value: &Value) -> Result<String, (StatusCode, Json<Value>)> {
    serde_json::to_vec(value)
        .map(|bytes| hex::encode(Sha256::digest(bytes)))
        .map_err(internal)
}

fn validate_label(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return Err(format!("{field} contains unsupported characters"));
    }
    Ok(())
}

fn validate_identifier(field: &str, value: &str) -> Result<(), String> {
    if !(8..=64).contains(&value.len())
        || !value.chars().all(|character| character.is_ascii_hexdigit())
    {
        return Err(format!("{field} must be a hexadecimal identifier"));
    }
    Ok(())
}

fn validate_host(value: &str) -> Result<(), String> {
    validate_label("host", value)?;
    if value.starts_with('.') || value.ends_with('.') || value.contains("..") {
        return Err("host is not canonical".to_owned());
    }
    Ok(())
}

fn validate_email(value: &str) -> Result<(), String> {
    if value.len() > 254
        || !value.contains('@')
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "@-_.+".contains(character))
    {
        return Err("acme_email is invalid".to_owned());
    }
    Ok(())
}

fn validate_reason(value: &str) -> Result<(), String> {
    if !(3..=256).contains(&value.len())
        || value.chars().any(char::is_control)
        || value.trim() != value
    {
        return Err("ban reason must contain 3-256 printable characters".to_owned());
    }
    Ok(())
}

fn validate_idempotency_key(value: &str) -> Result<(), String> {
    if !(16..=128).contains(&value.len())
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character))
    {
        return Err("Idempotency-Key must contain 16-128 safe ASCII characters".to_owned());
    }
    Ok(())
}

fn validate_actor(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || value.chars().any(char::is_control)
        || value.trim() != value
    {
        return Err("authenticated actor header is invalid".to_owned());
    }
    Ok(())
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn shell_arg(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn docker_mode() -> bool {
    env::var("ALVENQIS_DEPLOYMENT_MODE").is_ok_and(|value| value.eq_ignore_ascii_case("docker"))
}

fn env_flag(name: &str) -> bool {
    env::var(name).is_ok_and(|value| {
        value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
    })
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn sanitize_error(error: impl ToString) -> String {
    truncate(&error.to_string().replace(['\r', '\n'], " "), 320)
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn operation_error(message: impl ToString) -> (StatusCode, Json<Value>) {
    let message = message.to_string();
    if message.contains("not found") {
        not_found(message)
    } else if message.contains("idempotency") {
        conflict(message)
    } else {
        bad_request(message)
    }
}

fn bad_request(message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": message.to_string()})),
    )
}

fn unauthorized(message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": message.to_string()})),
    )
}

fn not_found(message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": message.to_string()})),
    )
}

fn conflict(message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::CONFLICT,
        Json(json!({"error": message.to_string()})),
    )
}

fn internal(message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": message.to_string()})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request as HttpRequest;
    use std::path::PathBuf;
    use tower::ServiceExt;

    fn test_state() -> (AdminState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AdminConfig {
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 10788,
            network_id: "alvenqis-mainnet-candidate".to_owned(),
            status_label: "test".to_owned(),
            node_name: "controller".to_owned(),
            advertise_host: "controller.example.org".to_owned(),
            p2p_port: 20787,
            local_rpc_url: "http://127.0.0.1:9".to_owned(),
            state_dir: PathBuf::from(dir.path()),
            release_bundle_url: "https://example.org/release.tar.gz".to_owned(),
            controller_url: None,
            report_interval_seconds: 15,
            invitation_ttl_seconds: 900,
        };
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        (AdminState::new(config, store).expect("state"), dir)
    }

    #[tokio::test]
    async fn protected_routes_require_proxy_authentication() {
        let (state, _dir) = test_state();
        let response = router(state)
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/audit")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bootstrap_manifest_contains_no_secret() {
        let (state, _dir) = test_state();
        let response = router(state)
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/bootstrap/manifests")
                    .header("content-type", "application/json")
                    .header("x-alvenqis-admin-authenticated", "1")
                    .body(Body::from(
                        r#"{"role":"node","node_name":"node-1","advertise_host":"node1.example.org","acme_email":"ops@example.org"}"#,
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(payload["contains_secrets"], false);
        let command = payload["install_command_template"]
            .as_str()
            .expect("command");
        assert!(command.contains("ALVENQIS_ENROLLMENT_TOKEN"));
        assert!(!command.contains("Bearer "));
    }

    #[test]
    fn redacts_sensitive_report_fields_recursively() {
        let mut report = NodeReport {
            network_id: "alvenqis-mainnet-candidate".to_owned(),
            node_name: "node-1".to_owned(),
            advertise_host: "node1.example.org".to_owned(),
            p2p_multiaddr: "/dns4/node1.example.org/tcp/20787".to_owned(),
            reported_at_unix_seconds: unix_seconds(),
            services: ServiceStates::default(),
            status: json!({"nested": {"api_token": "do-not-store"}}),
            sync: json!({}),
            p2p: json!({}),
            mempool: json!({}),
            indexer: json!({}),
            stratum: json!({}),
            probes: BTreeMap::new(),
        };
        sanitize_report(&mut report);
        assert_eq!(report.status["nested"]["api_token"], "[redacted]");
    }

    #[test]
    fn enrollment_command_installs_docker_uses_allowlisted_role_and_secures_access_file() {
        let (state, _dir) = test_state();
        let request = InvitationRequest {
            role: DeploymentRole::Stratum,
            node_name: "pool-1".to_owned(),
            advertise_host: "pool1.example.org".to_owned(),
            admin_domain: Some("control.example.org".to_owned()),
            acme_email: "ops@example.org".to_owned(),
        };
        let command = enrollment_command(
            &state,
            &request,
            "https://control.example.org",
            "/dns4/controller.example.org/tcp/20787",
        );
        assert!(command.contains("apt-get update"));
        assert!(command.contains("docker-ce"));
        assert!(command.contains("--profile pool"));
        assert!(command.contains("stratum-certbot alvenqis-pool"));
        assert!(command.contains("/root/alvenqis-access.txt"));
        assert!(command.contains("chmod 0600"));
        assert!(command.contains("--enrollment-token-stdin"));
        assert!(!command.contains("--enrollment-token "));
        assert!(!command.contains("eval "));
    }
}
