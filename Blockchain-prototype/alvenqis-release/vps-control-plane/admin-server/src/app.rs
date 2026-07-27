use crate::config::AdminConfig;
use crate::models::{
    AdminOverview, EnrollmentRequest, EnrollmentResponse, EnrollmentStep, FleetTopology,
    InvitationRequest, InvitationResponse, InvitationView, NodeDetailView, NodeReport,
    ReportRequest, ServiceStates,
};
use crate::store::{node_view, FleetStore};
use axum::extract::{Path as PathParam, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const INDEX_HTML: &str = include_str!("../static/index.html");
const APP_JS: &str = include_str!("../static/app.js");
const STYLES_CSS: &str = include_str!("../static/styles.css");
const LOGO_PNG: &[u8] = include_bytes!("../static/logo.png");
const LOGO_MARK_PNG: &[u8] = include_bytes!("../static/logo-mark.png");

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
        .route("/api/topology", get(topology))
        .route("/api/nodes", get(topology))
        .route("/api/nodes/:node_id", get(node_detail).delete(remove_node))
        .route(
            "/api/invitations",
            get(list_invitations).post(create_invitation),
        )
        .route("/api/invitations/:invitation_id", delete(revoke_invitation))
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
        "service": "alvenqis-vps-admin",
        "network_id": state.config.network_id,
        "status_label": state.config.status_label,
        "exposure": "loopback-only; authenticate at reverse proxy"
    }))
}

async fn overview(
    State(state): State<AdminState>,
) -> Result<Json<AdminOverview>, (StatusCode, Json<Value>)> {
    let local = collect_local_report(&state).await;
    let topology = build_topology(&state, Some(local.clone())).map_err(internal)?;
    Ok(Json(AdminOverview {
        mode: "Mainnet Candidate / VPS Fleet Prototype",
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

async fn create_invitation(
    State(state): State<AdminState>,
    Json(request): Json<InvitationRequest>,
) -> Result<Json<InvitationResponse>, (StatusCode, Json<Value>)> {
    validate_label("node_name", &request.node_name).map_err(bad_request)?;
    validate_host(&request.advertise_host).map_err(bad_request)?;
    if let Some(domain) = &request.admin_domain {
        validate_host(domain).map_err(bad_request)?;
    }
    validate_email(&request.acme_email).map_err(bad_request)?;
    let now = unix_seconds();
    let invite = state
        .store
        .create_invitation(
            request.node_name.clone(),
            request.advertise_host.clone(),
            now,
            state.config.invitation_ttl_seconds,
        )
        .map_err(internal)?;
    let controller = request
        .admin_domain
        .as_deref()
        .map(|domain| format!("https://{domain}"))
        .unwrap_or_else(|| format!("https://{}", state.config.advertise_host));
    let seed = format!(
        "/dns4/{}/tcp/{}",
        state.config.advertise_host, state.config.p2p_port
    );
    if !docker_mode() {
        return Err(bad_request(
            "VPS enrollment is available only in Docker deployment mode",
        ));
    }
    if state.config.release_bundle_url.trim().is_empty() {
        return Err(bad_request(
            "Docker enrollment requires an immutable release_bundle_url",
        ));
    }
    // Integrity-only verification today (sha256 from the same download channel).
    // TODO(security/CR-H13): same authenticity layer as desktop auto-update —
    // minisign/GPG detached signature (or GitHub release attestations) verified
    // against a pinned public key before extract. Do not claim resolved until shipped.
    let install_command = format!(
            "set -euo pipefail\ninstall -d -m 0755 /opt/alvenqis-agent\ntest -z \"$(ls -A /opt/alvenqis-agent)\" || {{ echo 'Refusing to overwrite /opt/alvenqis-agent' >&2; exit 73; }}\ncurl -fsSL {bundle} -o /tmp/alvenqis-docker-control-plane.tar.gz\ncurl -fsSL {bundle}.sha256 -o /tmp/alvenqis-docker-control-plane.tar.gz.sha256\ncd /tmp\nsha256sum -c alvenqis-docker-control-plane.tar.gz.sha256\n# TODO(CR-H13): verify detached signature / attestation here (shared with desktop updates)\ntar -xzf alvenqis-docker-control-plane.tar.gz -C /opt/alvenqis-agent\ncd /opt/alvenqis-agent/alvenqis-release/vps-control-plane\n./scripts/enroll-docker-node.sh --node-name {node} --p2p-host {domain} --email {email} --controller-url {controller} --enrollment-token {token} --seed {seed} --release-bundle-url {bundle}\n",
            bundle = shell_arg(&state.config.release_bundle_url),
            node = shell_arg(&request.node_name),
            domain = shell_arg(&request.advertise_host),
            email = shell_arg(&request.acme_email),
            controller = shell_arg(&controller),
            token = shell_arg(&invite.token),
            seed = shell_arg(&seed),
        );
    let steps = vec![
        EnrollmentStep {
            title: "1 - Prepare DNS and firewall".into(),
            detail: format!(
                "Point DNS for {} to the new host and open TCP {} for P2P.",
                request.advertise_host, state.config.p2p_port
            ),
            code: None,
        },
        EnrollmentStep {
            title: "2 - SSH to the new host".into(),
            detail: "Use a clean Ubuntu 24.04 machine. Run the install script as root (sudo).".into(),
            code: Some(format!("ssh root@{}", request.advertise_host)),
        },
        EnrollmentStep {
            title: "3 - Run one-time enrollment install".into(),
            detail: format!(
                "Token expires at {}. Single-use only. Grants fleet telemetry, not consensus privilege.",
                format_unix(invite.expires_at_unix_seconds)
            ),
            code: Some(install_command.clone()),
        },
        EnrollmentStep {
            title: "4 - Verify in this panel".into(),
            detail: "Within about 15-45 seconds the node should appear ONLINE under Nodes / Topology after its first report.".into(),
            code: None,
        },
    ];
    Ok(Json(InvitationResponse {
        invitation_id: invite.id,
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
    PathParam(invitation_id): PathParam<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    state
        .store
        .revoke_invitation(&invitation_id)
        .map_err(bad_request)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_node(
    State(state): State<AdminState>,
    PathParam(node_id): PathParam<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    state.store.remove_node(&node_id).map_err(bad_request)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn node_detail(
    State(state): State<AdminState>,
    PathParam(node_id): PathParam<String>,
) -> Result<Json<NodeDetailView>, (StatusCode, Json<Value>)> {
    let now = unix_seconds();
    if node_id == "local-controller" {
        let local = collect_local_report(&state).await;
        return Ok(Json(NodeDetailView {
            node: node_view("local-controller", &local, now),
            report: local,
            is_local_controller: true,
        }));
    }
    let report = state
        .store
        .get_node_report(&node_id)
        .map_err(internal)?
        .ok_or_else(|| bad_request("node not found"))?;
    Ok(Json(NodeDetailView {
        node: node_view(&node_id, &report, now),
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

fn format_unix(ts: u64) -> String {
    // Keep response self-contained; UI also formats locally.
    format!("unix:{ts}")
}

async fn enroll(
    State(state): State<AdminState>,
    Json(request): Json<EnrollmentRequest>,
) -> Result<Json<EnrollmentResponse>, (StatusCode, Json<Value>)> {
    validate_report(&state.config, &request.report).map_err(bad_request)?;
    let (node_id, node_token) = state
        .store
        .enroll(&request.invitation_token, request.report, unix_seconds())
        .map_err(unauthorized)?;
    Ok(Json(EnrollmentResponse {
        node_id,
        node_token,
    }))
}

async fn report(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(request): Json<ReportRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    validate_report(&state.config, &request.report).map_err(bad_request)?;
    let token = bearer_token(&headers).ok_or_else(|| unauthorized("missing bearer token"))?;
    state
        .store
        .update_report(&request.node_id, token, request.report)
        .map_err(unauthorized)?;
    Ok(StatusCode::NO_CONTENT)
}

fn build_topology(state: &AdminState, local: Option<NodeReport>) -> Result<FleetTopology, String> {
    let now = unix_seconds();
    let mut nodes = state.store.node_views(now)?;
    if let Some(report) = local {
        nodes.insert(0, node_view("local-controller", &report, now));
    }
    let online_node_count = nodes.iter().filter(|node| node.online).count();
    Ok(FleetTopology {
        mode: "Observed fleet telemetry; not a global network census",
        network_id: state.config.network_id.clone(),
        generated_at_unix_seconds: now,
        registered_node_count: nodes.len(),
        online_node_count,
        direct_validated_connections: nodes.iter().map(|node| node.validating_peers).sum(),
        // Multiple VPS nodes can observe the same miners, so summing would double count.
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
    let status_url = format!("{base}/status");
    let sync_url = format!("{base}/sync/status");
    let p2p_url = format!("{base}/p2p/status");
    let mempool_url = format!("{base}/mempool/status");
    let indexer_url = format!("{base}/indexer/status");
    let (status, sync, p2p, mempool, indexer) = tokio::join!(
        get_json(&state.client, &status_url),
        get_json(&state.client, &sync_url),
        get_json(&state.client, &p2p_url),
        get_json(&state.client, &mempool_url),
        get_json(&state.client, &indexer_url),
    );
    let services = if docker_mode() {
        ServiceStates {
            node: json_service_state(&p2p),
            rpc: json_service_state(&status),
            indexer_timer: json_service_state(&indexer),
            admin: "active".to_owned(),
        }
    } else {
        service_states()
    };
    NodeReport {
        network_id: state.config.network_id.clone(),
        node_name: state.config.node_name.clone(),
        advertise_host: state.config.advertise_host.clone(),
        p2p_multiaddr: format!(
            "/dns4/{}/tcp/{}",
            state.config.advertise_host, state.config.p2p_port
        ),
        reported_at_unix_seconds: unix_seconds(),
        services,
        status,
        sync,
        p2p,
        mempool,
        indexer,
    }
}

async fn get_json(client: &Client, url: &str) -> Value {
    match client.get(url).send().await {
        Ok(response) if response.status().is_success() => response
            .json()
            .await
            .unwrap_or_else(|error| json!({"error": error.to_string()})),
        Ok(response) => json!({"error": format!("HTTP {}", response.status())}),
        Err(error) => json!({"error": error.to_string()}),
    }
}

fn service_states() -> ServiceStates {
    ServiceStates {
        node: systemd_state("alvenqis-node"),
        rpc: systemd_state("alvenqis-rpc"),
        indexer_timer: systemd_state("alvenqis-indexer-refresh.timer"),
        admin: systemd_state("alvenqis-vps-admin"),
    }
}

fn docker_mode() -> bool {
    env::var("ALVENQIS_DEPLOYMENT_MODE").is_ok_and(|value| value.eq_ignore_ascii_case("docker"))
}

fn json_service_state(payload: &Value) -> String {
    if payload.get("error").is_some() {
        "inactive".to_owned()
    } else {
        "active".to_owned()
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
    Ok(())
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

fn validate_host(value: &str) -> Result<(), String> {
    validate_label("host", value)
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

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
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

fn internal(message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": message.to_string()})),
    )
}
