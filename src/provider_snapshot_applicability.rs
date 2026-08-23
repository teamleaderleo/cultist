#[allow(dead_code)]
#[path = "provider_snapshot_applicability/identity.rs"]
pub mod identity;

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

use crate::applicability::{ApplicabilityStatus, evaluate_exact_coordinate};

pub const PROVIDER_SNAPSHOT_APPLICABILITY_SCHEMA_VERSION: u32 = 1;
const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_BYTES: usize = 64;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProviderSnapshotIdentity(String);

impl ProviderSnapshotIdentity {
    pub fn parse(value: impl Into<String>) -> Result<Self, ProviderSnapshotIdentityError> {
        let value = value.into();
        validate_identity(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ProviderSnapshotIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ProviderSnapshotApplicability {
    pub schema_version: u32,
    pub status: ApplicabilityStatus,
    pub required: ProviderSnapshotIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<ProviderSnapshotIdentity>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderSnapshotApplicabilityWire {
    schema_version: u32,
    status: ApplicabilityStatus,
    required: ProviderSnapshotIdentity,
    #[serde(default)]
    actual: Option<ProviderSnapshotIdentity>,
}

impl<'de> Deserialize<'de> for ProviderSnapshotApplicability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProviderSnapshotApplicabilityWire::deserialize(deserializer)?;
        if wire.schema_version != PROVIDER_SNAPSHOT_APPLICABILITY_SCHEMA_VERSION {
            return Err(<D::Error as serde::de::Error>::custom(format!(
                "unsupported provider snapshot applicability schema {}; expected {PROVIDER_SNAPSHOT_APPLICABILITY_SCHEMA_VERSION}",
                wire.schema_version
            )));
        }

        let expected = evaluate_provider_snapshot(&wire.required, wire.actual.as_ref());
        if wire.status != expected.status {
            return Err(<D::Error as serde::de::Error>::custom(
                "provider snapshot applicability status is inconsistent with required/actual identities",
            ));
        }
        Ok(expected)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProviderSnapshotIdentityError {
    message: String,
}

impl ProviderSnapshotIdentityError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ProviderSnapshotIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ProviderSnapshotIdentityError {}

pub fn evaluate_provider_snapshot(
    required: &ProviderSnapshotIdentity,
    current: Option<&ProviderSnapshotIdentity>,
) -> ProviderSnapshotApplicability {
    let status = evaluate_exact_coordinate(
        required.as_str(),
        current.map(ProviderSnapshotIdentity::as_str),
    );

    ProviderSnapshotApplicability {
        schema_version: PROVIDER_SNAPSHOT_APPLICABILITY_SCHEMA_VERSION,
        status,
        required: required.clone(),
        actual: current.cloned(),
    }
}

fn validate_identity(value: &str) -> Result<(), ProviderSnapshotIdentityError> {
    let Some(digest) = value.strip_prefix(SHA256_PREFIX) else {
        return Err(ProviderSnapshotIdentityError::new(
            "provider snapshot identity must use `sha256:<digest>`",
        ));
    };

    if digest.len() != SHA256_HEX_BYTES
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(ProviderSnapshotIdentityError::new(
            "provider snapshot sha256 digest must contain exactly 64 lowercase hexadecimal characters",
        ));
    }

    Ok(())
}
