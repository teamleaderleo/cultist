#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/provider_snapshot_applicability.rs"]
mod provider_snapshot_applicability;

use applicability::ApplicabilityStatus;
use provider_snapshot_applicability::{
    PROVIDER_SNAPSHOT_APPLICABILITY_SCHEMA_VERSION, ProviderSnapshotApplicability,
    ProviderSnapshotIdentity, evaluate_provider_snapshot,
};

fn identity(hex: char) -> ProviderSnapshotIdentity {
    ProviderSnapshotIdentity::parse(format!("sha256:{}", hex.to_string().repeat(64))).unwrap()
}

#[test]
fn exact_provider_snapshot_identity_applies() {
    let required = identity('a');
    let current = identity('a');
    assert_eq!(required.as_str(), format!("sha256:{}", "a".repeat(64)));
    let evaluation = evaluate_provider_snapshot(&required, Some(&current));

    assert_eq!(evaluation.status, ApplicabilityStatus::Applies);
    assert_eq!(evaluation.required, required);
    assert_eq!(evaluation.actual, Some(current));
}

#[test]
fn known_provider_snapshot_movement_invalidates() {
    let required = identity('a');
    let current = identity('b');
    let evaluation = evaluate_provider_snapshot(&required, Some(&current));

    assert_eq!(evaluation.status, ApplicabilityStatus::Invalid);
    assert_eq!(evaluation.actual, Some(current));
}

#[test]
fn unavailable_current_provider_snapshot_is_unknown() {
    let required = identity('a');
    let evaluation = evaluate_provider_snapshot(&required, None);

    assert_eq!(evaluation.status, ApplicabilityStatus::Unknown);
    assert_eq!(evaluation.actual, None);
}

#[test]
fn canonical_identity_parser_fails_closed() {
    let uppercase = format!("sha256:{}", "A".repeat(64));
    let malformed = [
        "a".repeat(64),
        "sha256:".to_string(),
        format!("sha256:{}", "a".repeat(63)),
        format!("sha256:{}", "a".repeat(65)),
        format!("sha256:{}g", "a".repeat(63)),
        uppercase,
        format!(" sha256:{}", "a".repeat(64)),
        format!("sha256:{} ", "a".repeat(64)),
    ];

    for value in malformed {
        assert!(
            ProviderSnapshotIdentity::parse(&value).is_err(),
            "accepted malformed identity `{value}`"
        );
    }
}

#[test]
fn identity_and_evaluation_round_trip_as_machine_objects() {
    let required = identity('a');
    let current = identity('b');

    let identity_json = serde_json::to_string(&required).unwrap();
    assert_eq!(
        serde_json::from_str::<ProviderSnapshotIdentity>(&identity_json).unwrap(),
        required
    );

    let evaluation = evaluate_provider_snapshot(&required, Some(&current));
    let evaluation_json = serde_json::to_string(&evaluation).unwrap();
    let decoded: ProviderSnapshotApplicability = serde_json::from_str(&evaluation_json).unwrap();
    assert_eq!(decoded, evaluation);
    assert_eq!(
        decoded.schema_version,
        PROVIDER_SNAPSHOT_APPLICABILITY_SCHEMA_VERSION
    );
}

#[test]
fn malformed_machine_identity_is_rejected_during_deserialization() {
    let json = format!("\"sha256:{}\"", "A".repeat(64));
    let error = serde_json::from_str::<ProviderSnapshotIdentity>(&json).unwrap_err();
    assert!(error.to_string().contains("lowercase hexadecimal"));
}

#[test]
fn evaluation_deserialization_rejects_unsupported_schema() {
    let json = format!(
        r#"{{
            "schema_version": 2,
            "status": "applies",
            "required": "sha256:{}",
            "actual": "sha256:{}"
        }}"#,
        "a".repeat(64),
        "a".repeat(64)
    );
    let error = serde_json::from_str::<ProviderSnapshotApplicability>(&json).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unsupported provider snapshot applicability schema")
    );
}

#[test]
fn evaluation_deserialization_rejects_inconsistent_status() {
    let different_actual = format!(
        r#"{{
            "schema_version": 1,
            "status": "applies",
            "required": "sha256:{}",
            "actual": "sha256:{}"
        }}"#,
        "a".repeat(64),
        "b".repeat(64)
    );
    let missing_actual = format!(
        r#"{{
            "schema_version": 1,
            "status": "applies",
            "required": "sha256:{}"
        }}"#,
        "a".repeat(64)
    );

    for json in [different_actual, missing_actual] {
        let error = serde_json::from_str::<ProviderSnapshotApplicability>(&json).unwrap_err();
        assert!(error.to_string().contains("status is inconsistent"));
    }
}

#[test]
fn unknown_evaluation_fields_fail_closed() {
    let json = format!(
        r#"{{
            "schema_version": 1,
            "status": "applies",
            "required": "sha256:{}",
            "future_semantics": true
        }}"#,
        "a".repeat(64)
    );
    let error = serde_json::from_str::<ProviderSnapshotApplicability>(&json).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}
