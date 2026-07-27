use crate::protocol::constants::{ATOMIC_UNITS_PER_ALVE, DECIMALS, MAX_SUPPLY_ATOMIC};
use crate::protocol::errors::{AlvenqisError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Amount(u64);

impl Amount {
    pub const ZERO: Self = Self(0);

    pub const fn from_atomic(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_atomic(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Result<Self> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(AlvenqisError::AmountOverflow)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(AlvenqisError::AmountOverflow)
    }

    pub fn format_alve(self) -> String {
        let whole = self.0 / ATOMIC_UNITS_PER_ALVE;
        let fractional = self.0 % ATOMIC_UNITS_PER_ALVE;
        format!("{whole}.{fractional:0width$}", width = DECIMALS as usize)
    }

    pub fn parse_alve(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AlvenqisError::AmountParse(input.to_owned()));
        }

        let mut parts = trimmed.split('.');
        let whole_part = parts.next().unwrap_or_default();
        let fractional_part = parts.next();
        if parts.next().is_some() {
            return Err(AlvenqisError::AmountParse(trimmed.to_owned()));
        }

        let whole_value: u64 = if whole_part.is_empty() {
            0
        } else {
            whole_part
                .parse()
                .map_err(|_| AlvenqisError::AmountParse(trimmed.to_owned()))?
        };

        let whole_atomic = whole_value
            .checked_mul(ATOMIC_UNITS_PER_ALVE)
            .ok_or(AlvenqisError::AmountOverflow)?;

        let fractional_atomic = match fractional_part {
            Some(part) => {
                if part.len() > DECIMALS as usize {
                    return Err(AlvenqisError::TooManyDecimals {
                        value: trimmed.to_owned(),
                        max: DECIMALS,
                    });
                }

                let mut padded = part.to_owned();
                while padded.len() < DECIMALS as usize {
                    padded.push('0');
                }

                if padded.is_empty() {
                    0
                } else {
                    padded
                        .parse::<u64>()
                        .map_err(|_| AlvenqisError::AmountParse(trimmed.to_owned()))?
                }
            }
            None => 0,
        };

        let total = whole_atomic
            .checked_add(fractional_atomic)
            .ok_or(AlvenqisError::AmountOverflow)?;

        if total > MAX_SUPPLY_ATOMIC {
            return Err(AlvenqisError::AmountParse(format!(
                "{trimmed} exceeds max supply"
            )));
        }

        Ok(Self(total))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_alve())
    }
}
