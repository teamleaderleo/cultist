use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

const INVENTORY_SCHEMA_VERSION: u32 = 1;
const MAX_INVENTORY_BYTES: usize = 1024 * 1024;
const MAX_ACTIVE_WORK: usize = 128;
const MAX_CHANGED_PATHS: usize = 1024;
const MAX_EDGES: usize = 512;
const MAX_ID_BYTES: usize = 128;
const MAX_KIND_BYTES: usize = 64;
const MAX_TITLE_BYTES: usize = 512;
const MAX_URL_BYTES: usize = 2048;
const MAX_REF_BYTES: usize = 512;
const MAX_SHA_BYTES: usize = 128;
const MAX_TIME_BYTES: usize = 128;
const MAX_SOURCE_BYTES: usize = 512;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveWorkInventory {
    schema_version: u32,
    source: String,
    observed_at: String,
    current: WorkItem,
    active_work: Vec<WorkItem>,
    #[serde(default)]
    coordination_edges: Vec<CoordinationEdge>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkItem {
    id: String,
    kind: String,
    title: String,
    url: String,
    head_ref: String,
    head_sha: String,
    updated_at: String,
    draft: bool,
    #[serde(default)]
    activity: WorkActivity,
    changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WorkActivity {
    #[default]
    ConfirmedActive,
    Preparation,
    Unresolved,
}

impl WorkActivity {
    fn as_str(self) -> &'static str {
        match self {
            Self::ConfirmedActive => "confirmed_active",
            Self::Preparation => "preparation",
            Self::Unresolved => "unresolved",
        }
    }
}

impl WorkItem {
    fn activity(&self) -> WorkActivity {
        self.activity
    }
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CoordinationKind {
    DependsOn,
    Blocks,
    HoldMergeWhile,
    Supersedes,
}

impl CoordinationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::DependsOn => "depends_on",
            Self::Blocks => "blocks",
            Self::HoldMergeWhile => "hold_merge_while",
            Self::Supersedes => "supersedes",
        }
    }

    fn question(self) -> &'static str {
        match self {
            Self::DependsOn => {
                "Should the dependency be integrated or settled before these changes proceed independently?"
            }
            Self::Blocks => {
                "Should the blocked change pause, rebase, or coordinate ownership before proceeding?"
            }
            Self::HoldMergeWhile => {
                "Should merge order be coordinated before either change advances the shared evidence baseline?"
            }
            Self::Supersedes => {
                "Should the superseded change be retired or reconciled before parallel work continues?"
            }
        }
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinationEdge {
    kind: CoordinationKind,
    from: String,
    to: String,
    source: String,
}

#[derive(Debug)]
struct ValidatedInventory {
    source: String,
    observed_at: String,
    current: WorkItem,
    active_work: Vec<WorkItem>,
    by_id: BTreeMap<String, WorkItem>,
    coordination_edges: Vec<CoordinationEdge>,
}

pub fn build_active_inventory_analysis_report(
    root: &Path,
    inventory_path: &Path,
    scope: Option<&Path>,
) -> Result<AnalysisReport, Box<dyn Error>> {
    let bytes = read_bounded_inventory(inventory_path)?;
    let inventory = validate_inventory(serde_json::from_slice(&bytes)?)?;
    Ok(analyze_inventory(root, &inventory, scope))
}

fn analyze_inventory(
    root: &Path,
    inventory: &ValidatedInventory,
    scope: Option<&Path>,
) -> AnalysisReport {
    let mut analysis = AnalysisReport::new(
        "preflight-active-inventory",
        root.to_string_lossy().into_owned(),
    );

    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Admitted active-work inventory schema v{INVENTORY_SCHEMA_VERSION} from `{}` observed at `{}`.",
            inventory.source, inventory.observed_at
        ),
    ));
    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Current work `{}` is `{}` at head `{}` (updated `{}`, draft={}).",
            inventory.current.id,
            inventory.current.title,
            inventory.current.head_sha,
            inventory.current.updated_at,
            inventory.current.draft
        ),
    ));

    let current_paths = scoped_paths(&inventory.current.changed_paths, scope);
    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Current work records {} changed path(s) in the selected scope.",
            current_paths.len()
        ),
    ));

    let mut direct_overlap_count = 0usize;
    let mut candidates_examined = 0usize;
    let mut self_candidates_excluded = 0usize;

    for work in &inventory.active_work {
        if same_work(&inventory.current, work) {
            self_candidates_excluded += 1;
            continue;
        }
        candidates_examined += 1;
        let other_paths = scoped_paths(&work.changed_paths, scope);
        for path in current_paths.intersection(&other_paths) {
            direct_overlap_count += 1;
            let display = path.to_string_lossy().into_owned();
            analysis
                .findings
                .push(path_overlap_finding(&inventory.current, work, display));
        }
    }

    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Examined {candidates_examined} supplied work candidate(s) and excluded {self_candidates_excluded} self candidate(s)."
        ),
    ));

    if direct_overlap_count == 0 {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "The admitted inventory records no direct path overlap between the current work and the other supplied work in the selected scope.",
        ));
    }

    let mut current_edge_count = 0usize;
    for edge in &inventory.coordination_edges {
        if edge.from != inventory.current.id && edge.to != inventory.current.id {
            continue;
        }
        current_edge_count += 1;
        let other_id = if edge.from == inventory.current.id {
            edge.to.as_str()
        } else {
            edge.from.as_str()
        };
        let other = inventory
            .by_id
            .get(other_id)
            .expect("validated coordination endpoint exists");

        let mut finding = Finding::new(
            "preflight-explicit-coordination",
            "Explicit coordination edge",
        )
        .with_claim(
            Claim::new(
                ClaimKind::Observed,
                format!(
                    "The admitted inventory records `{}` from `{}` to `{}`.",
                    edge.kind.as_str(),
                    edge.from,
                    edge.to
                ),
            )
            .with_evidence(Evidence::new(format!(
                "Coordination source reference: `{}`.",
                edge.source
            )))
            .with_evidence(Evidence::new(format!(
                "Related supplied work: `{}` — `{}` at head `{}` (updated `{}`, activity={}).",
                other.id,
                other.title,
                other.head_sha,
                other.updated_at,
                other.activity().as_str()
            ))),
        );

        if !pair_has_scoped_overlap(&inventory.current, other, scope) {
            finding = finding.with_claim(Claim::new(
                ClaimKind::Observed,
                format!(
                    "The admitted inventory records no direct path overlap between `{}` and `{other_id}` in the selected scope.",
                    inventory.current.id
                ),
            ));
        }

        if other.activity() == WorkActivity::Unresolved {
            finding = finding.with_claim(activity_unknown_claim(other));
        }

        finding = finding
            .with_claim(Claim::new(
                ClaimKind::Unknown,
                "The inventory does not establish the operational consequence or intent beyond the declared coordination relation.",
            ))
            .with_question(edge.kind.question());
        analysis.findings.push(finding);
    }

    if current_edge_count == 0 {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "The admitted inventory contains no explicit coordination edge involving the current work.",
        ));
    }

    analysis.claims.push(Claim::new(
        ClaimKind::Unknown,
        "No direct path overlap is not evidence that supplied work is semantically independent.",
    ));
    analysis.claims.push(Claim::new(
        ClaimKind::Unknown,
        "Inventory mode does not independently fetch provider objects or infer generated, historical, policy, behavioral, ownership, or incompatibility relationships absent from the supplied snapshot.",
    ));

    analysis
}

fn path_overlap_finding(current: &WorkItem, other: &WorkItem, display: String) -> Finding {
    let observed = Claim::new(
        ClaimKind::Observed,
        format!(
            "The admitted inventory records both `{}` and `{}` modifying `{display}`.",
            current.id, other.id
        ),
    )
    .with_evidence(Evidence::new(format!(
        "Other supplied work: `{}` — `{}` at head `{}` (updated `{}`, activity={}).",
        other.id,
        other.title,
        other.head_sha,
        other.updated_at,
        other.activity().as_str()
    )))
    .with_evidence(Evidence::new(format!(
        "Provider reference: `{}`.",
        other.url
    )));

    if other.activity() == WorkActivity::Unresolved {
        Finding::new(
            "preflight-inventory-path-overlap-activity-unknown",
            "Path overlap with unresolved activity",
        )
        .at(Location::new(display, None))
        .with_claim(observed)
        .with_claim(activity_unknown_claim(other))
        .with_question(
            "Refresh or resolve current activity before treating this path overlap as an active collision.",
        )
    } else {
        Finding::new(
            "preflight-inventory-path-overlap",
            "Active-change path overlap",
        )
        .at(Location::new(display, None))
        .with_claim(observed)
        .with_question("Is there anything worth reconciling before continuing on this path?")
    }
}

fn activity_unknown_claim(work: &WorkItem) -> Claim {
    Claim::new(
        ClaimKind::Unknown,
        format!(
            "The admitted inventory does not establish that `{}` is currently active or owned.",
            work.id
        ),
    )
}

fn same_work(current: &WorkItem, other: &WorkItem) -> bool {
    current.id == other.id
}

fn pair_has_scoped_overlap(current: &WorkItem, other: &WorkItem, scope: Option<&Path>) -> bool {
    let current = scoped_paths(&current.changed_paths, scope);
    let other = scoped_paths(&other.changed_paths, scope);
    current.intersection(&other).next().is_some()
}

fn scoped_paths(paths: &[String], scope: Option<&Path>) -> BTreeSet<PathBuf> {
    paths
        .iter()
        .map(PathBuf::from)
        .filter(|path| scope.is_none_or(|scope| path.starts_with(scope)))
        .collect()
}

fn read_bounded_inventory(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_INVENTORY_BYTES as u64 {
        return Err(
            format!("active-work inventory exceeds the {MAX_INVENTORY_BYTES}-byte limit").into(),
        );
    }

    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_INVENTORY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_INVENTORY_BYTES {
        return Err(
            format!("active-work inventory exceeds the {MAX_INVENTORY_BYTES}-byte limit").into(),
        );
    }
    Ok(bytes)
}

fn validate_inventory(
    mut document: ActiveWorkInventory,
) -> Result<ValidatedInventory, Box<dyn Error>> {
    if document.schema_version != INVENTORY_SCHEMA_VERSION {
        return Err(format!(
            "unsupported active-work inventory schema {}; expected {INVENTORY_SCHEMA_VERSION}",
            document.schema_version
        )
        .into());
    }
    validate_bounded_text(&document.source, "source", MAX_SOURCE_BYTES, false)?;
    validate_bounded_text(&document.observed_at, "observed_at", MAX_TIME_BYTES, false)?;

    if document.active_work.len() > MAX_ACTIVE_WORK {
        return Err(
            format!("active-work inventory exceeds the {MAX_ACTIVE_WORK}-candidate limit").into(),
        );
    }
    if document.coordination_edges.len() > MAX_EDGES {
        return Err(format!("active-work inventory exceeds the {MAX_EDGES}-edge limit").into());
    }

    let mut total_paths = 0usize;
    normalize_work(&mut document.current, &mut total_paths)?;
    for work in &mut document.active_work {
        normalize_work(work, &mut total_paths)?;
    }

    let mut by_id = BTreeMap::new();
    by_id.insert(document.current.id.clone(), document.current.clone());
    for work in &document.active_work {
        if same_work(&document.current, work) {
            continue;
        }
        if by_id.insert(work.id.clone(), work.clone()).is_some() {
            return Err(format!("duplicate active work id `{}`", work.id).into());
        }
    }

    let mut seen_edges = BTreeSet::new();
    for edge in &document.coordination_edges {
        validate_id(&edge.from)?;
        validate_id(&edge.to)?;
        validate_bounded_text(&edge.source, "coordination source", MAX_SOURCE_BYTES, false)?;
        if edge.from == edge.to {
            return Err("coordination edge endpoints must be distinct".into());
        }
        if !by_id.contains_key(&edge.from) {
            return Err(format!(
                "coordination edge references missing active work `{}`",
                edge.from
            )
            .into());
        }
        if !by_id.contains_key(&edge.to) {
            return Err(format!(
                "coordination edge references missing active work `{}`",
                edge.to
            )
            .into());
        }
        if !seen_edges.insert((
            edge.kind,
            edge.from.clone(),
            edge.to.clone(),
            edge.source.clone(),
        )) {
            return Err(format!(
                "duplicate coordination edge `{}` from `{}` to `{}`",
                edge.kind.as_str(),
                edge.from,
                edge.to
            )
            .into());
        }
    }

    Ok(ValidatedInventory {
        source: document.source,
        observed_at: document.observed_at,
        current: document.current,
        active_work: document.active_work,
        by_id,
        coordination_edges: document.coordination_edges,
    })
}

fn normalize_work(work: &mut WorkItem, total_paths: &mut usize) -> Result<(), Box<dyn Error>> {
    validate_id(&work.id)?;
    validate_bounded_text(&work.kind, "work kind", MAX_KIND_BYTES, true)?;
    validate_bounded_text(&work.title, "work title", MAX_TITLE_BYTES, false)?;
    validate_bounded_text(&work.url, "work url", MAX_URL_BYTES, false)?;
    validate_bounded_text(&work.head_ref, "head ref", MAX_REF_BYTES, true)?;
    validate_bounded_text(&work.head_sha, "head sha", MAX_SHA_BYTES, true)?;
    validate_bounded_text(&work.updated_at, "updated_at", MAX_TIME_BYTES, false)?;

    let mut normalized = BTreeSet::new();
    for raw_path in &work.changed_paths {
        let path = validate_relative_path(raw_path)?;
        if !normalized.insert(path) {
            return Err(format!(
                "active work `{}` contains duplicate path `{raw_path}`",
                work.id
            )
            .into());
        }
        *total_paths += 1;
        if *total_paths > MAX_CHANGED_PATHS {
            return Err(format!(
                "active-work inventory exceeds the {MAX_CHANGED_PATHS}-path limit"
            )
            .into());
        }
    }
    work.changed_paths = normalized
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    Ok(())
}

fn validate_id(value: &str) -> Result<(), Box<dyn Error>> {
    validate_bounded_text(value, "work id", MAX_ID_BYTES, true)
}

fn validate_bounded_text(
    value: &str,
    label: &str,
    max_bytes: usize,
    token: bool,
) -> Result<(), Box<dyn Error>> {
    if value.is_empty() || value.len() > max_bytes {
        return Err(format!("{label} must contain 1..={max_bytes} bytes").into());
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} contains a control character").into());
    }
    if token && !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(format!("{label} must contain only printable non-space ASCII").into());
    }
    Ok(())
}

fn validate_relative_path(raw: &str) -> Result<PathBuf, Box<dyn Error>> {
    if raw.is_empty() || raw.len() > MAX_PATH_BYTES {
        return Err(format!("changed path must contain 1..={MAX_PATH_BYTES} bytes").into());
    }
    if raw.contains('\\') {
        return Err(format!("changed path `{raw}` must use `/` separators").into());
    }

    let path = Path::new(raw);
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            _ => {
                return Err(format!(
                    "changed path `{raw}` must be a canonical relative path without traversal"
                )
                .into());
            }
        }
    }
    if parts.is_empty() || parts.join("/") != raw {
        return Err(format!("changed path `{raw}` is not canonical").into());
    }
    Ok(PathBuf::from(raw))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn work(id: &str, sha: &str, paths: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "kind": "pull_request",
            "title": format!("Work {id}"),
            "url": format!("https://example.invalid/{id}"),
            "head_ref": format!("branch-{id}"),
            "head_sha": sha,
            "updated_at": "2026-08-19T00:00:00Z",
            "draft": false,
            "changed_paths": paths,
        })
    }

    fn work_with_activity(
        id: &str,
        sha: &str,
        paths: &[&str],
        activity: &str,
    ) -> serde_json::Value {
        let mut value = work(id, sha, paths);
        value["activity"] = serde_json::json!(activity);
        value
    }

    fn document(
        current: serde_json::Value,
        active_work: Vec<serde_json::Value>,
        edges: Vec<serde_json::Value>,
    ) -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "source": "github_pull_requests",
            "observed_at": "2026-08-19T00:01:00Z",
            "current": current,
            "active_work": active_work,
            "coordination_edges": edges,
        })
    }

    fn parse(value: serde_json::Value) -> Result<ValidatedInventory, Box<dyn Error>> {
        validate_inventory(serde_json::from_value(value)?)
    }

    fn unique_temp_file(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cultist-active-inventory-{name}-{}-{nanos}.json",
            std::process::id()
        ))
    }

    #[test]
    fn accepts_landed_research_inventory_without_edges() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let inventory = parse(document(
            current.clone(),
            vec![current, work("#2", "bbb", &["src/b.rs"])],
            Vec::new(),
        ))
        .unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(analysis.findings.is_empty());
        assert!(
            analysis
                .claims
                .iter()
                .any(|claim| { claim.message.contains("excluded 1 self candidate") })
        );
    }

    #[test]
    fn distinct_work_ids_remain_distinct_when_their_heads_match() {
        let current = work("#1", "shared", &["src/a.rs"]);
        let other = work("#2", "shared", &["src/a.rs"]);
        let edge = serde_json::json!({
            "kind": "depends_on",
            "from": "#1",
            "to": "#2",
            "source": "provider:shared-head"
        });
        let inventory = parse(document(current, vec![other], vec![edge])).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(
            analysis
                .findings
                .iter()
                .any(|finding| { finding.kind == "preflight-inventory-path-overlap" })
        );
        assert!(
            analysis
                .findings
                .iter()
                .any(|finding| { finding.kind == "preflight-explicit-coordination" })
        );
        assert!(
            analysis
                .claims
                .iter()
                .any(|claim| { claim.message.contains("excluded 0 self candidate") })
        );
    }

    #[test]
    fn explicit_coordination_survives_disjoint_paths() {
        let current = work("#748", "aaa", &["preflight-cli/src/AgentJarStaging.java"]);
        let other = work(
            "#703",
            "bbb",
            &["preflight-desktop/src/report_authority.rs"],
        );
        let edge = serde_json::json!({
            "kind": "hold_merge_while",
            "from": "#748",
            "to": "#703",
            "source": "github:pull/748"
        });
        let inventory = parse(document(current, vec![other], vec![edge])).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(
            analysis
                .findings
                .iter()
                .all(|finding| { finding.kind != "preflight-inventory-path-overlap" })
        );
        let coordination: Vec<_> = analysis
            .findings
            .iter()
            .filter(|finding| finding.kind == "preflight-explicit-coordination")
            .collect();
        assert_eq!(coordination.len(), 1);
        assert!(coordination[0].claims.iter().any(|claim| {
            claim.kind == ClaimKind::Observed && claim.message.contains("hold_merge_while")
        }));
        assert!(
            coordination[0]
                .claims
                .iter()
                .any(|claim| claim.kind == ClaimKind::Unknown)
        );
        assert!(
            coordination[0].claims[0]
                .evidence
                .iter()
                .any(|evidence| { evidence.message.contains("github:pull/748") })
        );
    }

    #[test]
    fn supplied_path_overlap_is_observed() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let other = work("#2", "bbb", &["src/a.rs"]);
        let inventory = parse(document(current, vec![other], Vec::new())).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        let overlap = analysis
            .findings
            .iter()
            .find(|finding| finding.kind == "preflight-inventory-path-overlap")
            .unwrap();
        assert_eq!(overlap.claims[0].kind, ClaimKind::Observed);
    }

    #[test]
    fn confirmed_active_overlap_keeps_active_collision_finding() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let other = work_with_activity("#2", "bbb", &["src/a.rs"], "confirmed_active");
        let inventory = parse(document(current, vec![other], Vec::new())).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(analysis.findings.iter().any(|finding| {
            finding.kind == "preflight-inventory-path-overlap"
                && finding.title == "Active-change path overlap"
        }));
    }

    #[test]
    fn preparation_overlap_keeps_active_collision_finding() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let other = work_with_activity("#2", "bbb", &["src/a.rs"], "preparation");
        let inventory = parse(document(current, vec![other], Vec::new())).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(analysis.findings.iter().any(|finding| {
            finding.kind == "preflight-inventory-path-overlap"
                && finding.title == "Active-change path overlap"
        }));
    }

    #[test]
    fn unresolved_overlap_preserves_path_fact_and_activity_unknown() {
        let current = work("#1", "aaa", &["tests/regression.rs"]);
        let other = work_with_activity("branch-old", "bbb", &["tests/regression.rs"], "unresolved");
        let inventory = parse(document(current, vec![other], Vec::new())).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(
            analysis
                .findings
                .iter()
                .all(|finding| { finding.kind != "preflight-inventory-path-overlap" })
        );
        let finding = analysis
            .findings
            .iter()
            .find(|finding| finding.kind == "preflight-inventory-path-overlap-activity-unknown")
            .unwrap();
        assert_eq!(finding.title, "Path overlap with unresolved activity");
        assert!(finding.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Observed
                && claim.message.contains("modifying `tests/regression.rs`")
        }));
        assert!(finding.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Unknown && claim.message.contains("currently active or owned")
        }));
        assert!(
            finding
                .question
                .as_deref()
                .is_some_and(|question| question.contains("Refresh or resolve current activity"))
        );
    }

    #[test]
    fn unresolved_disjoint_work_stays_quiet() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let other = work_with_activity("branch-old", "bbb", &["src/b.rs"], "unresolved");
        let inventory = parse(document(current, vec![other], Vec::new())).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(analysis.findings.is_empty());
    }

    #[test]
    fn kind_spelling_alone_does_not_make_activity_unresolved() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let mut other = work("#2", "bbb", &["src/a.rs"]);
        other["kind"] = serde_json::json!("branch_observation_ambiguous");
        let inventory = parse(document(current, vec![other], Vec::new())).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(
            analysis
                .findings
                .iter()
                .any(|finding| { finding.kind == "preflight-inventory-path-overlap" })
        );
    }

    #[test]
    fn explicit_coordination_survives_unresolved_activity() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let other = work_with_activity("#2", "bbb", &["src/b.rs"], "unresolved");
        let edge = serde_json::json!({
            "kind": "depends_on",
            "from": "#1",
            "to": "#2",
            "source": "fixture"
        });
        let inventory = parse(document(current, vec![other], vec![edge])).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        let finding = analysis
            .findings
            .iter()
            .find(|finding| finding.kind == "preflight-explicit-coordination")
            .unwrap();
        assert!(finding.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Unknown && claim.message.contains("currently active or owned")
        }));
    }

    #[test]
    fn rejects_null_activity_state() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let mut other = work("#2", "bbb", &["src/a.rs"]);
        other["activity"] = serde_json::Value::Null;
        assert!(parse(document(current, vec![other], Vec::new())).is_err());
    }

    #[test]
    fn rejects_unknown_activity_state() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let other = work_with_activity("#2", "bbb", &["src/a.rs"], "maybe_active");
        assert!(parse(document(current, vec![other], Vec::new())).is_err());
    }

    #[test]
    fn unrelated_edges_are_ignored_for_current_work() {
        let current = work("#1", "aaa", &["a"]);
        let two = work("#2", "bbb", &["b"]);
        let three = work("#3", "ccc", &["c"]);
        let edge = serde_json::json!({
            "kind": "depends_on",
            "from": "#2",
            "to": "#3",
            "source": "github:pull/2"
        });
        let inventory = parse(document(current, vec![two, three], vec![edge])).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim
                .message
                .contains("no explicit coordination edge involving the current work")
        }));
    }

    #[test]
    fn rejects_unknown_edge_kind() {
        let value = document(
            work("#1", "aaa", &["a"]),
            vec![work("#2", "bbb", &["b"])],
            vec![serde_json::json!({
                "kind": "vibes_with",
                "from": "#1",
                "to": "#2",
                "source": "fixture"
            })],
        );
        assert!(parse(value).is_err());
    }

    #[test]
    fn rejects_missing_edge_endpoint() {
        let value = document(
            work("#1", "aaa", &["a"]),
            vec![work("#2", "bbb", &["b"])],
            vec![serde_json::json!({
                "kind": "depends_on",
                "from": "#1",
                "to": "#999",
                "source": "fixture"
            })],
        );
        assert!(parse(value).is_err());
    }

    #[test]
    fn rejects_duplicate_active_work_id() {
        let value = document(
            work("#1", "aaa", &["a"]),
            vec![work("#2", "bbb", &["b"]), work("#2", "ccc", &["c"])],
            Vec::new(),
        );
        assert!(parse(value).is_err());
    }

    #[test]
    fn rejects_duplicate_edge() {
        let edge = serde_json::json!({
            "kind": "depends_on",
            "from": "#1",
            "to": "#2",
            "source": "fixture"
        });
        let value = document(
            work("#1", "aaa", &["a"]),
            vec![work("#2", "bbb", &["b"])],
            vec![edge.clone(), edge],
        );
        assert!(parse(value).is_err());
    }

    #[test]
    fn rejects_traversing_path() {
        let value = document(work("#1", "aaa", &["../outside"]), Vec::new(), Vec::new());
        assert!(parse(value).is_err());
    }

    #[test]
    fn rejects_oversized_inventory_file() {
        let path = unique_temp_file("oversized");
        fs::write(&path, vec![b'x'; MAX_INVENTORY_BYTES + 1]).unwrap();
        assert!(build_active_inventory_analysis_report(Path::new("/repo"), &path, None).is_err());
        fs::remove_file(path).unwrap();
    }
}
