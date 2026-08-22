use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MAX_SNAPSHOT_BYTES: usize = 1024 * 1024;
const MAX_WORK_ITEMS: usize = 128;
const MAX_BODY_BYTES: usize = 128 * 1024;
const MAX_BODY_LINE_BYTES: usize = 32 * 1024;
const MAX_SOURCE_BYTES: usize = 512;
const MAX_KIND_BYTES: usize = 64;
const MAX_TIME_BYTES: usize = 128;
const MAX_EDGES: usize = 512;
const HOLD_PREFIX: &str = "Do not merge while #";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeError {
    message: String,
}

impl EdgeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for EdgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for EdgeError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkMetadataSnapshot {
    schema_version: u32,
    source: String,
    work: Vec<WorkMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkMetadata {
    id: String,
    kind: String,
    source: String,
    head_sha: String,
    updated_at: String,
    body: String,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationKind {
    HoldMergeWhile,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CoordinationEdge {
    pub kind: CoordinationKind,
    pub from: String,
    pub to: String,
    pub source: String,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceReceipt {
    pub kind: CoordinationKind,
    pub from: String,
    pub to: String,
    pub source: String,
    pub source_head_sha: String,
    pub source_updated_at: String,
    pub matched_clause: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize)]
pub struct ExtractionStats {
    pub work_items_examined: usize,
    pub operative_clauses_matched: usize,
    pub self_references_ignored: usize,
    pub unresolved_endpoints_ignored: usize,
    pub duplicate_edges_ignored: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ExtractionReport {
    pub schema_version: u32,
    pub analysis: String,
    pub source: String,
    pub coordination_edges: Vec<CoordinationEdge>,
    pub source_receipts: Vec<SourceReceipt>,
    pub unresolved_endpoint_receipts: Vec<SourceReceipt>,
    pub stats: ExtractionStats,
    pub unknowns: Vec<String>,
}

pub fn extract_snapshot(input: &str) -> Result<ExtractionReport, EdgeError> {
    if input.len() > MAX_SNAPSHOT_BYTES {
        return Err(EdgeError::new(format!(
            "coordination metadata snapshot is {} bytes; maximum is {MAX_SNAPSHOT_BYTES}",
            input.len()
        )));
    }

    let snapshot: WorkMetadataSnapshot = serde_json::from_str(input).map_err(|error| {
        EdgeError::new(format!("invalid coordination metadata snapshot: {error}"))
    })?;
    validate_snapshot(&snapshot)?;

    let known_ids = snapshot
        .work
        .iter()
        .map(|work| work.id.as_str())
        .collect::<BTreeSet<_>>();

    let mut edge_receipts = BTreeMap::<CoordinationEdge, SourceReceipt>::new();
    let mut unresolved_endpoint_receipts = BTreeSet::<SourceReceipt>::new();
    let mut stats = ExtractionStats {
        work_items_examined: snapshot.work.len(),
        ..ExtractionStats::default()
    };

    for work in &snapshot.work {
        let mut fence: Option<(char, usize)> = None;
        for raw_line in work.body.lines() {
            let line = raw_line.trim_end_matches('\r');
            if line.len() > MAX_BODY_LINE_BYTES {
                return Err(EdgeError::new(format!(
                    "{} body contains a line larger than {MAX_BODY_LINE_BYTES} bytes",
                    work.id
                )));
            }

            if let Some((marker, width)) = fence {
                if closes_fence(line, marker, width) {
                    fence = None;
                }
                continue;
            }
            if let Some(opened) = opens_fence(line) {
                fence = Some(opened);
                continue;
            }

            let Some(target) = hold_merge_target(line) else {
                continue;
            };
            stats.operative_clauses_matched += 1;

            if target == work.id {
                stats.self_references_ignored += 1;
                continue;
            }
            if !known_ids.contains(target.as_str()) {
                stats.unresolved_endpoints_ignored += 1;
                unresolved_endpoint_receipts.insert(SourceReceipt {
                    kind: CoordinationKind::HoldMergeWhile,
                    from: work.id.clone(),
                    to: target.clone(),
                    source: work.source.clone(),
                    source_head_sha: work.head_sha.clone(),
                    source_updated_at: work.updated_at.clone(),
                    matched_clause: line.to_string(),
                });
                if unresolved_endpoint_receipts.len() > MAX_EDGES {
                    return Err(EdgeError::new(format!(
                        "unresolved coordination endpoint receipt count exceeds maximum {MAX_EDGES}"
                    )));
                }
                continue;
            }

            let edge = CoordinationEdge {
                kind: CoordinationKind::HoldMergeWhile,
                from: work.id.clone(),
                to: target.clone(),
                source: work.source.clone(),
            };
            let receipt = SourceReceipt {
                kind: edge.kind,
                from: edge.from.clone(),
                to: edge.to.clone(),
                source: edge.source.clone(),
                source_head_sha: work.head_sha.clone(),
                source_updated_at: work.updated_at.clone(),
                matched_clause: line.to_string(),
            };

            if edge_receipts.insert(edge, receipt).is_some() {
                stats.duplicate_edges_ignored += 1;
            }
            if edge_receipts.len() > MAX_EDGES {
                return Err(EdgeError::new(format!(
                    "coordination edge count exceeds maximum {MAX_EDGES}"
                )));
            }
        }
    }

    let coordination_edges = edge_receipts.keys().cloned().collect::<Vec<_>>();
    let source_receipts = edge_receipts.values().cloned().collect::<Vec<_>>();
    let unresolved_endpoint_receipts = unresolved_endpoint_receipts.into_iter().collect();

    Ok(ExtractionReport {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        analysis: "explicit-coordination-metadata".to_string(),
        source: snapshot.source,
        coordination_edges,
        source_receipts,
        unresolved_endpoint_receipts,
        stats,
        unknowns: vec![
            "A body-derived edge proves only that the admitted source metadata contained the reviewed operative clause at the recorded head/update coordinates; implementation intent or continued applicability beyond that clause remains unknown without independent evidence.".to_string(),
        ],
    })
}

fn validate_snapshot(snapshot: &WorkMetadataSnapshot) -> Result<(), EdgeError> {
    if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(EdgeError::new(format!(
            "unsupported coordination metadata schema {}; expected {SNAPSHOT_SCHEMA_VERSION}",
            snapshot.schema_version
        )));
    }
    validate_bounded_nonempty("snapshot source", &snapshot.source, MAX_SOURCE_BYTES)?;
    if snapshot.work.len() > MAX_WORK_ITEMS {
        return Err(EdgeError::new(format!(
            "work item count {} exceeds maximum {MAX_WORK_ITEMS}",
            snapshot.work.len()
        )));
    }

    let mut ids = BTreeSet::new();
    for work in &snapshot.work {
        validate_work_id(&work.id)?;
        if !ids.insert(work.id.as_str()) {
            return Err(EdgeError::new(format!("duplicate work id `{}`", work.id)));
        }
        validate_bounded_nonempty("work kind", &work.kind, MAX_KIND_BYTES)?;
        if work.kind != "pull_request" {
            return Err(EdgeError::new(format!(
                "unsupported work kind `{}` for {}; expected `pull_request`",
                work.kind, work.id
            )));
        }
        validate_bounded_nonempty("work source", &work.source, MAX_SOURCE_BYTES)?;
        let expected_source = format!("github:pull/{}", &work.id[1..]);
        if work.source != expected_source {
            return Err(EdgeError::new(format!(
                "work source `{}` does not match canonical source `{expected_source}` for {}",
                work.source, work.id
            )));
        }
        if work.head_sha.len() != 40 || !work.head_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(EdgeError::new(format!(
                "{} head_sha must be exactly 40 hexadecimal characters",
                work.id
            )));
        }
        validate_bounded_nonempty("work updated_at", &work.updated_at, MAX_TIME_BYTES)?;
        if work.body.len() > MAX_BODY_BYTES {
            return Err(EdgeError::new(format!(
                "{} body is {} bytes; maximum is {MAX_BODY_BYTES}",
                work.id,
                work.body.len()
            )));
        }
    }

    Ok(())
}

fn validate_bounded_nonempty(label: &str, value: &str, maximum: usize) -> Result<(), EdgeError> {
    if value.is_empty() {
        return Err(EdgeError::new(format!("{label} must not be empty")));
    }
    if value.len() > maximum {
        return Err(EdgeError::new(format!(
            "{label} is {} bytes; maximum is {maximum}",
            value.len()
        )));
    }
    Ok(())
}

fn validate_work_id(id: &str) -> Result<(), EdgeError> {
    let Some(digits) = id.strip_prefix('#') else {
        return Err(EdgeError::new(format!(
            "invalid work id `{id}`; expected # followed by a positive decimal number"
        )));
    };
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(EdgeError::new(format!(
            "invalid work id `{id}`; expected canonical positive decimal form"
        )));
    }
    Ok(())
}

fn hold_merge_target(line: &str) -> Option<String> {
    let rest = line.strip_prefix(HOLD_PREFIX)?;
    let digit_count = rest
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return None;
    }

    let digits = &rest[..digit_count];
    if digits.starts_with('0') {
        return None;
    }
    let tail = &rest[digit_count..];
    if !tail.starts_with(' ') || tail.trim().is_empty() {
        return None;
    }

    Some(format!("#{digits}"))
}

fn fence_content(line: &str) -> Option<&str> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }
    Some(&line[indent..])
}

fn opens_fence(line: &str) -> Option<(char, usize)> {
    let content = fence_content(line)?;
    let first = content.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let width = content
        .chars()
        .take_while(|character| *character == first)
        .count();
    (width >= 3).then_some((first, width))
}

fn closes_fence(line: &str, marker: char, minimum_width: usize) -> bool {
    let Some(content) = fence_content(line) else {
        return false;
    };
    let width = content
        .chars()
        .take_while(|character| *character == marker)
        .count();
    width >= minimum_width && content[width..].trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sha(byte: char) -> String {
        std::iter::repeat_n(byte, 40).collect()
    }

    fn snapshot(entries: Vec<serde_json::Value>) -> String {
        serde_json::to_string(&json!({
            "schema_version": 1,
            "source": "test",
            "work": entries,
        }))
        .unwrap()
    }

    fn work(id: &str, body: &str) -> serde_json::Value {
        json!({
            "id": id,
            "kind": "pull_request",
            "source": format!("github:pull/{}", &id[1..]),
            "head_sha": sha('a'),
            "updated_at": "2026-08-19T00:00:00Z",
            "body": body,
        })
    }

    #[test]
    fn extracts_only_reviewed_top_level_hold_clause() {
        let input = snapshot(vec![
            work(
                "#748",
                "Refs #703.\nDo not merge while #703 is using current-main package evidence; keep the baseline still.\n",
            ),
            work("#703", ""),
        ]);
        let report = extract_snapshot(&input).unwrap();
        assert_eq!(report.coordination_edges.len(), 1);
        assert_eq!(report.coordination_edges[0].from, "#748");
        assert_eq!(report.coordination_edges[0].to, "#703");
        assert_eq!(report.coordination_edges[0].source, "github:pull/748");
        assert_eq!(report.source_receipts[0].source_head_sha, sha('a'));
    }

    #[test]
    fn ordinary_reference_language_stays_quiet() {
        let input = snapshot(vec![
            work(
                "#748",
                "Refs #703\nRelated: #703\nParent: #703\nsee #703\nafter discussing #703\nthis may conflict with #703\nCompeting replacement experiment for #703\n",
            ),
            work("#703", ""),
        ]);
        let report = extract_snapshot(&input).unwrap();
        assert!(report.coordination_edges.is_empty());
    }

    #[test]
    fn quoted_fenced_indented_and_list_examples_stay_quiet() {
        let input = snapshot(vec![
            work(
                "#748",
                "> Do not merge while #703 is active\n```text\nDo not merge while #703 is active\n```\n   ~~~text\nDo not merge while #703 is active\n   ~~~\n    Do not merge while #703 is active\n- Do not merge while #703 is active\n",
            ),
            work("#703", ""),
        ]);
        let report = extract_snapshot(&input).unwrap();
        assert!(report.coordination_edges.is_empty());
    }

    #[test]
    fn self_and_unresolved_endpoints_are_not_promoted() {
        let input = snapshot(vec![work(
            "#748",
            "Do not merge while #748 is active\nDo not merge while #999 is active\n",
        )]);
        let report = extract_snapshot(&input).unwrap();
        assert!(report.coordination_edges.is_empty());
        assert_eq!(report.stats.self_references_ignored, 1);
        assert_eq!(report.stats.unresolved_endpoints_ignored, 1);
        assert_eq!(report.unresolved_endpoint_receipts.len(), 1);
        let unresolved = &report.unresolved_endpoint_receipts[0];
        assert_eq!(unresolved.from, "#748");
        assert_eq!(unresolved.to, "#999");
        assert_eq!(unresolved.source, "github:pull/748");
        assert_eq!(unresolved.source_head_sha, sha('a'));
        assert_eq!(unresolved.source_updated_at, "2026-08-19T00:00:00Z");
        assert_eq!(
            unresolved.matched_clause,
            "Do not merge while #999 is active"
        );
    }

    #[test]
    fn duplicate_exact_edges_collapse() {
        let input = snapshot(vec![
            work(
                "#748",
                "Do not merge while #703 is active\nDo not merge while #703 is active\n",
            ),
            work("#703", ""),
        ]);
        let report = extract_snapshot(&input).unwrap();
        assert_eq!(report.coordination_edges.len(), 1);
        assert_eq!(report.stats.duplicate_edges_ignored, 1);
    }

    #[test]
    fn source_receipt_keeps_applicability_coordinate_without_strengthening_claim() {
        let input = snapshot(vec![
            work("#748", "Do not merge while #703 is active\n"),
            work("#703", ""),
        ]);
        let report = extract_snapshot(&input).unwrap();
        assert_eq!(
            report.source_receipts[0].source_updated_at,
            "2026-08-19T00:00:00Z"
        );
        assert!(report.unknowns[0].contains("continued applicability"));
    }

    #[test]
    fn rejects_duplicate_work_ids_and_noncanonical_sources() {
        let duplicate = snapshot(vec![work("#748", ""), work("#748", "")]);
        assert!(extract_snapshot(&duplicate).is_err());

        let input = serde_json::to_string(&json!({
            "schema_version": 1,
            "source": "test",
            "work": [{
                "id": "#748",
                "kind": "pull_request",
                "source": "github:pull/999",
                "head_sha": sha('a'),
                "updated_at": "2026-08-19T00:00:00Z",
                "body": ""
            }]
        }))
        .unwrap();
        assert!(extract_snapshot(&input).is_err());
    }

    #[test]
    fn rejects_oversized_snapshot_before_parsing() {
        let input = "x".repeat(MAX_SNAPSHOT_BYTES + 1);
        assert!(extract_snapshot(&input).is_err());
    }
}
