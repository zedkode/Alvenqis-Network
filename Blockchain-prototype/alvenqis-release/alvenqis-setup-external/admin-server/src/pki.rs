use rand::{rngs::OsRng, RngCore};
use rcgen::{
    BasicConstraints, CertificateParams, CertificateSigningRequestParams, DistinguishedName,
    DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair, KeyUsagePurpose, PublicKeyData,
};
use rustls::client::verify_server_name;
use rustls::pki_types::{pem::PemObject, CertificateDer, ServerName};
use rustls::server::ParsedCertificate;
use sha1::{Digest, Sha1};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use time::{Duration, OffsetDateTime};

const CA_CERT_FILE: &str = "fleet-ca.crt.pem";
const CA_KEY_FILE: &str = "fleet-ca.key.pem";
const SERVER_CERT_FILE: &str = "fleet-server.crt.pem";
const SERVER_KEY_FILE: &str = "fleet-server.key.pem";

#[derive(Clone, Debug)]
pub struct FleetPki {
    ca_directory: PathBuf,
    edge_directory: PathBuf,
}

#[derive(Clone, Debug)]
pub struct IssuedClientCertificate {
    pub certificate_pem: String,
    pub ca_certificate_pem: String,
    pub fingerprint_sha1: String,
    pub expires_at_unix_seconds: u64,
}

impl FleetPki {
    pub fn load_or_initialize(state_dir: &Path, server_name: &str) -> Result<Self, String> {
        let pki_root = state_dir.join("pki");
        let ca_directory = pki_root.join("ca");
        let edge_directory = pki_root.join("edge");
        ensure_private_directory(&ca_directory)?;
        ensure_private_directory(&edge_directory)?;
        let pki = Self {
            ca_directory,
            edge_directory,
        };
        pki.ensure_ca()?;
        pki.ensure_server_certificate(server_name)?;
        Ok(pki)
    }

    pub fn issue_client_certificate(
        &self,
        csr_pem: &str,
    ) -> Result<IssuedClientCertificate, String> {
        let mut request = CertificateSigningRequestParams::from_pem(csr_pem)
            .map_err(|error| format!("invalid certificate signing request: {error}"))?;
        request.params.is_ca = IsCa::NoCa;
        request.params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        request.params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let now = OffsetDateTime::now_utc();
        request.params.not_before = now - Duration::minutes(5);
        request.params.not_after = now + Duration::days(90);
        let issuer = self.issuer()?;
        let certificate = request
            .signed_by(&issuer)
            .map_err(|error| format!("cannot sign agent certificate: {error}"))?;
        let certificate_pem = certificate.pem();
        let fingerprint_sha1 = hex::encode_upper(Sha1::digest(certificate.der().as_ref()));
        Ok(IssuedClientCertificate {
            certificate_pem,
            ca_certificate_pem: self.read_ca(CA_CERT_FILE)?,
            fingerprint_sha1,
            expires_at_unix_seconds: u64::try_from((now + Duration::days(90)).unix_timestamp())
                .map_err(|_| "agent certificate expiry is outside supported range")?,
        })
    }

    pub fn validate_client_csr(csr_pem: &str) -> Result<(), String> {
        CertificateSigningRequestParams::from_pem(csr_pem)
            .map(|_| ())
            .map_err(|error| format!("invalid certificate signing request: {error}"))
    }

    pub fn generate_agent_key_and_csr(common_name: &str) -> Result<(String, String), String> {
        let key = KeyPair::generate().map_err(|error| error.to_string())?;
        let mut params =
            CertificateParams::new(Vec::<String>::new()).map_err(|error| error.to_string())?;
        params.distinguished_name = distinguished_name(common_name);
        params.is_ca = IsCa::NoCa;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let csr = params
            .serialize_request(&key)
            .map_err(|error| error.to_string())?
            .pem()
            .map_err(|error| error.to_string())?;
        Ok((key.serialize_pem(), csr))
    }

    fn ensure_ca(&self) -> Result<(), String> {
        let cert_path = self.ca_directory.join(CA_CERT_FILE);
        let key_path = self.ca_directory.join(CA_KEY_FILE);
        let edge_cert_path = self.edge_directory.join(CA_CERT_FILE);
        if cert_path.exists() && key_path.exists() {
            let certificate = fs::read(&cert_path).map_err(|error| error.to_string())?;
            if edge_cert_path.exists() {
                if fs::read(&edge_cert_path).map_err(|error| error.to_string())? != certificate {
                    return Err("fleet edge CA certificate does not match controller CA".to_owned());
                }
            } else {
                write_public(&edge_cert_path, &certificate)?;
            }
            return Ok(());
        }
        if cert_path.exists() || key_path.exists() {
            return Err("fleet CA is incomplete; refusing to replace existing material".to_owned());
        }
        let key = KeyPair::generate().map_err(|error| error.to_string())?;
        let mut params =
            CertificateParams::new(Vec::<String>::new()).map_err(|error| error.to_string())?;
        params.distinguished_name = distinguished_name("Alvenqis Fleet CA");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::minutes(5);
        params.not_after = now + Duration::days(3_650);
        let certificate = params
            .self_signed(&key)
            .map_err(|error| error.to_string())?;
        write_private(&key_path, key.serialize_pem().as_bytes())?;
        let certificate_pem = certificate.pem();
        write_public(&cert_path, certificate_pem.as_bytes())?;
        write_public(&edge_cert_path, certificate_pem.as_bytes())
    }

    fn ensure_server_certificate(&self, server_name: &str) -> Result<(), String> {
        let cert_path = self.edge_directory.join(SERVER_CERT_FILE);
        let key_path = self.edge_directory.join(SERVER_KEY_FILE);
        if cert_path.exists() && key_path.exists() {
            let certificate_pem = fs::read_to_string(&cert_path)
                .map_err(|error| format!("cannot read fleet server certificate: {error}"))?;
            let key_pem = fs::read_to_string(&key_path)
                .map_err(|error| format!("cannot read fleet server private key: {error}"))?;
            let key = KeyPair::from_pem(&key_pem)
                .map_err(|error| format!("invalid fleet server private key: {error}"))?;
            if server_certificate_matches(&certificate_pem, &key, server_name)? {
                return Ok(());
            }

            let certificate_pem = self.issue_server_certificate(server_name, &key)?;
            return write_public(&cert_path, certificate_pem.as_bytes());
        }
        if cert_path.exists() || key_path.exists() {
            return Err("fleet server certificate is incomplete; refusing replacement".to_owned());
        }
        let key = KeyPair::generate().map_err(|error| error.to_string())?;
        let certificate_pem = self.issue_server_certificate(server_name, &key)?;
        write_private(&key_path, key.serialize_pem().as_bytes())?;
        write_public(&cert_path, certificate_pem.as_bytes())
    }

    fn issue_server_certificate(&self, server_name: &str, key: &KeyPair) -> Result<String, String> {
        let mut params = CertificateParams::new(vec![server_name.to_owned()])
            .map_err(|error| error.to_string())?;
        params.distinguished_name = distinguished_name(server_name);
        params.is_ca = IsCa::NoCa;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::minutes(5);
        params.not_after = now + Duration::days(397);
        let certificate = params
            .signed_by(&key, &self.issuer()?)
            .map_err(|error| error.to_string())?;
        Ok(certificate.pem())
    }

    fn issuer(&self) -> Result<Issuer<'static, KeyPair>, String> {
        let certificate = self.read_ca(CA_CERT_FILE)?;
        let key = KeyPair::from_pem(&self.read_ca(CA_KEY_FILE)?)
            .map_err(|error| format!("invalid fleet CA key: {error}"))?;
        Issuer::from_ca_cert_pem(&certificate, key)
            .map_err(|error| format!("invalid fleet CA certificate: {error}"))
    }

    fn read_ca(&self, name: &str) -> Result<String, String> {
        fs::read_to_string(self.ca_directory.join(name)).map_err(|error| error.to_string())
    }
}

fn server_certificate_matches(
    certificate_pem: &str,
    key: &KeyPair,
    server_name: &str,
) -> Result<bool, String> {
    let certificate = CertificateDer::from_pem_slice(certificate_pem.as_bytes())
        .map_err(|error| format!("invalid fleet server certificate PEM: {error}"))?;
    let parsed = ParsedCertificate::try_from(&certificate)
        .map_err(|error| format!("invalid fleet server certificate: {error}"))?;
    let certificate_public_key = parsed.subject_public_key_info();
    let private_key_public_key = key.subject_public_key_info();
    if certificate_public_key.as_ref() != private_key_public_key.as_slice() {
        return Err(
            "fleet server certificate does not match existing private key; refusing replacement"
                .to_owned(),
        );
    }
    let server_name = ServerName::try_from(server_name.to_owned())
        .map_err(|error| format!("invalid fleet server name: {error}"))?;
    Ok(verify_server_name(&parsed, &server_name).is_ok())
}

fn distinguished_name(common_name: &str) -> DistinguishedName {
    let mut name = DistinguishedName::new();
    name.push(DnType::CommonName, common_name);
    name
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_file(path, bytes, 0o600)
}

fn write_public(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_file(path, bytes, 0o644)
}

fn write_file(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("PKI path has no parent: {}", path.display()))?;
    ensure_private_directory(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("PKI path has no valid filename: {}", path.display()))?;
    let mut random = [0_u8; 8];
    OsRng.fill_bytes(&mut random);
    let temporary = parent.join(format!(".{file_name}.{}.tmp", hex::encode(random)));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    let result = (|| -> Result<(), String> {
        let mut file = options
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        fs::File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| error.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::client::verify_server_name;
    use rustls::pki_types::{pem::PemObject, CertificateDer, ServerName};
    use rustls::server::ParsedCertificate;

    fn certificate_matches_name(certificate_pem: &str, server_name: &str) -> bool {
        let certificate =
            CertificateDer::from_pem_slice(certificate_pem.as_bytes()).expect("certificate PEM");
        let certificate = ParsedCertificate::try_from(&certificate).expect("parsed certificate");
        let server_name = ServerName::try_from(server_name.to_owned()).expect("server name");
        verify_server_name(&certificate, &server_name).is_ok()
    }

    #[test]
    fn agent_key_stays_with_requester_and_controller_signs_only_the_csr() {
        let directory = tempfile::tempdir().expect("tempdir");
        let pki = FleetPki::load_or_initialize(directory.path(), "fleet.example.org")
            .expect("initialize pki");
        let (private_key, csr) = FleetPki::generate_agent_key_and_csr("peer-2").expect("agent csr");
        let issued = pki
            .issue_client_certificate(&csr)
            .expect("client certificate");
        assert!(private_key.contains("PRIVATE KEY"));
        assert!(!csr.contains("PRIVATE KEY"));
        assert!(!issued.certificate_pem.contains("PRIVATE KEY"));
        assert_eq!(issued.fingerprint_sha1.len(), 40);
        assert!(issued.expires_at_unix_seconds > 0);
        assert!(issued.ca_certificate_pem.contains("CERTIFICATE"));
        let pki_root = directory.path().join("pki");
        assert!(pki_root.join("ca/fleet-ca.key.pem").is_file());
        assert!(pki_root.join("ca/fleet-ca.crt.pem").is_file());
        assert!(!pki_root.join("edge/fleet-ca.key.pem").exists());
        assert!(pki_root.join("edge/fleet-ca.crt.pem").is_file());
        assert!(pki_root.join("edge/fleet-server.key.pem").is_file());
        assert!(pki_root.join("edge/fleet-server.crt.pem").is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(pki_root.join("ca/fleet-ca.key.pem"))
                    .expect("CA key metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(pki_root.join("edge/fleet-server.key.pem"))
                    .expect("server key metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn server_hostname_change_reissues_certificate_without_rotating_private_key() {
        let directory = tempfile::tempdir().expect("tempdir");
        FleetPki::load_or_initialize(directory.path(), "fleet-old.example.org")
            .expect("initialize original PKI");
        let edge = directory.path().join("pki/edge");
        let certificate_path = edge.join(SERVER_CERT_FILE);
        let key_path = edge.join(SERVER_KEY_FILE);
        let original_certificate = fs::read_to_string(&certificate_path).expect("original cert");
        let original_key = fs::read_to_string(&key_path).expect("original key");
        assert!(certificate_matches_name(
            &original_certificate,
            "fleet-old.example.org"
        ));
        assert!(!certificate_matches_name(
            &original_certificate,
            "fleet-new.example.org"
        ));

        FleetPki::load_or_initialize(directory.path(), "fleet-new.example.org")
            .expect("reconcile changed server name");

        let replacement_certificate =
            fs::read_to_string(&certificate_path).expect("replacement cert");
        let replacement_key = fs::read_to_string(&key_path).expect("preserved key");
        assert_ne!(replacement_certificate, original_certificate);
        assert_eq!(replacement_key, original_key);
        assert!(certificate_matches_name(
            &replacement_certificate,
            "fleet-new.example.org"
        ));
        assert!(!certificate_matches_name(
            &replacement_certificate,
            "fleet-old.example.org"
        ));
    }

    #[test]
    fn matching_server_certificate_is_reused_without_file_changes() {
        let directory = tempfile::tempdir().expect("tempdir");
        FleetPki::load_or_initialize(directory.path(), "fleet.example.org")
            .expect("initialize PKI");
        let edge = directory.path().join("pki/edge");
        let certificate_path = edge.join(SERVER_CERT_FILE);
        let key_path = edge.join(SERVER_KEY_FILE);
        let original_certificate = fs::read(&certificate_path).expect("original cert");
        let original_key = fs::read(&key_path).expect("original key");

        FleetPki::load_or_initialize(directory.path(), "fleet.example.org")
            .expect("reload matching PKI");

        assert_eq!(
            fs::read(&certificate_path).expect("reused cert"),
            original_certificate
        );
        assert_eq!(fs::read(&key_path).expect("reused key"), original_key);
    }

    #[test]
    fn mismatched_existing_server_certificate_and_key_are_rejected_unchanged() {
        let first = tempfile::tempdir().expect("first tempdir");
        let second = tempfile::tempdir().expect("second tempdir");
        FleetPki::load_or_initialize(first.path(), "fleet.example.org")
            .expect("initialize first PKI");
        FleetPki::load_or_initialize(second.path(), "fleet.example.org")
            .expect("initialize second PKI");
        let first_edge = first.path().join("pki/edge");
        let certificate_path = first_edge.join(SERVER_CERT_FILE);
        let key_path = first_edge.join(SERVER_KEY_FILE);
        let original_certificate = fs::read(&certificate_path).expect("original cert");
        let unrelated_key =
            fs::read(second.path().join("pki/edge").join(SERVER_KEY_FILE)).expect("other key");
        fs::write(&key_path, &unrelated_key).expect("install mismatched key fixture");

        let error = FleetPki::load_or_initialize(first.path(), "fleet.example.org")
            .expect_err("mismatched key and certificate must fail closed");

        assert!(error.contains("does not match existing private key"));
        assert_eq!(
            fs::read(&certificate_path).expect("cert unchanged"),
            original_certificate
        );
        assert_eq!(fs::read(&key_path).expect("key unchanged"), unrelated_key);
    }

    #[test]
    fn partial_existing_server_certificate_state_is_rejected_unchanged() {
        let directory = tempfile::tempdir().expect("tempdir");
        let edge = directory.path().join("pki/edge");
        ensure_private_directory(&edge).expect("edge directory");
        let key_path = edge.join(SERVER_KEY_FILE);
        let key = KeyPair::generate().expect("server key").serialize_pem();
        write_private(&key_path, key.as_bytes()).expect("persist partial key");

        let error = FleetPki::load_or_initialize(directory.path(), "fleet.example.org")
            .expect_err("partial server PKI must fail closed");

        assert!(error.contains("incomplete; refusing replacement"));
        assert_eq!(fs::read_to_string(&key_path).expect("key unchanged"), key);
        assert!(!edge.join(SERVER_CERT_FILE).exists());
    }
}
