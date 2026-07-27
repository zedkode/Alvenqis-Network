//! Pool block maturity helpers.
//!
//! Same rule as `alvenqis-mining-pool` and the public TypeScript `@alvenqis/sdk`:
//! a block is mature when `tip_height >= block_height + required_confirmations`.

/// Progress of a pool-found block toward maturity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaturityProgress {
    pub status: MaturityStatus,
    pub confirmations: u64,
    pub required: u64,
    pub remaining: u64,
    /// 0–100 integer percent toward required confirmations.
    pub percent: u8,
    /// Chain tip must reach this height (inclusive) for maturity.
    pub mature_at_tip: u64,
    pub label: String,
}

/// Maturity classification for a pool block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaturityStatus {
    Immature,
    Mature,
    Orphaned,
    Unknown,
}

impl MaturityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Immature => "immature",
            Self::Mature => "mature",
            Self::Orphaned => "orphaned",
            Self::Unknown => "unknown",
        }
    }
}

/// Default confirmations required by the Mainnet Candidate mining pool product.
pub const DEFAULT_BLOCK_MATURITY_CONFIRMATIONS: u64 = 12;

/// Compute maturity for a pool block.
///
/// - `block_height`: height of the found block
/// - `tip_height`: current chain tip (if known)
/// - `required_confirmations`: usually from pool status (`block_maturity_confirmations`)
/// - `pool_status_field`: optional pool-reported status string (e.g. `"mature"`, `"orphaned"`)
pub fn pool_block_maturity(
    block_height: u64,
    tip_height: Option<u64>,
    required_confirmations: u64,
    pool_status_field: Option<&str>,
) -> MaturityProgress {
    let required = required_confirmations.max(1);
    let mature_at_tip = block_height.saturating_add(required);
    let field = pool_status_field.unwrap_or("").to_ascii_lowercase();

    if field.contains("orphan") {
        return MaturityProgress {
            status: MaturityStatus::Orphaned,
            confirmations: 0,
            required,
            remaining: 0,
            percent: 0,
            mature_at_tip,
            label: "orphaned".to_owned(),
        };
    }

    if field == "mature" || field.contains("matured") {
        return MaturityProgress {
            status: MaturityStatus::Mature,
            confirmations: required,
            required,
            remaining: 0,
            percent: 100,
            mature_at_tip,
            label: "mature".to_owned(),
        };
    }

    let Some(tip) = tip_height else {
        return MaturityProgress {
            status: MaturityStatus::Unknown,
            confirmations: 0,
            required,
            remaining: required,
            percent: 0,
            mature_at_tip,
            label: "immature · tip unknown".to_owned(),
        };
    };

    if tip >= mature_at_tip {
        return MaturityProgress {
            status: MaturityStatus::Mature,
            confirmations: required,
            required,
            remaining: 0,
            percent: 100,
            mature_at_tip,
            label: "mature".to_owned(),
        };
    }

    let confirmations = if tip < block_height {
        0
    } else {
        (tip - block_height).min(required)
    };
    let remaining = required.saturating_sub(confirmations);
    let percent = ((confirmations * 100) / required) as u8;

    MaturityProgress {
        status: MaturityStatus::Immature,
        confirmations,
        required,
        remaining,
        percent,
        mature_at_tip,
        label: format!("immature · {confirmations}/{required}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immature_until_tip_reaches_required() {
        let m = pool_block_maturity(10, Some(15), 12, None);
        assert_eq!(m.status, MaturityStatus::Immature);
        assert_eq!(m.confirmations, 5);
        assert_eq!(m.remaining, 7);
        assert_eq!(m.mature_at_tip, 22);
        assert!(m.label.contains("5/12"));
    }

    #[test]
    fn mature_when_tip_high_enough() {
        let m = pool_block_maturity(10, Some(22), 12, None);
        assert_eq!(m.status, MaturityStatus::Mature);
        assert_eq!(m.remaining, 0);
        assert_eq!(m.percent, 100);
    }

    #[test]
    fn orphan_from_pool_status_field() {
        let m = pool_block_maturity(10, Some(100), 12, Some("orphaned"));
        assert_eq!(m.status, MaturityStatus::Orphaned);
    }

    #[test]
    fn unknown_without_tip() {
        let m = pool_block_maturity(10, None, 12, None);
        assert_eq!(m.status, MaturityStatus::Unknown);
    }
}
