use crate::config::{HostConfig, PkiConfig};
use async_trait::async_trait;
use pingora::listeners::tls::TlsSettings;
use pingora::listeners::TlsAccept;
use pingora::protocols::tls::TlsRef;
use pingora::proxy::Session;
use pingora::tls::ssl::{NameType, SslFiletype, SslOptions, SslVerifyMode, SslVersion};
use pingora::tls::x509::{X509Name, X509};
use pingora::Result as PingoraResult;
use sha1::{Digest as _, Sha1};
use std::any::Any;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientCertificateInfo {
    pub fingerprint_sha1: String,
    pub sni: Option<String>,
}

#[derive(Debug)]
struct FleetTlsCallbacks;

#[async_trait]
impl TlsAccept for FleetTlsCallbacks {
    async fn handshake_complete_callback(
        &self,
        tls_ref: &TlsRef,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        let certificate = tls_ref.peer_certificate()?;
        let der = certificate.to_der().ok()?;
        let fingerprint_sha1 = hex::encode_upper(Sha1::digest(der));
        let sni = tls_ref
            .servername(NameType::HOST_NAME)
            .map(str::to_ascii_lowercase);
        Some(Arc::new(ClientCertificateInfo {
            fingerprint_sha1,
            sni,
        }))
    }
}

pub fn validate_pki(hosts: &HostConfig, pki: &PkiConfig) -> Result<(), String> {
    if pki.forbidden_ca_private_key.exists() {
        return Err("fleet CA private key must never be mounted in the gateway".to_owned());
    }
    for (label, path) in [
        ("fleet CA certificate", &pki.ca_certificate),
        ("fleet server certificate", &pki.server_certificate),
        ("fleet server private key", &pki.server_private_key),
    ] {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("{label} is unavailable at {}: {error}", path.display()))?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(format!("{label} must be a non-empty regular file"));
        }
    }
    let certificate_pem = fs::read(&pki.server_certificate)
        .map_err(|error| format!("unable to read fleet server certificate: {error}"))?;
    let certificate = X509::from_pem(&certificate_pem)
        .map_err(|error| format!("fleet server certificate is invalid: {error}"))?;
    let san_matches = certificate.subject_alt_names().is_some_and(|names| {
        names
            .iter()
            .filter_map(|name| name.dnsname())
            .any(|name| name.eq_ignore_ascii_case(&hosts.fleet_mtls))
    });
    if !san_matches {
        return Err(format!(
            "fleet server certificate SAN does not contain {}",
            hosts.fleet_mtls
        ));
    }
    Ok(())
}

pub fn build_mtls_settings(hosts: &HostConfig, pki: &PkiConfig) -> Result<TlsSettings, String> {
    validate_pki(hosts, pki)?;
    let callbacks = Box::new(FleetTlsCallbacks);
    let mut settings = TlsSettings::with_callbacks(callbacks)
        .map_err(|error| format!("unable to initialize fleet TLS: {error}"))?;
    settings
        .set_certificate_chain_file(path_str(&pki.server_certificate)?)
        .map_err(|error| format!("unable to load fleet server certificate: {error}"))?;
    settings
        .set_private_key_file(path_str(&pki.server_private_key)?, SslFiletype::PEM)
        .map_err(|error| format!("unable to load fleet server key: {error}"))?;
    settings
        .check_private_key()
        .map_err(|error| format!("fleet server certificate/key mismatch: {error}"))?;
    settings.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
    settings.set_verify_depth(2);
    settings
        .set_ca_file(path_str(&pki.ca_certificate)?)
        .map_err(|error| format!("unable to load fleet client CA: {error}"))?;
    settings.set_client_ca_list(
        X509Name::load_client_ca_file(path_str(&pki.ca_certificate)?)
            .map_err(|error| format!("unable to load fleet client CA names: {error}"))?,
    );
    settings
        .set_min_proto_version(Some(SslVersion::TLS1_2))
        .map_err(|error| format!("unable to set fleet minimum TLS version: {error}"))?;
    settings
        .set_max_proto_version(Some(SslVersion::TLS1_3))
        .map_err(|error| format!("unable to set fleet maximum TLS version: {error}"))?;
    settings.set_options(SslOptions::NO_TICKET);
    Ok(settings)
}

pub fn client_certificate(session: &Session) -> Option<&ClientCertificateInfo> {
    session
        .digest()
        .and_then(|digest| digest.ssl_digest.as_ref())
        .and_then(|digest| digest.extension.get::<ClientCertificateInfo>())
}

fn path_str(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("PKI path is not valid UTF-8: {}", path.display()))
}

#[allow(dead_code)]
fn _assert_pingora_result_is_linked(_: PingoraResult<()>) {}
