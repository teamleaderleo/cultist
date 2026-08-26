#[allow(dead_code)]
#[path = "../src/c1_local_alias.rs"]
mod c1_local_alias;
#[allow(dead_code)]
#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;

use c1_local_alias::{alias_canonical_c1, expand_c1_aliases};
use finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

fn representative_report() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "projection-compression-research".to_string(),
        repository: "/repo".to_string(),
        claims: vec![
            Claim::new(
                ClaimKind::Derived,
                "Current work and active work were compared at the admitted coordinates.",
            )
            .with_evidence(Evidence::new(
                "Current work is #121 at exact head abcdef1234567890.",
            ))
            .with_evidence(Evidence::new(
                "Other work is #124 at exact head fedcba0987654321.",
            )),
        ],
        findings: vec![
            Finding::new("preflight-explicit-coordination", "Explicit coordination edge")
                .at(Location::new("src/auth.rs", Some(42)))
                .with_claim(
                    Claim::new(
                        ClaimKind::Observed,
                        "The admitted inventory records `hold_merge_while` from `#121` to `#124`.",
                    )
                    .with_evidence(Evidence::new(
                        "Coordination source reference: github:pull/121.",
                    ))
                    .with_evidence(Evidence::new(
                        "Related active work #124 is at exact head fedcba0987654321.",
                    )),
                )
                .with_claim(Claim::new(
                    ClaimKind::Unknown,
                    "The inventory does not establish the operational consequence beyond the declared relation.",
                ))
                .with_question(
                    "Should merge order be coordinated before either change advances the evidence baseline?",
                ),
        ],
    }
}

#[test]
fn retained_historical_scan_gets_material_alias_savings_and_exact_round_trip() {
    let report: AnalysisReport =
        serde_json::from_str(include_str!("fixtures/smolrunner_scan_ed3b70e.json")).unwrap();
    let canonical = compact_ir::encode_report(&report).unwrap();
    let transformed = alias_canonical_c1(&canonical).unwrap();

    assert!(transformed.used_aliases());
    assert_eq!(transformed.canonical_bytes, canonical.len());
    assert_eq!(transformed.encoded_bytes, transformed.encoded.len());
    assert!(transformed.encoded_bytes < transformed.canonical_bytes);

    let bytes_saved = transformed.canonical_bytes - transformed.encoded_bytes;
    assert!(
        bytes_saved * 100 >= transformed.canonical_bytes * 20,
        "expected at least 20% savings; canonical={} aliased={} saved={}",
        transformed.canonical_bytes,
        transformed.encoded_bytes,
        bytes_saved
    );

    let values = transformed
        .aliases
        .iter()
        .map(|alias| alias.value.as_str())
        .collect::<Vec<_>>();
    assert!(values.contains(&"src/unix_personal_worker_store.rs"));
    assert!(values.contains(&"test-module-one-off"));
    assert!(values.contains(&"One-off test-module name"));
    assert!(values.contains(
        &"Is this one-off name intentionally scoped, or an accidental deviation from local precedent?"
    ));
    assert!(
        !values.contains(&"O"),
        "the frequent one-character claim code must lose to declaration overhead"
    );
    assert!(
        transformed
            .aliases
            .iter()
            .all(|alias| alias.net_bytes_saved > 0)
    );

    let second = alias_canonical_c1(&canonical).unwrap();
    assert_eq!(
        second, transformed,
        "same C1 input must alias deterministically"
    );

    let expanded = expand_c1_aliases(&transformed.encoded).unwrap();
    assert_eq!(expanded.as_bytes(), canonical.as_bytes());
    assert_eq!(compact_ir::decode_report(&expanded).unwrap(), report);
}

#[test]
fn representative_c1_report_remains_exact_after_optional_aliasing() {
    let report = representative_report();
    let canonical = compact_ir::encode_report(&report).unwrap();
    let transformed = alias_canonical_c1(&canonical).unwrap();
    let expanded = expand_c1_aliases(&transformed.encoded).unwrap();

    assert_eq!(expanded, canonical);
    assert_eq!(compact_ir::decode_report(&expanded).unwrap(), report);
    assert!(transformed.encoded_bytes <= transformed.canonical_bytes);
}

#[test]
fn tiny_no_repetition_report_stays_plain_c1() {
    let report = AnalysisReport {
        schema_version: 1,
        analysis: "a".to_string(),
        repository: "r".to_string(),
        claims: vec![Claim::new(ClaimKind::Observed, "m")],
        findings: Vec::new(),
    };
    let canonical = compact_ir::encode_report(&report).unwrap();
    let transformed = alias_canonical_c1(&canonical).unwrap();

    assert!(!transformed.used_aliases());
    assert_eq!(transformed.encoded, canonical);
    assert_eq!(transformed.encoded_bytes, transformed.canonical_bytes);
    assert_eq!(expand_c1_aliases(&transformed.encoded).unwrap(), canonical);
}

#[test]
fn alias_expansion_is_strict_and_only_resolves_tokens_outside_json_strings() {
    let packet = concat!(
        "C1A\n",
        "A1 \"replacement\"\n",
        "C1\n",
        "R[1,\"literal @1 stays literal\",\"repo\"]\n",
        "C[\"O\",@1]\n",
    );
    let expanded = expand_c1_aliases(packet).unwrap();
    assert_eq!(
        expanded,
        concat!(
            "C1\n",
            "R[1,\"literal @1 stays literal\",\"repo\"]\n",
            "C[\"O\",\"replacement\"]\n",
        )
    );

    assert!(expand_c1_aliases("C1A\nC1\nR[1,\"a\",\"r\"]\n").is_err());
    assert!(expand_c1_aliases("C1A\nA2 \"x\"\nC1\nR[1,@2,\"r\"]\n").is_err());
    assert!(expand_c1_aliases("C1A\nA1 \"x\"\nA2 \"x\"\nC1\nR[1,@1,@2]\n").is_err());
    assert!(expand_c1_aliases("C1A\nA1 \"x\"\nC1\nR[1,@2,\"r\"]\n").is_err());
    assert!(expand_c1_aliases("C1A\nA1 \"x\"\nC1\nR[1,@x,\"r\"]\n").is_err());
    assert!(expand_c1_aliases("C1A\nA1 nope\nC1\nR[1,@1,\"r\"]\n").is_err());
}

#[test]
fn alias_expansion_enforces_encoded_and_canonical_byte_limits() {
    const RAW_LITERAL: &str = "\"0123456789\"";

    let available = compact_ir::MAX_C1_BYTES - "C1\n".len();
    let repeats = available / RAW_LITERAL.len();
    let remainder = available % RAW_LITERAL.len();
    let body = format!("C1\n{}{}", "@1".repeat(repeats), "x".repeat(remainder));
    let packet = format!("C1A\nA1 {RAW_LITERAL}\n{body}");
    assert!(packet.len() < compact_ir::MAX_C1_BYTES);

    let expanded = expand_c1_aliases(&packet).unwrap();
    assert_eq!(expanded.len(), compact_ir::MAX_C1_BYTES);

    let oversized_body = format!("{body}@1");
    let oversized_packet = format!("C1A\nA1 {RAW_LITERAL}\n{oversized_body}");
    assert!(oversized_packet.len() < compact_ir::MAX_C1_BYTES);
    let error = expand_c1_aliases(&oversized_packet)
        .unwrap_err()
        .to_string();
    assert!(error.contains("expanded canonical C1 exceeds"));

    let oversized_input = format!("C1A\n{}", "x".repeat(compact_ir::MAX_C1_BYTES));
    let error = expand_c1_aliases(&oversized_input).unwrap_err().to_string();
    assert!(error.contains("C1A input exceeds"));
}
