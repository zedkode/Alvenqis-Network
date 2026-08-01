use base64::Engine as _;
use http::HeaderValue;
use sha2::{Digest as _, Sha256};
use std::fmt;
use subtle::ConstantTimeEq as _;

const MAX_AUTHORIZATION_BYTES: usize = 1_024;
const MAX_DECODED_CREDENTIAL_BYTES: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminRole {
    Viewer,
    Operator,
}

impl AdminRole {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Operator => "operator",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminIdentity {
    pub username: String,
    pub role: AdminRole,
}

#[derive(Clone)]
pub struct AdminAuthenticator {
    viewer_username: String,
    operator_username: String,
    viewer_password_digest: [u8; 32],
    operator_password_digest: [u8; 32],
}

impl fmt::Debug for AdminAuthenticator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdminAuthenticator")
            .field("viewer_username", &self.viewer_username)
            .field("operator_username", &self.operator_username)
            .field("passwords", &"[REDACTED]")
            .finish()
    }
}

impl AdminAuthenticator {
    pub fn new(
        viewer_username: String,
        viewer_password: &[u8],
        operator_username: String,
        operator_password: &[u8],
    ) -> Result<Self, String> {
        validate_username("viewer", &viewer_username)?;
        validate_username("operator", &operator_username)?;
        if viewer_username == operator_username {
            return Err("viewer and operator usernames must be distinct".to_owned());
        }
        validate_password("viewer", viewer_password)?;
        validate_password("operator", operator_password)?;

        Ok(Self {
            viewer_username,
            operator_username,
            viewer_password_digest: Sha256::digest(viewer_password).into(),
            operator_password_digest: Sha256::digest(operator_password).into(),
        })
    }

    pub fn authenticate(&self, authorization: Option<&HeaderValue>) -> Option<AdminIdentity> {
        let value = authorization?.as_bytes();
        if value.len() > MAX_AUTHORIZATION_BYTES {
            return None;
        }
        let encoded = value.strip_prefix(b"Basic ")?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?;
        if decoded.len() > MAX_DECODED_CREDENTIAL_BYTES {
            return None;
        }
        let separator = decoded.iter().position(|byte| *byte == b':')?;
        let username = std::str::from_utf8(&decoded[..separator]).ok()?;
        let password_digest: [u8; 32] = Sha256::digest(&decoded[separator + 1..]).into();

        if username == self.viewer_username
            && bool::from(password_digest.ct_eq(&self.viewer_password_digest))
        {
            return Some(AdminIdentity {
                username: self.viewer_username.clone(),
                role: AdminRole::Viewer,
            });
        }
        if username == self.operator_username
            && bool::from(password_digest.ct_eq(&self.operator_password_digest))
        {
            return Some(AdminIdentity {
                username: self.operator_username.clone(),
                role: AdminRole::Operator,
            });
        }
        None
    }
}

fn validate_username(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    {
        return Err(format!(
            "{label} username must contain 1-64 safe ASCII characters"
        ));
    }
    Ok(())
}

fn validate_password(label: &str, value: &[u8]) -> Result<(), String> {
    if value.is_empty() || value.len() > 256 || value.contains(&b'\0') {
        return Err(format!("{label} password must contain 1-256 non-NUL bytes"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authorization(username: &str, password: &str) -> HeaderValue {
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
        HeaderValue::from_str(&format!("Basic {encoded}")).unwrap()
    }

    #[test]
    fn authenticates_distinct_viewer_and_operator_roles() {
        let auth = AdminAuthenticator::new(
            "viewer".to_owned(),
            b"viewer-secret",
            "operator".to_owned(),
            b"operator-secret",
        )
        .unwrap();

        assert_eq!(
            auth.authenticate(Some(&authorization("viewer", "viewer-secret")))
                .unwrap()
                .role,
            AdminRole::Viewer
        );
        assert_eq!(
            auth.authenticate(Some(&authorization("operator", "operator-secret")))
                .unwrap()
                .role,
            AdminRole::Operator
        );
    }

    #[test]
    fn rejects_wrong_or_oversized_credentials() {
        let auth = AdminAuthenticator::new(
            "viewer".to_owned(),
            b"viewer-secret",
            "operator".to_owned(),
            b"operator-secret",
        )
        .unwrap();

        assert!(auth
            .authenticate(Some(&authorization("viewer", "wrong")))
            .is_none());
        assert!(auth
            .authenticate(Some(&HeaderValue::from_bytes(&vec![b'a'; 1_025]).unwrap()))
            .is_none());
    }

    #[test]
    fn debug_output_never_contains_password_material() {
        let auth = AdminAuthenticator::new(
            "viewer".to_owned(),
            b"viewer-secret",
            "operator".to_owned(),
            b"operator-secret",
        )
        .unwrap();

        let rendered = format!("{auth:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("viewer-secret"));
        assert!(!rendered.contains("operator-secret"));
    }
}
