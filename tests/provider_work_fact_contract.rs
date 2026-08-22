use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};

const WORK_FACT_SCHEMA_VERSION: u32 = 0;
const MAX_PATH_BYTES: usize = 4096;
const MAX_SOURCE_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ActivityInput {
    ConfirmedActive,
    Preparation,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum CoordinationKindInput {
    DependsOn,
    Blocks,
    HoldMergeWhile,
    Supersedes,
}

#[derive(Clone, Debug)]
struct WorkInput<'a> {
    id: &'a str,
    head_sha: &'a str,
    activity: ActivityInput,
    changed_paths: Vec<&'a str>,
}

#[derive(Clone, Debug)]
struct CoordinationInput<'a> {
    kind: CoordinationKindInput,
    from: &'a str,
    to: &'a str,
    source: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct CanonicalWorkFact {
    id: String,
    head_sha: String,
    activity: ActivityInput,
    changed_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct SemanticCoordinationEdge {
    kind: CoordinationKindInput,
    from: String,
    to: String,
}

#[derive(Serialize)]
struct WorkFactDocument {
    schema_version: u32,
    work: Vec<CanonicalWorkFact>,
    coordination_edges: Vec<SemanticCoordinationEdge>,
}

fn canonical_work_id(raw: &str) -> Result<String, String> {
    let digits = raw
        .strip_prefix("pull/")
        .ok_or_else(|| format!("work id `{raw}` must use canonical `pull/<number>` form"))?;
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(format!(
            "work id `{raw}` must use a positive canonical decimal number"
        ));
    }
    Ok(format!("pull/{digits}"))
}

fn canonical_head_sha(raw: &str) -> Result<String, String> {
    if raw.len() != 40 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("head sha must contain exactly 40 hexadecimal characters".to_string());
    }
    Ok(raw.to_ascii_lowercase())
}

fn canonical_path(raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw.len() > MAX_PATH_BYTES {
        return Err(format!(
            "changed path must contain 1..={MAX_PATH_BYTES} bytes"
        ));
    }
    if raw.contains('\\') {
        return Err(format!("changed path `{raw}` must use `/` separators"));
    }
    if raw.chars().any(char::is_control) {
        return Err(format!("changed path `{raw}` contains a control character"));
    }

    let mut parts = 0usize;
    for part in raw.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(format!(
                "changed path `{raw}` must be a canonical relative path without traversal"
            ));
        }
        parts += 1;
    }
    if parts == 0 {
        return Err(format!("changed path `{raw}` is not canonical"));
    }
    Ok(raw.to_string())
}

fn validate_source(source: &str) -> Result<(), String> {
    if source.is_empty() || source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "coordination source must contain 1..={MAX_SOURCE_BYTES} bytes"
        ));
    }
    if source.chars().any(char::is_control) {
        return Err("coordination source contains a control character".to_string());
    }
    Ok(())
}

fn canonical_work(input: &WorkInput<'_>) -> Result<CanonicalWorkFact, String> {
    let id = canonical_work_id(input.id)?;
    let head_sha = canonical_head_sha(input.head_sha)?;
    let mut paths = BTreeSet::new();
    for raw_path in &input.changed_paths {
        let path = canonical_path(raw_path)?;
        if !paths.insert(path.clone()) {
            return Err(format!("work `{id}` contains duplicate path `{path}`"));
        }
    }

    Ok(CanonicalWorkFact {
        id,
        head_sha,
        activity: input.activity,
        changed_paths: paths.into_iter().collect(),
    })
}

fn semantic_edge(input: &CoordinationInput<'_>) -> Result<SemanticCoordinationEdge, String> {
    validate_source(input.source)?;
    let from = canonical_work_id(input.from)?;
    let to = canonical_work_id(input.to)?;
    if from == to {
        return Err("coordination edge endpoints must be distinct".to_string());
    }
    Ok(SemanticCoordinationEdge {
        kind: input.kind,
        from,
        to,
    })
}

fn fingerprint(
    work: &[WorkInput<'_>],
    coordination: &[CoordinationInput<'_>],
) -> Result<String, String> {
    let mut work_by_id = BTreeMap::new();
    for input in work {
        let item = canonical_work(input)?;
        if work_by_id.insert(item.id.clone(), item).is_some() {
            return Err(format!("duplicate work id `{}`", input.id));
        }
    }

    let mut exact_coordination_inputs = BTreeSet::new();
    let mut semantic_edges = BTreeSet::new();
    for input in coordination {
        let edge = semantic_edge(input)?;
        if !work_by_id.contains_key(&edge.from) {
            return Err(format!(
                "coordination edge references missing work `{}`",
                edge.from
            ));
        }
        if !work_by_id.contains_key(&edge.to) {
            return Err(format!(
                "coordination edge references missing work `{}`",
                edge.to
            ));
        }

        let exact = (
            input.kind,
            edge.from.clone(),
            edge.to.clone(),
            input.source.to_string(),
        );
        if !exact_coordination_inputs.insert(exact) {
            return Err(format!(
                "duplicate coordination evidence for `{}` -> `{}`",
                edge.from, edge.to
            ));
        }
        semantic_edges.insert(edge);
    }

    let document = WorkFactDocument {
        schema_version: WORK_FACT_SCHEMA_VERSION,
        work: work_by_id.into_values().collect(),
        coordination_edges: semantic_edges.into_iter().collect(),
    };
    let bytes = serde_json::to_vec(&document).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(encoded)
}

fn baseline_work() -> Vec<WorkInput<'static>> {
    vec![
        WorkInput {
            id: "pull/604",
            head_sha: "63eece80df17a97a8544c4d716feca4fad1970ea",
            activity: ActivityInput::ConfirmedActive,
            changed_paths: vec!["AGENTS.md", "docs/agent-native-operating-mode.md"],
        },
        WorkInput {
            id: "pull/608",
            head_sha: "515c60f694664f3b691bfd7f920e4740d75226d1",
            activity: ActivityInput::ConfirmedActive,
            changed_paths: vec!["src/quarry/research_ir.py", "tests/test_research_ir.py"],
        },
    ]
}

fn edge<'a>(source: &'a str) -> CoordinationInput<'a> {
    CoordinationInput {
        kind: CoordinationKindInput::DependsOn,
        from: "pull/604",
        to: "pull/608",
        source,
    }
}

#[test]
fn work_path_and_edge_order_are_identity_invariant() {
    let first_work = baseline_work();
    let first_edges = vec![
        edge("provider:pull/604"),
        CoordinationInput {
            kind: CoordinationKindInput::Blocks,
            from: "pull/608",
            to: "pull/604",
            source: "provider:pull/608",
        },
    ];

    let mut second_work = baseline_work();
    second_work.reverse();
    second_work[0].changed_paths.reverse();
    let mut second_edges = first_edges.clone();
    second_edges.reverse();

    assert_eq!(
        fingerprint(&first_work, &first_edges).unwrap(),
        fingerprint(&second_work, &second_edges).unwrap()
    );
}

#[test]
fn coordination_source_only_movement_preserves_identity() {
    assert_eq!(
        fingerprint(&baseline_work(), &[edge("provider:pull/604")]).unwrap(),
        fingerprint(&baseline_work(), &[edge("provider:reviewed-metadata")]).unwrap()
    );
}

#[test]
fn multiple_provenance_sources_for_same_semantic_edge_preserve_identity() {
    let single = vec![edge("provider:pull/604")];
    let multiple = vec![
        edge("provider:pull/604"),
        edge("provider:reviewed-metadata"),
    ];

    assert_eq!(
        fingerprint(&baseline_work(), &single).unwrap(),
        fingerprint(&baseline_work(), &multiple).unwrap()
    );
}

#[test]
fn semantic_coordination_change_changes_identity() {
    let baseline = vec![edge("provider:pull/604")];
    let changed = vec![CoordinationInput {
        kind: CoordinationKindInput::HoldMergeWhile,
        from: "pull/604",
        to: "pull/608",
        source: "provider:pull/604",
    }];

    assert_ne!(
        fingerprint(&baseline_work(), &baseline).unwrap(),
        fingerprint(&baseline_work(), &changed).unwrap()
    );
}

#[test]
fn equivalent_head_hex_case_preserves_identity() {
    let first = baseline_work();
    let mut second = baseline_work();
    second[0].head_sha = "63EECE80DF17A97A8544C4D716FECA4FAD1970EA";

    assert_eq!(
        fingerprint(&first, &[]).unwrap(),
        fingerprint(&second, &[]).unwrap()
    );
}

#[test]
fn head_movement_changes_identity() {
    let first = baseline_work();
    let mut second = baseline_work();
    second[0].head_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    assert_ne!(
        fingerprint(&first, &[]).unwrap(),
        fingerprint(&second, &[]).unwrap()
    );
}

#[test]
fn activity_change_changes_identity_without_merging_activity_semantics() {
    let first = baseline_work();
    let mut second = baseline_work();
    second[0].activity = ActivityInput::Unresolved;

    assert_ne!(
        fingerprint(&first, &[]).unwrap(),
        fingerprint(&second, &[]).unwrap()
    );
}

#[test]
fn changed_path_set_changes_identity() {
    let first = baseline_work();
    let mut second = baseline_work();
    second[0].changed_paths = vec!["AGENTS.md", "src/new_collision_surface.rs"];

    assert_ne!(
        fingerprint(&first, &[]).unwrap(),
        fingerprint(&second, &[]).unwrap()
    );
}

#[test]
fn work_membership_change_changes_identity() {
    let first = baseline_work();
    let mut second = baseline_work();
    second.push(WorkInput {
        id: "pull/627",
        head_sha: "769ded20439efe0567d4553141598cfd3965a013",
        activity: ActivityInput::ConfirmedActive,
        changed_paths: vec!["tests/test_research_610_strict_carrier.py"],
    });

    assert_ne!(
        fingerprint(&first, &[]).unwrap(),
        fingerprint(&second, &[]).unwrap()
    );
}

#[test]
fn noncanonical_work_id_spellings_fail_closed() {
    for malformed in ["#604", "pull/0604", "pull/0", "PULL/604", "pull/604 "] {
        let mut work = baseline_work();
        work[0].id = malformed;
        assert!(fingerprint(&work, &[]).is_err(), "accepted `{malformed}`");
    }
}

#[test]
fn noncanonical_path_spellings_fail_closed() {
    for malformed in [
        "./src/lib.rs",
        "src//lib.rs",
        "src/../lib.rs",
        "src/./lib.rs",
        "src\\lib.rs",
        "/src/lib.rs",
        "src/lib.rs/",
    ] {
        let mut work = baseline_work();
        work[0].changed_paths = vec![malformed];
        assert!(fingerprint(&work, &[]).is_err(), "accepted `{malformed}`");
    }
}

#[test]
fn malformed_or_duplicate_work_facts_fail_closed() {
    let mut bad_head = baseline_work();
    bad_head[0].head_sha = "abc";
    assert!(fingerprint(&bad_head, &[]).is_err());

    let mut duplicate_path = baseline_work();
    duplicate_path[0].changed_paths = vec!["src/lib.rs", "src/lib.rs"];
    assert!(fingerprint(&duplicate_path, &[]).is_err());

    let mut duplicate_work = baseline_work();
    duplicate_work.push(WorkInput {
        id: "pull/604",
        head_sha: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        activity: ActivityInput::Preparation,
        changed_paths: Vec::new(),
    });
    assert!(fingerprint(&duplicate_work, &[]).is_err());
}

#[test]
fn malformed_coordination_fails_closed() {
    let exact_duplicate = vec![edge("provider:pull/604"), edge("provider:pull/604")];
    assert!(fingerprint(&baseline_work(), &exact_duplicate).is_err());

    let missing_endpoint = vec![CoordinationInput {
        kind: CoordinationKindInput::Supersedes,
        from: "pull/604",
        to: "pull/999",
        source: "provider:pull/604",
    }];
    assert!(fingerprint(&baseline_work(), &missing_endpoint).is_err());

    let self_edge = vec![CoordinationInput {
        kind: CoordinationKindInput::Blocks,
        from: "pull/604",
        to: "pull/604",
        source: "provider:pull/604",
    }];
    assert!(fingerprint(&baseline_work(), &self_edge).is_err());
}
