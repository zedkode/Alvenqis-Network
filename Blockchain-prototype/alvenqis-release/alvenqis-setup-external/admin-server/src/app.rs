use crate::config::AdminConfig;
use crate::models::{
    AdminOverview, AuditEntry, BanRequest, BootstrapManifest, BootstrapManifestRequest,
    BootstrapPort, CertificateRotationRequest, CertificateRotationResponse, DeploymentRole,
    EnrollmentRequest, EnrollmentResponse, EnrollmentStep, FleetTopology, InvitationRequest,
    InvitationResponse, InvitationView, MutationResponse, NodeDetailView, NodeReport, ProbeResult,
    ReportRequest, ServiceInventoryItem, ServiceStates,
};
use crate::pki::FleetPki;
use crate::store::{ControlledMutation, FleetStore, MutationContext};
use axum::extract::{Path as PathParam, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use rand::{rngs::OsRng, RngCore};
use reqwest::Client;
use rustls::pki_types::ServerName;
use rustls::pki_types::{pem::PemObject, CertificateDer};
use rustls::{ClientConfig, RootCertStore};
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_rustls::TlsConnector;

const INDEX_HTML: &str = include_str!("../static/index.html");
const APP_JS: &str = include_str!("../static/app.js");
const STYLES_CSS: &str = include_str!("../static/styles.css");
const LOGO_PNG: &[u8] = include_bytes!("../static/logo.png");
const LOGO_MARK_PNG: &[u8] = include_bytes!("../static/logo-mark.png");
const MAX_REPORT_BYTES: usize = 512 * 1024;
const MAX_IDENTITY_FILE_BYTES: u64 = 1024 * 1024;
const AGENT_IDENTITY_DIRECTORY: &str = "agent-identity";
const AGENT_IDENTITY_VERSIONS_DIRECTORY: &str = "versions";
const AGENT_IDENTITY_CURRENT_MANIFEST: &str = "current";
const AGENT_KEY_FILE: &str = "client.key.pem";
const AGENT_CERTIFICATE_FILE: &str = "client.crt.pem";
const AGENT_CA_FILE: &str = "fleet-ca.crt.pem";
const AGENT_CREDENTIALS_FILE: &str = "credentials.json";
const LEGACY_AGENT_KEY_FILE: &str = "agent-client.key.pem";
const LEGACY_AGENT_CERTIFICATE_FILE: &str = "agent-client.crt.pem";
const LEGACY_AGENT_CA_FILE: &str = "fleet-ca.crt.pem";
const LEGACY_AGENT_CREDENTIALS_FILE: &str = "agent-credentials.json";
const CONTROL_PROXY_TOKEN_HEADER: &str = "x-alvenqis-proxy-token";
const CONTROL_PROXY_TOKEN_FILE_ENV: &str = "ALVENQIS_CONTROL_PROXY_TOKEN_FILE";
const DEFAULT_CONTROL_PROXY_TOKEN_FILE: &str = "/run/secrets/control_proxy_token";

#[derive(Clone)]
pub struct AdminState {
    pub config: AdminConfig,
    pub store: FleetStore,
    client: Client,
    pki: Option<Arc<FleetPki>>,
    control_proxy_token: Option<[u8; 32]>,
}

impl AdminState {
    pub fn new(config: AdminConfig, store: FleetStore) -> Result<Self, String> {
        let token_path = env::var_os(CONTROL_PROXY_TOKEN_FILE_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONTROL_PROXY_TOKEN_FILE));
        Self::new_for_mode(config, store, docker_mode(), &token_path)
    }

    fn new_for_mode(
        config: AdminConfig,
        store: FleetStore,
        docker_mode: bool,
        control_proxy_token_file: &Path,
    ) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .map_err(|error| error.to_string())?;
        let pki = if config
            .controller_url
            .as_deref()
            .unwrap_or_default()
            .is_empty()
        {
            if config
                .fleet_report_url
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err("controller requires fleet_report_url for mTLS reporting".to_owned());
            }
            if config
                .fleet_enrollment_url
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err("controller requires fleet_enrollment_url".to_owned());
            }
            let server_name = config
                .fleet_server_name
                .as_deref()
                .unwrap_or(&config.advertise_host);
            Some(Arc::new(FleetPki::load_or_initialize(
                &config.state_dir,
                server_name,
            )?))
        } else {
            None
        };
        let control_proxy_token = docker_mode
            .then(|| load_control_proxy_token(control_proxy_token_file))
            .transpose()?;
        Ok(Self {
            config,
            store,
            client,
            pki,
            control_proxy_token,
        })
    }
}

pub fn router(state: AdminState) -> Router {
    let viewer = Router::new()
        .route("/api/overview", get(overview))
        .route("/api/health", get(control_health))
        .route("/api/status", get(control_status))
        .route("/api/topology", get(topology))
        .route("/api/services", get(service_inventory))
        .route("/api/session", get(admin_session))
        .route("/api/nodes", get(topology))
        .route("/api/nodes/:node_id", get(node_detail))
        .route("/api/invitations", get(list_invitations))
        .route("/api/bootstrap/roles", get(bootstrap_roles))
        .route("/api/audit", get(audit_log))
        .route("/api/fleet/summary", get(fleet_summary))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_viewer,
        ));

    let operator = Router::new()
        .route("/api/nodes", post(create_invitation))
        .route("/api/nodes/:node_id", delete(remove_node))
        .route("/api/nodes/:node_id/ban", post(ban_node))
        .route("/api/nodes/:node_id/unban", post(unban_node))
        .route(
            "/api/nodes/:node_id/certificate/revoke",
            post(revoke_node_certificate),
        )
        .route("/api/invitations", post(create_invitation))
        .route("/api/invitations/:invitation_id", delete(revoke_invitation))
        .route(
            "/api/bootstrap/manifests",
            post(generate_bootstrap_manifest),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_operator,
        ));

    let fleet = Router::new()
        .route("/fleet/enroll", post(enroll))
        .route("/fleet/report", post(report))
        .route("/fleet/certificate/rotate", post(rotate_certificate))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_control_proxy,
        ));

    Router::new()
        .route("/", get(index))
        .route("/app.js", get(javascript))
        .route("/styles.css", get(styles))
        .route("/logo.png", get(logo_png))
        .route("/logo-mark.png", get(logo_mark_png))
        .route("/health", get(health))
        .route("/public/topology", get(public_topology))
        .merge(fleet)
        .merge(viewer)
        .merge(operator)
        .with_state(state)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AdminRole {
    Viewer,
    Operator,
}

#[derive(Clone, Copy, Debug)]
enum AdminAuthError {
    InvalidControlProxy,
    MissingAuthentication,
    InvalidRole,
}

impl AdminAuthError {
    fn response(self) -> Response {
        match self {
            Self::InvalidControlProxy => (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "trusted control proxy authentication required"})),
            )
                .into_response(),
            Self::MissingAuthentication => (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "admin reverse-proxy authentication required"})),
            )
                .into_response(),
            Self::InvalidRole => (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "a valid admin role is required"})),
            )
                .into_response(),
        }
    }
}

fn load_control_proxy_token(path: &Path) -> Result<[u8; 32], String> {
    let raw = fs::read(path).map_err(|error| {
        format!(
            "cannot read control proxy token from {}: {error}",
            path.display()
        )
    })?;
    let encoded = raw
        .strip_suffix(b"\r\n")
        .or_else(|| raw.strip_suffix(b"\n"))
        .unwrap_or(&raw);
    decode_control_proxy_token(encoded).ok_or_else(|| {
        format!(
            "control proxy token in {} must contain exactly 64 hexadecimal characters",
            path.display()
        )
    })
}

fn decode_control_proxy_token(encoded: &[u8]) -> Option<[u8; 32]> {
    if encoded.len() != 64 || !encoded.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut decoded = [0_u8; 32];
    hex::decode_to_slice(encoded, &mut decoded).ok()?;
    Some(decoded)
}

fn verify_control_proxy(state: &AdminState, headers: &HeaderMap) -> Result<(), AdminAuthError> {
    let Some(expected) = state.control_proxy_token.as_ref() else {
        return Ok(());
    };
    let supplied = headers
        .get(CONTROL_PROXY_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| decode_control_proxy_token(value.as_bytes()))
        .ok_or(AdminAuthError::InvalidControlProxy)?;
    if bool::from(expected.ct_eq(&supplied)) {
        Ok(())
    } else {
        Err(AdminAuthError::InvalidControlProxy)
    }
}

fn proxy_role(state: &AdminState, headers: &HeaderMap) -> Result<AdminRole, AdminAuthError> {
    verify_control_proxy(state, headers)?;
    if headers
        .get("x-alvenqis-admin-authenticated")
        .and_then(|value| value.to_str().ok())
        != Some("1")
    {
        return Err(AdminAuthError::MissingAuthentication);
    }
    match headers
        .get("x-alvenqis-admin-role")
        .and_then(|value| value.to_str().ok())
    {
        Some("viewer") => Ok(AdminRole::Viewer),
        Some("operator") => Ok(AdminRole::Operator),
        _ => Err(AdminAuthError::InvalidRole),
    }
}

async fn require_control_proxy(
    State(state): State<AdminState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    match verify_control_proxy(&state, &headers) {
        Ok(()) => next.run(request).await,
        Err(error) => error.response(),
    }
}

async fn require_viewer(
    State(state): State<AdminState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    match proxy_role(&state, &headers) {
        Ok(AdminRole::Viewer | AdminRole::Operator) => next.run(request).await,
        Err(error) => error.response(),
    }
}

async fn require_operator(
    State(state): State<AdminState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    match proxy_role(&state, &headers) {
        Ok(AdminRole::Operator) => next.run(request).await,
        Ok(AdminRole::Viewer) => (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "operator role required"})),
        )
            .into_response(),
        Err(error) => error.response(),
    }
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn admin_session(State(state): State<AdminState>, headers: HeaderMap) -> Json<Value> {
    let role = match proxy_role(&state, &headers) {
        Ok(AdminRole::Operator) => "operator",
        _ => "viewer",
    };
    let actor = headers
        .get("x-alvenqis-admin-user")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("authenticated-proxy");
    Json(json!({"role": role, "actor": actor}))
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
    let controller = state
        .config
        .fleet_enrollment_url
        .clone()
        .ok_or_else(|| internal("fleet_enrollment_url is not configured"))?;
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

async fn revoke_node_certificate(
    State(state): State<AdminState>,
    headers: HeaderMap,
    PathParam(node_id): PathParam<String>,
) -> Result<Json<MutationResponse>, (StatusCode, Json<Value>)> {
    validate_identifier("node_id", &node_id).map_err(bad_request)?;
    let fingerprint = request_fingerprint(&json!({
        "action": "revoke-node-certificate",
        "node_id": node_id,
    }))?;
    let mutation = state
        .store
        .revoke_node_certificate_controlled(&node_id, mutation_context(&headers, &fingerprint)?)
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
        "public_rpc_url": state.config.public_rpc_url,
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
    FleetPki::validate_client_csr(&request.certificate_signing_request_pem).map_err(bad_request)?;
    state
        .store
        .validate_enrollment(&request.invitation_token, &request.report, unix_seconds())
        .map_err(unauthorized)?;
    let issued = state
        .pki
        .as_ref()
        .ok_or_else(|| internal("fleet certificate authority is unavailable"))?
        .issue_client_certificate(&request.certificate_signing_request_pem)
        .map_err(bad_request)?;
    let (node_id, node_token) = state
        .store
        .enroll(
            &request.invitation_token,
            issued.fingerprint_sha1.clone(),
            request.report,
            unix_seconds(),
            state.config.report_interval_seconds,
        )
        .map_err(unauthorized)?;
    Ok(Json(EnrollmentResponse {
        node_id,
        node_token,
        client_certificate_pem: issued.certificate_pem,
        fleet_ca_certificate_pem: issued.ca_certificate_pem,
        certificate_fingerprint_sha1: issued.fingerprint_sha1,
        certificate_expires_at_unix_seconds: issued.expires_at_unix_seconds,
        report_url: state
            .config
            .fleet_report_url
            .clone()
            .ok_or_else(|| internal("fleet_report_url is not configured"))?,
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
    let certificate_fingerprint = verified_client_certificate(&headers)?;
    state
        .store
        .update_report(
            &request.node_id,
            token,
            &certificate_fingerprint,
            request.report,
            state.config.report_interval_seconds,
        )
        .map_err(unauthorized)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn rotate_certificate(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(request): Json<CertificateRotationRequest>,
) -> Result<Json<CertificateRotationResponse>, (StatusCode, Json<Value>)> {
    validate_identifier("node_id", &request.node_id).map_err(bad_request)?;
    let token = bearer_token(&headers).ok_or_else(|| unauthorized("missing bearer token"))?;
    let current_fingerprint = verified_client_certificate(&headers)?;
    state
        .store
        .validate_node_certificate_credentials(&request.node_id, token, &current_fingerprint)
        .map_err(unauthorized)?;
    let issued = state
        .pki
        .as_ref()
        .ok_or_else(|| internal("fleet certificate authority is unavailable"))?
        .issue_client_certificate(&request.certificate_signing_request_pem)
        .map_err(bad_request)?;
    state
        .store
        .stage_certificate_rotation(
            &request.node_id,
            token,
            &current_fingerprint,
            issued.fingerprint_sha1.clone(),
        )
        .map_err(unauthorized)?;
    Ok(Json(CertificateRotationResponse {
        client_certificate_pem: issued.certificate_pem,
        fleet_ca_certificate_pem: issued.ca_certificate_pem,
        certificate_fingerprint_sha1: issued.fingerprint_sha1,
        certificate_expires_at_unix_seconds: issued.expires_at_unix_seconds,
    }))
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

pub async fn rotate_agent_certificate_once(state: &AdminState) -> Result<(), String> {
    let identity = load_or_migrate_agent_identity(&state.config.state_dir)?;
    let mut credentials = identity.credentials.clone();
    let (private_key_pem, csr_pem) = FleetPki::generate_agent_key_and_csr(&state.config.node_name)?;
    let response: CertificateRotationResponse = identity
        .client
        .post(format!(
            "{}/fleet/certificate/rotate",
            credentials.report_url.trim_end_matches('/')
        ))
        .bearer_auth(&credentials.node_token)
        .json(&CertificateRotationRequestOwned {
            node_id: &credentials.node_id,
            certificate_signing_request_pem: &csr_pem,
        })
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    credentials.certificate_fingerprint_sha1 = response.certificate_fingerprint_sha1;
    credentials.certificate_expires_at_unix_seconds = response.certificate_expires_at_unix_seconds;
    install_agent_identity_bundle(
        &state.config.state_dir,
        &private_key_pem,
        &response.client_certificate_pem,
        &response.fleet_ca_certificate_pem,
        &credentials,
    )?;
    Ok(())
}

async fn report_once(state: &AdminState, controller: &str) -> Result<(), String> {
    let report = collect_local_report(state).await;
    if agent_identity_present(&state.config.state_dir) {
        let mut identity = load_or_migrate_agent_identity(&state.config.state_dir)?;
        if identity.credentials.certificate_expires_at_unix_seconds
            <= unix_seconds().saturating_add(7 * 86_400)
        {
            rotate_agent_certificate_once(state).await?;
            identity = load_or_migrate_agent_identity(&state.config.state_dir)?;
        }
        identity
            .client
            .post(format!(
                "{}/fleet/report",
                identity.credentials.report_url.trim_end_matches('/')
            ))
            .bearer_auth(&identity.credentials.node_token)
            .json(&ReportRequestOwned {
                node_id: &identity.credentials.node_id,
                report,
            })
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
    let (private_key_pem, csr_pem) = FleetPki::generate_agent_key_and_csr(&state.config.node_name)?;
    let response: EnrollmentResponse = state
        .client
        .post(format!("{}/fleet/enroll", controller.trim_end_matches('/')))
        .json(&EnrollmentRequestOwned {
            invitation_token: token.trim(),
            certificate_signing_request_pem: &csr_pem,
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
    let credentials = AgentCredentials {
        node_id: response.node_id,
        node_token: response.node_token,
        report_url: response.report_url,
        certificate_fingerprint_sha1: response.certificate_fingerprint_sha1,
        certificate_expires_at_unix_seconds: response.certificate_expires_at_unix_seconds,
    };
    install_agent_identity_bundle(
        &state.config.state_dir,
        &private_key_pem,
        &response.client_certificate_pem,
        &response.fleet_ca_certificate_pem,
        &credentials,
    )?;
    let _ = fs::remove_file(invitation_path);
    Ok(())
}

#[derive(serde::Serialize)]
struct EnrollmentRequestOwned<'a> {
    invitation_token: &'a str,
    certificate_signing_request_pem: &'a str,
    report: NodeReport,
}

#[derive(serde::Serialize)]
struct ReportRequestOwned<'a> {
    node_id: &'a str,
    report: NodeReport,
}

#[derive(serde::Serialize)]
struct CertificateRotationRequestOwned<'a> {
    node_id: &'a str,
    certificate_signing_request_pem: &'a str,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct AgentCredentials {
    node_id: String,
    node_token: String,
    report_url: String,
    certificate_fingerprint_sha1: String,
    certificate_expires_at_unix_seconds: u64,
}

struct AgentIdentity {
    credentials: AgentCredentials,
    client: Client,
}

struct StagedAgentIdentity {
    generation: String,
    directory: PathBuf,
}

fn agent_identity_present(state_dir: &Path) -> bool {
    current_identity_manifest(state_dir).exists()
        || legacy_identity_paths(state_dir)
            .iter()
            .any(|path| path.exists())
}

fn load_or_migrate_agent_identity(state_dir: &Path) -> Result<AgentIdentity, String> {
    let manifest = current_identity_manifest(state_dir);
    if manifest.exists() {
        return load_current_agent_identity(state_dir);
    }
    let legacy = legacy_identity_paths(state_dir);
    let present = legacy.iter().filter(|path| path.exists()).count();
    if present == 0 {
        return Err("agent identity is not initialized".to_owned());
    }
    if present != legacy.len() {
        return Err("legacy agent identity is incomplete; refusing migration".to_owned());
    }
    let credentials: AgentCredentials = serde_json::from_slice(&read_private_file(&legacy[3])?)
        .map_err(|error| format!("invalid legacy agent credentials: {error}"))?;
    install_agent_identity_bundle(
        state_dir,
        &String::from_utf8(read_private_file(&legacy[0])?)
            .map_err(|error| format!("legacy agent key is not UTF-8 PEM: {error}"))?,
        &String::from_utf8(read_private_file(&legacy[1])?)
            .map_err(|error| format!("legacy agent certificate is not UTF-8 PEM: {error}"))?,
        &String::from_utf8(read_private_file(&legacy[2])?)
            .map_err(|error| format!("legacy fleet CA is not UTF-8 PEM: {error}"))?,
        &credentials,
    )
}

fn install_agent_identity_bundle(
    state_dir: &Path,
    private_key_pem: &str,
    certificate_pem: &str,
    ca_certificate_pem: &str,
    credentials: &AgentCredentials,
) -> Result<AgentIdentity, String> {
    let staged = stage_agent_identity_bundle(
        state_dir,
        private_key_pem,
        certificate_pem,
        ca_certificate_pem,
        credentials,
    )?;
    activate_staged_agent_identity(state_dir, &staged)
}

fn stage_agent_identity_bundle(
    state_dir: &Path,
    private_key_pem: &str,
    certificate_pem: &str,
    ca_certificate_pem: &str,
    credentials: &AgentCredentials,
) -> Result<StagedAgentIdentity, String> {
    validate_fingerprint(&credentials.certificate_fingerprint_sha1)?;
    let root = identity_root(state_dir);
    let versions = root.join(AGENT_IDENTITY_VERSIONS_DIRECTORY);
    ensure_private_directory(&root)?;
    ensure_private_directory(&versions)?;
    let generation = format!(
        "v1-{}-{}",
        credentials
            .certificate_fingerprint_sha1
            .to_ascii_lowercase(),
        random_identity_suffix()
    );
    let staging = versions.join(format!(".staging-{generation}"));
    let final_directory = versions.join(&generation);
    ensure_private_directory(&staging)?;
    let result = (|| {
        write_private_file(&staging.join(AGENT_KEY_FILE), private_key_pem.as_bytes())?;
        write_private_file(
            &staging.join(AGENT_CERTIFICATE_FILE),
            certificate_pem.as_bytes(),
        )?;
        write_private_file(&staging.join(AGENT_CA_FILE), ca_certificate_pem.as_bytes())?;
        write_private_json(&staging.join(AGENT_CREDENTIALS_FILE), credentials)?;
        load_agent_identity_directory(&generation, &staging)?;
        sync_directory(&staging)?;
        fs::rename(&staging, &final_directory)
            .map_err(|error| format!("cannot persist staged agent identity: {error}"))?;
        sync_directory(&versions)?;
        load_agent_identity_directory(&generation, &final_directory)?;
        Ok(StagedAgentIdentity {
            generation,
            directory: final_directory,
        })
    })();
    if result.is_err() && staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn activate_staged_agent_identity(
    state_dir: &Path,
    staged: &StagedAgentIdentity,
) -> Result<AgentIdentity, String> {
    validate_generation(&staged.generation)?;
    let root = identity_root(state_dir);
    let expected = root
        .join(AGENT_IDENTITY_VERSIONS_DIRECTORY)
        .join(&staged.generation);
    if staged.directory != expected {
        return Err("staged agent identity is outside the version repository".to_owned());
    }
    load_agent_identity_directory(&staged.generation, &staged.directory)?;
    let temporary_manifest = root.join(format!(".current-{}.tmp", random_identity_suffix()));
    write_private_file(&temporary_manifest, staged.generation.as_bytes())?;
    let manifest = current_identity_manifest(state_dir);
    if let Err(error) = fs::rename(&temporary_manifest, &manifest) {
        let _ = fs::remove_file(&temporary_manifest);
        return Err(format!(
            "cannot atomically activate agent identity: {error}"
        ));
    }
    sync_directory(&root)?;
    load_current_agent_identity(state_dir)
}

fn load_current_agent_identity(state_dir: &Path) -> Result<AgentIdentity, String> {
    let generation = current_agent_identity_generation(state_dir)?;
    load_agent_identity_directory(
        &generation,
        &identity_root(state_dir)
            .join(AGENT_IDENTITY_VERSIONS_DIRECTORY)
            .join(&generation),
    )
}

fn current_agent_identity_generation(state_dir: &Path) -> Result<String, String> {
    let manifest = read_private_file(&current_identity_manifest(state_dir))?;
    let generation = std::str::from_utf8(&manifest)
        .map_err(|error| format!("agent identity manifest is not UTF-8: {error}"))?
        .trim();
    validate_generation(generation)?;
    Ok(generation.to_owned())
}

fn load_agent_identity_directory(
    generation: &str,
    directory: &Path,
) -> Result<AgentIdentity, String> {
    validate_generation(generation)?;
    let metadata = fs::symlink_metadata(directory)
        .map_err(|error| format!("cannot inspect agent identity directory: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("agent identity version must be a real directory".to_owned());
    }
    let certificate = read_private_file(&directory.join(AGENT_CERTIFICATE_FILE))?;
    let credentials: AgentCredentials =
        serde_json::from_slice(&read_private_file(&directory.join(AGENT_CREDENTIALS_FILE))?)
            .map_err(|error| format!("invalid agent credentials: {error}"))?;
    validate_agent_credentials(&credentials, &certificate)?;
    let client = agent_mtls_client(directory)?;
    Ok(AgentIdentity {
        credentials,
        client,
    })
}

fn agent_mtls_client(identity_directory: &Path) -> Result<Client, String> {
    let certificate = read_private_file(&identity_directory.join(AGENT_CERTIFICATE_FILE))?;
    let private_key = read_private_file(&identity_directory.join(AGENT_KEY_FILE))?;
    let mut identity_pem = certificate;
    identity_pem.extend_from_slice(&private_key);
    let identity = reqwest::Identity::from_pem(&identity_pem)
        .map_err(|error| format!("invalid agent TLS identity: {error}"))?;
    let ca = reqwest::Certificate::from_pem(&read_private_file(
        &identity_directory.join(AGENT_CA_FILE),
    )?)
    .map_err(|error| format!("invalid fleet CA certificate: {error}"))?;
    Client::builder()
        .timeout(Duration::from_secs(10))
        .identity(identity)
        .add_root_certificate(ca)
        .build()
        .map_err(|error| error.to_string())
}

fn validate_agent_credentials(
    credentials: &AgentCredentials,
    certificate_pem: &[u8],
) -> Result<(), String> {
    if credentials.node_id.is_empty() || credentials.node_token.is_empty() {
        return Err("agent credentials are missing node identity or token".to_owned());
    }
    if !credentials.report_url.starts_with("https://")
        || credentials.report_url.contains('@')
        || credentials.report_url.chars().any(char::is_whitespace)
    {
        return Err("agent report URL must use HTTPS without credentials".to_owned());
    }
    validate_fingerprint(&credentials.certificate_fingerprint_sha1)?;
    let certificate = CertificateDer::from_pem_slice(certificate_pem)
        .map_err(|error| format!("invalid agent certificate PEM: {error}"))?;
    let actual = hex::encode_upper(Sha1::digest(certificate.as_ref()));
    if !actual.eq_ignore_ascii_case(&credentials.certificate_fingerprint_sha1) {
        return Err("agent certificate fingerprint does not match credentials".to_owned());
    }
    Ok(())
}

fn validate_fingerprint(value: &str) -> Result<(), String> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "agent certificate fingerprint must contain 40 hexadecimal characters".to_owned(),
        );
    }
    Ok(())
}

fn validate_generation(value: &str) -> Result<(), String> {
    if !(45..=96).contains(&value.len())
        || !value.starts_with("v1-")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("agent identity manifest contains an invalid generation".to_owned());
    }
    Ok(())
}

fn identity_root(state_dir: &Path) -> PathBuf {
    state_dir.join(AGENT_IDENTITY_DIRECTORY)
}

fn current_identity_manifest(state_dir: &Path) -> PathBuf {
    identity_root(state_dir).join(AGENT_IDENTITY_CURRENT_MANIFEST)
}

fn legacy_identity_paths(state_dir: &Path) -> [PathBuf; 4] {
    [
        state_dir.join(LEGACY_AGENT_KEY_FILE),
        state_dir.join(LEGACY_AGENT_CERTIFICATE_FILE),
        state_dir.join(LEGACY_AGENT_CA_FILE),
        state_dir.join(LEGACY_AGENT_CREDENTIALS_FILE),
    ]
}

fn random_identity_suffix() -> String {
    let mut bytes = [0_u8; 12];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(format!(
                    "private identity path is not a directory: {}",
                    path.display()
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(path)
                    .map_err(|error| error.to_string())?;
            }
            #[cfg(not(unix))]
            fs::create_dir(path).map_err(|error| error.to_string())?;
        }
        Err(error) => return Err(error.to_string()),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn read_private_file(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "private identity path is not a regular file: {}",
            path.display()
        ));
    }
    if metadata.len() > MAX_IDENTITY_FILE_BYTES {
        return Err(format!(
            "private identity file is too large: {}",
            path.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "private identity file permissions are too broad: {}",
                path.display()
            ));
        }
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(bytes)
}

fn write_private_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    write_private_file(
        path,
        &serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true).mode(0o600);
        let mut file = options.open(path).map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
    }
    #[cfg(not(unix))]
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| error.to_string())?;
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
    format!(
        "set -Eeuo pipefail\nread -r -s -p 'Enrollment token: ' ALVENQIS_ENROLLMENT_TOKEN\nprintf '\\n'\n{docker_install}\ninstall -d -m 0755 /opt/alvenqis-agent\ntest -z \"$(ls -A /opt/alvenqis-agent)\" || {{ echo 'Refusing to overwrite /opt/alvenqis-agent' >&2; exit 73; }}\ncurl -fsSL {bundle} -o /tmp/alvenqis-setup-external.tar.gz\ncurl -fsSL {bundle}.sha256 -o /tmp/alvenqis-setup-external.tar.gz.sha256\ncd /tmp\nsha256sum -c alvenqis-setup-external.tar.gz.sha256\ntar -xzf alvenqis-setup-external.tar.gz -C /opt/alvenqis-agent\ncd /opt/alvenqis-agent/alvenqis-release/alvenqis-setup-external\nprintf '%s\\n' \"$ALVENQIS_ENROLLMENT_TOKEN\" | ./scripts/enroll-docker-node.sh --node-name {node} --p2p-host {domain} --email {email} --controller-url {controller} --enrollment-token-stdin --seed {seed} --release-bundle-url {bundle} --role {role} {pool_flag}\nunset ALVENQIS_ENROLLMENT_TOKEN\n./scripts/compose.sh ps\ninstall -m 0600 /dev/null /root/alvenqis-access.txt\n{{ printf 'Node: %s\\nRole: %s\\nController: %s\\nControl URL: %s\\n' {node} {role} {controller} {control_url}; if test -s state/secrets/admin_password; then printf 'Admin password: '; cat state/secrets/admin_password; fi; }} > /root/alvenqis-access.txt\nchmod 0600 /root/alvenqis-access.txt\nprintf 'Credentials saved to /root/alvenqis-access.txt (mode 0600)\\n'\n",
        docker_install = docker_install_commands(),
        bundle = shell_arg(&state.config.release_bundle_url),
        node = shell_arg(&request.node_name),
        domain = shell_arg(&request.advertise_host),
        email = shell_arg(&request.acme_email),
        controller = shell_arg(controller),
        seed = shell_arg(seed),
        role = shell_arg(request.role.as_str()),
        pool_flag = if matches!(request.role, DeploymentRole::Stratum | DeploymentRole::FullStack) { "--enable-pool" } else { "" },
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
    format!(
        "set -Eeuo pipefail\nread -r -s -p 'Enrollment token: ' ALVENQIS_ENROLLMENT_TOKEN\nprintf '\\n'\n{docker_install}\ninstall -d -m 0755 /opt/alvenqis-agent\ntest -z \"$(ls -A /opt/alvenqis-agent)\" || {{ echo 'Refusing to overwrite /opt/alvenqis-agent' >&2; exit 73; }}\ncurl -fsSL {bundle} -o /tmp/alvenqis-setup-external.tar.gz\ncurl -fsSL {bundle}.sha256 -o /tmp/alvenqis-setup-external.tar.gz.sha256\ncd /tmp && sha256sum -c alvenqis-setup-external.tar.gz.sha256\ntar -xzf /tmp/alvenqis-setup-external.tar.gz -C /opt/alvenqis-agent\ncd /opt/alvenqis-agent/alvenqis-release/alvenqis-setup-external\nprintf '%s\\n' \"$ALVENQIS_ENROLLMENT_TOKEN\" | ./scripts/enroll-docker-node.sh --node-name {node} --p2p-host {host} --email {email} --controller-url {controller} --enrollment-token-stdin --seed {seed} --release-bundle-url {bundle} --role {role} {pool_flag}\nunset ALVENQIS_ENROLLMENT_TOKEN\n./scripts/compose.sh ps\ninstall -m 0600 /dev/null /root/alvenqis-access.txt\n{{ printf 'Node: %s\\nRole: %s\\nController: %s\\n' {node} {role} {controller}; if test -s state/secrets/admin_password; then printf 'Admin password: '; cat state/secrets/admin_password; fi; }} > /root/alvenqis-access.txt\nchmod 0600 /root/alvenqis-access.txt\n",
        docker_install = docker_install_commands(),
        bundle = shell_arg(bundle),
        node = shell_arg(&request.node_name),
        host = shell_arg(&request.advertise_host),
        email = shell_arg(&request.acme_email),
        controller = shell_arg(controller),
        seed = shell_arg(seed),
        role = shell_arg(request.role.as_str()),
        pool_flag = if matches!(request.role, DeploymentRole::Stratum | DeploymentRole::FullStack) { "--enable-pool" } else { "" },
    )
}

fn docker_install_commands() -> &'static str {
    "export DEBIAN_FRONTEND=noninteractive\napt-get update\napt-get install -y ca-certificates curl openssl python3\ninstall -m 0755 -d /etc/apt/keyrings\ncurl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc\nchmod a+r /etc/apt/keyrings/docker.asc\n. /etc/os-release\necho \"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${UBUNTU_CODENAME:-$VERSION_CODENAME} stable\" > /etc/apt/sources.list.d/docker.list\napt-get update\napt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin\nsystemctl enable --now docker"
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

fn verified_client_certificate(headers: &HeaderMap) -> Result<String, (StatusCode, Json<Value>)> {
    if headers
        .get("x-alvenqis-client-verify")
        .and_then(|value| value.to_str().ok())
        != Some("SUCCESS")
    {
        return Err(unauthorized("verified agent client certificate required"));
    }
    let fingerprint = headers
        .get("x-alvenqis-client-fingerprint")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| unauthorized("client certificate fingerprint is missing"))?
        .replace(':', "")
        .to_ascii_uppercase();
    if fingerprint.len() != 40
        || !fingerprint
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(unauthorized("client certificate fingerprint is invalid"));
    }
    Ok(fingerprint)
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

    const TEST_CONTROL_PROXY_TOKEN: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const WRONG_CONTROL_PROXY_TOKEN: &str =
        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

    fn test_config(state_dir: &Path) -> AdminConfig {
        AdminConfig {
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 10788,
            network_id: "alvenqis-mainnet-candidate".to_owned(),
            status_label: "test".to_owned(),
            node_name: "controller".to_owned(),
            advertise_host: "controller.example.org".to_owned(),
            p2p_port: 20787,
            local_rpc_url: "http://127.0.0.1:9".to_owned(),
            state_dir: PathBuf::from(state_dir),
            release_bundle_url: "https://example.org/release.tar.gz".to_owned(),
            controller_url: None,
            public_rpc_url: Some("https://rpc.example.org".to_owned()),
            fleet_enrollment_url: Some("https://fleet.example.org".to_owned()),
            fleet_report_url: Some("https://fleet.example.org:10443".to_owned()),
            fleet_server_name: Some("fleet.example.org".to_owned()),
            report_interval_seconds: 15,
            invitation_ttl_seconds: 900,
        }
    }

    fn test_state() -> (AdminState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = test_config(dir.path());
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        (AdminState::new(config, store).expect("state"), dir)
    }

    fn docker_test_state() -> (AdminState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let token_path = dir.path().join("control_proxy_token");
        write_private_file(
            &token_path,
            format!("{TEST_CONTROL_PROXY_TOKEN}\n").as_bytes(),
        )
        .expect("control proxy token");
        let config = test_config(dir.path());
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        (
            AdminState::new_for_mode(config, store, true, &token_path).expect("docker state"),
            dir,
        )
    }

    fn identity_material(
        pki: &FleetPki,
        node_name: &str,
    ) -> (String, String, String, AgentCredentials) {
        let (private_key, csr) =
            FleetPki::generate_agent_key_and_csr(node_name).expect("agent key and csr");
        let issued = pki
            .issue_client_certificate(&csr)
            .expect("issued client certificate");
        let credentials = AgentCredentials {
            node_id: format!("{node_name}-id"),
            node_token: format!("{node_name}-token"),
            report_url: "https://fleet.example.org:10443".to_owned(),
            certificate_fingerprint_sha1: issued.fingerprint_sha1,
            certificate_expires_at_unix_seconds: issued.expires_at_unix_seconds,
        };
        (
            private_key,
            issued.certificate_pem,
            issued.ca_certificate_pem,
            credentials,
        )
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

    #[test]
    fn docker_mode_fails_closed_for_missing_or_invalid_proxy_secret() {
        let dir = tempfile::tempdir().expect("tempdir");
        let token_path = dir.path().join("control_proxy_token");
        let config = test_config(dir.path());
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");

        assert!(
            AdminState::new_for_mode(config.clone(), store.clone(), true, &token_path).is_err()
        );
        write_private_file(&token_path, b"not-a-64-character-hex-token\n")
            .expect("invalid token file");
        assert!(AdminState::new_for_mode(config, store, true, &token_path).is_err());
    }

    #[tokio::test]
    async fn docker_admin_headers_are_rejected_without_valid_proxy_token() {
        let (state, _dir) = docker_test_state();
        for token in [None, Some(WRONG_CONTROL_PROXY_TOKEN)] {
            let mut request = HttpRequest::builder()
                .uri("/api/audit")
                .header("x-alvenqis-admin-authenticated", "1")
                .header("x-alvenqis-admin-role", "viewer");
            if let Some(token) = token {
                request = request.header(CONTROL_PROXY_TOKEN_HEADER, token);
            }
            let response = router(state.clone())
                .oneshot(request.body(Body::empty()).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        let response = router(state)
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/audit")
                    .header("x-alvenqis-admin-authenticated", "1")
                    .header("x-alvenqis-admin-role", "viewer")
                    .header(CONTROL_PROXY_TOKEN_HEADER, TEST_CONTROL_PROXY_TOKEN)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn docker_fleet_routes_reject_direct_header_forgery() {
        let (state, _dir) = docker_test_state();
        let forged_report = HttpRequest::builder()
            .method("POST")
            .uri("/fleet/report")
            .header("content-type", "application/json")
            .header("authorization", "Bearer forged-node-token")
            .header("x-alvenqis-client-verify", "SUCCESS")
            .header(
                "x-alvenqis-client-fingerprint",
                "0123456789ABCDEF0123456789ABCDEF01234567",
            )
            .body(Body::from("{}"))
            .expect("request");
        let response = router(state.clone())
            .oneshot(forged_report)
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let direct_enrollment = HttpRequest::builder()
            .method("POST")
            .uri("/fleet/enroll")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .expect("request");
        let response = router(state.clone())
            .oneshot(direct_enrollment)
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        for route in ["/fleet/enroll", "/fleet/report"] {
            let response = router(state.clone())
                .oneshot(
                    HttpRequest::builder()
                        .method("POST")
                        .uri(route)
                        .header("content-type", "application/json")
                        .header(CONTROL_PROXY_TOKEN_HEADER, TEST_CONTROL_PROXY_TOKEN)
                        .body(Body::from("{}"))
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[tokio::test]
    async fn authenticated_request_without_role_is_rejected() {
        let (state, _dir) = test_state();
        let response = router(state)
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/audit")
                    .header("x-alvenqis-admin-authenticated", "1")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn client_certificate_headers_must_be_proxy_verified_and_well_formed() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-alvenqis-client-fingerprint",
            "01:23:45:67:89:AB:CD:EF:01:23:45:67:89:AB:CD:EF:01:23:45:67"
                .parse()
                .expect("header"),
        );
        assert!(verified_client_certificate(&headers).is_err());
        headers.insert(
            "x-alvenqis-client-verify",
            "FAILED:self-signed certificate".parse().expect("header"),
        );
        assert!(verified_client_certificate(&headers).is_err());
        headers.insert(
            "x-alvenqis-client-verify",
            "SUCCESS".parse().expect("header"),
        );
        assert_eq!(
            verified_client_certificate(&headers).expect("verified certificate"),
            "0123456789ABCDEF0123456789ABCDEF01234567"
        );
    }

    #[tokio::test]
    async fn viewer_can_read_but_cannot_mutate() {
        let (state, _dir) = test_state();
        let read_response = router(state.clone())
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/audit")
                    .header("x-alvenqis-admin-authenticated", "1")
                    .header("x-alvenqis-admin-role", "viewer")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(read_response.status(), StatusCode::OK);

        let write_response = router(state)
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/bootstrap/manifests")
                    .header("content-type", "application/json")
                    .header("x-alvenqis-admin-authenticated", "1")
                    .header("x-alvenqis-admin-role", "viewer")
                    .body(Body::from(
                        r#"{"role":"node","node_name":"node-1","advertise_host":"node1.example.org","acme_email":"ops@example.org"}"#,
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(write_response.status(), StatusCode::FORBIDDEN);
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
                    .header("x-alvenqis-admin-role", "operator")
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
        assert!(command.contains("--role 'stratum'"));
        assert!(command.contains("--enable-pool"));
        assert!(command.contains("./scripts/compose.sh ps"));
        assert!(command.contains("/root/alvenqis-access.txt"));
        assert!(command.contains("chmod 0600"));
        assert!(command.contains("--enrollment-token-stdin"));
        assert!(!command.contains("--enrollment-token "));
        assert!(!command.contains("eval "));
    }

    #[test]
    fn staged_identity_is_invisible_until_single_manifest_switch() {
        let directory = tempfile::tempdir().expect("tempdir");
        let controller_dir = directory.path().join("controller");
        let agent_dir = directory.path().join("agent");
        fs::create_dir(&agent_dir).expect("agent directory");
        let pki = FleetPki::load_or_initialize(&controller_dir, "fleet.example.org")
            .expect("controller pki");
        let (first_key, first_cert, first_ca, first_credentials) =
            identity_material(&pki, "agent-first");
        install_agent_identity_bundle(
            &agent_dir,
            &first_key,
            &first_cert,
            &first_ca,
            &first_credentials,
        )
        .expect("first identity");
        let first_generation =
            current_agent_identity_generation(&agent_dir).expect("first generation");

        let (next_key, next_cert, next_ca, next_credentials) =
            identity_material(&pki, "agent-next");
        let staged = stage_agent_identity_bundle(
            &agent_dir,
            &next_key,
            &next_cert,
            &next_ca,
            &next_credentials,
        )
        .expect("staged identity");

        // Simulate a crash after the complete version directory was persisted
        // but before the current manifest was replaced.
        let recovered = load_or_migrate_agent_identity(&agent_dir).expect("old identity recovery");
        assert_eq!(
            current_agent_identity_generation(&agent_dir).expect("recovered generation"),
            first_generation
        );
        assert_eq!(
            recovered.credentials.certificate_fingerprint_sha1,
            first_credentials.certificate_fingerprint_sha1
        );

        let activated = activate_staged_agent_identity(&agent_dir, &staged)
            .expect("single manifest activation");
        assert_eq!(
            current_agent_identity_generation(&agent_dir).expect("activated generation"),
            staged.generation
        );
        assert_eq!(
            activated.credentials.certificate_fingerprint_sha1,
            next_credentials.certificate_fingerprint_sha1
        );
    }

    #[test]
    fn mismatched_key_and_certificate_never_replace_current_identity() {
        let directory = tempfile::tempdir().expect("tempdir");
        let controller_dir = directory.path().join("controller");
        let agent_dir = directory.path().join("agent");
        fs::create_dir(&agent_dir).expect("agent directory");
        let pki = FleetPki::load_or_initialize(&controller_dir, "fleet.example.org")
            .expect("controller pki");
        let (first_key, first_cert, first_ca, first_credentials) =
            identity_material(&pki, "agent-first");
        install_agent_identity_bundle(
            &agent_dir,
            &first_key,
            &first_cert,
            &first_ca,
            &first_credentials,
        )
        .expect("first identity");
        let (_next_key, next_cert, next_ca, next_credentials) =
            identity_material(&pki, "agent-next");

        assert!(stage_agent_identity_bundle(
            &agent_dir,
            &first_key,
            &next_cert,
            &next_ca,
            &next_credentials,
        )
        .is_err());
        let current = load_or_migrate_agent_identity(&agent_dir).expect("current identity");
        assert_eq!(
            current.credentials.certificate_fingerprint_sha1,
            first_credentials.certificate_fingerprint_sha1
        );
    }

    #[test]
    fn complete_legacy_identity_is_migrated_without_deleting_source_files() {
        let directory = tempfile::tempdir().expect("tempdir");
        let controller_dir = directory.path().join("controller");
        let agent_dir = directory.path().join("agent");
        fs::create_dir(&agent_dir).expect("agent directory");
        let pki = FleetPki::load_or_initialize(&controller_dir, "fleet.example.org")
            .expect("controller pki");
        let (key, cert, ca, credentials) = identity_material(&pki, "legacy-agent");
        write_private_file(&agent_dir.join("agent-client.key.pem"), key.as_bytes())
            .expect("legacy key");
        write_private_file(&agent_dir.join("agent-client.crt.pem"), cert.as_bytes())
            .expect("legacy certificate");
        write_private_file(&agent_dir.join("fleet-ca.crt.pem"), ca.as_bytes()).expect("legacy ca");
        write_private_json(&agent_dir.join("agent-credentials.json"), &credentials)
            .expect("legacy credentials");

        let migrated = load_or_migrate_agent_identity(&agent_dir).expect("migrated identity");
        assert_eq!(
            migrated.credentials.certificate_fingerprint_sha1,
            credentials.certificate_fingerprint_sha1
        );
        assert!(agent_dir.join("agent-client.key.pem").is_file());
        assert!(agent_dir.join("agent-client.crt.pem").is_file());
        assert!(agent_dir.join("fleet-ca.crt.pem").is_file());
        assert!(agent_dir.join("agent-credentials.json").is_file());
        assert_eq!(
            load_or_migrate_agent_identity(&agent_dir)
                .expect("repeat load")
                .credentials
                .certificate_fingerprint_sha1,
            migrated.credentials.certificate_fingerprint_sha1
        );
    }

    #[test]
    fn incomplete_legacy_identity_is_rejected_without_creating_a_manifest() {
        let directory = tempfile::tempdir().expect("tempdir");
        let credentials = AgentCredentials {
            node_id: "legacy-node".to_owned(),
            node_token: "legacy-token".to_owned(),
            report_url: "https://fleet.example.org:10443".to_owned(),
            certificate_fingerprint_sha1: "0123456789ABCDEF0123456789ABCDEF01234567".to_owned(),
            certificate_expires_at_unix_seconds: 1,
        };
        write_private_json(
            &directory.path().join("agent-credentials.json"),
            &credentials,
        )
        .expect("partial legacy credentials");

        assert!(load_or_migrate_agent_identity(directory.path()).is_err());
        assert!(!directory
            .path()
            .join("agent-identity")
            .join("current")
            .exists());
    }
}
