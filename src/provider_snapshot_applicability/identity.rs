use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::ProviderSnapshotIdentity;

const SELECTION_SCHEMA_VERSION: u32 = 0;
const WORK_FACT_SCHEMA_VERSION: u32 = 0;
const SNAPSHOT_COMPOSITION_SCHEMA_VERSION: u32 = 0;
const MAX_PATH_BYTES: usize = 4096;
const MAX_SOURCE_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Github,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    PullRequest,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderState {
    Open,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftPolicy {
    Include,
    Exclude,
    Only,
}

impl DraftPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Exclude => "exclude",
            Self::Only => "only",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalMode {
    Exhaustive,
    Bounded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderField {
    UpdatedAt,
    CreatedAt,
}

impl OrderField {
    fn as_str(self) -> &'static str {
        match self {
            Self::UpdatedAt => "updated_at",
            Self::CreatedAt => "created_at",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl OrderDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TieBreakField {
    LocalWorkNumber,
}

impl TieBreakField {
    fn as_str(self) -> &'static str {
        match self {
            Self::LocalWorkNumber => "local_work_number",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TieBreak {
    pub field: TieBreakField,
    pub direction: OrderDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSelection {
    pub provider_kind: ProviderKind,
    pub provider_instance: String,
    pub collection: String,
    pub work_kind: WorkKind,
    pub states: Vec<ProviderState>,
    pub draft_policy: DraftPolicy,
    pub traversal_mode: TraversalMode,
    pub page_size: u32,
    pub max_items: Option<u32>,
    pub order_field: OrderField,
    pub order_direction: OrderDirection,
    pub tie_break: Option<TieBreak>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    ConfirmedActive,
    Preparation,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationKind {
    DependsOn,
    Blocks,
    HoldMergeWhile,
    Supersedes,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderWorkFact {
    pub id: String,
    pub head_sha: String,
    pub activity: Activity,
    pub changed_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCoordination {
    pub kind: CoordinationKind,
    pub from: String,
    pub to: String,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSnapshotFacts {
    pub selection: ProviderSelection,
    pub work: Vec<ProviderWorkFact>,
    pub coordination_edges: Vec<ProviderCoordination>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProviderSnapshotIdentityBuildError {
    message: String,
}

impl ProviderSnapshotIdentityBuildError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ProviderSnapshotIdentityBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ProviderSnapshotIdentityBuildError {}

#[derive(Debug, Serialize)]
struct SelectionDocument {
    schema_version: u32,
    provider_kind: ProviderKind,
    provider_instance: String,
    collection: String,
    work_kind: WorkKind,
    states: Vec<ProviderState>,
    draft_policy: String,
    coverage: CoverageIdentity,
}

#[derive(Debug, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum CoverageIdentity {
    Exhaustive,
    Bounded {
        max_items: u32,
        order_field: String,
        order_direction: String,
        tie_break_field: String,
        tie_break_direction: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct CanonicalWorkFact {
    id: String,
    head_sha: String,
    activity: Activity,
    changed_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct SemanticCoordinationEdge {
    kind: CoordinationKind,
    from: String,
    to: String,
}

#[derive(Serialize)]
struct WorkFactDocument {
    schema_version: u32,
    work: Vec<CanonicalWorkFact>,
    coordination_edges: Vec<SemanticCoordinationEdge>,
}

#[derive(Serialize)]
struct ProviderSnapshotDocument<'a> {
    schema_version: u32,
    selection_identity: &'a str,
    work_fact_identity: &'a str,
}

pub fn build_provider_snapshot_identity(
    facts: &ProviderSnapshotFacts,
) -> Result<ProviderSnapshotIdentity, ProviderSnapshotIdentityBuildError> {
    let selection_identity = selection_identity(&facts.selection)?;
    let work_fact_identity = work_fact_identity(&facts.work, &facts.coordination_edges)?;
    let digest = digest_hex(&ProviderSnapshotDocument {
        schema_version: SNAPSHOT_COMPOSITION_SCHEMA_VERSION,
        selection_identity: &selection_identity,
        work_fact_identity: &work_fact_identity,
    })?;
    ProviderSnapshotIdentity::parse(format!("sha256:{digest}"))
        .map_err(|error| ProviderSnapshotIdentityBuildError::new(error.to_string()))
}

fn selection_identity(
    input: &ProviderSelection,
) -> Result<String, ProviderSnapshotIdentityBuildError> {
    digest_hex(&selection_document(input)?)
}

fn selection_document(
    input: &ProviderSelection,
) -> Result<SelectionDocument, ProviderSnapshotIdentityBuildError> {
    if input.page_size == 0 {
        return Err(ProviderSnapshotIdentityBuildError::new(
            "page size must be positive",
        ));
    }

    let provider_instance = canonical_host(&input.provider_instance)?;
    let collection = canonical_collection(&input.collection)?;

    let mut states = BTreeSet::new();
    for state in &input.states {
        if !states.insert(*state) {
            return Err(ProviderSnapshotIdentityBuildError::new(format!(
                "duplicate provider state `{state:?}`"
            )));
        }
    }
    if states.is_empty() {
        return Err(ProviderSnapshotIdentityBuildError::new(
            "selection contract must contain at least one provider state",
        ));
    }

    let coverage = match (input.traversal_mode, input.max_items, input.tie_break) {
        (TraversalMode::Exhaustive, None, None) => CoverageIdentity::Exhaustive,
        (TraversalMode::Exhaustive, Some(_), _) => {
            return Err(ProviderSnapshotIdentityBuildError::new(
                "exhaustive traversal must not declare max_items",
            ));
        }
        (TraversalMode::Exhaustive, None, Some(_)) => {
            return Err(ProviderSnapshotIdentityBuildError::new(
                "exhaustive traversal must not declare a tie-break",
            ));
        }
        (TraversalMode::Bounded, None, _) => {
            return Err(ProviderSnapshotIdentityBuildError::new(
                "bounded traversal requires max_items",
            ));
        }
        (TraversalMode::Bounded, Some(0), _) => {
            return Err(ProviderSnapshotIdentityBuildError::new(
                "bounded max_items must be positive",
            ));
        }
        (TraversalMode::Bounded, Some(_), None) => {
            return Err(ProviderSnapshotIdentityBuildError::new(
                "bounded traversal requires a deterministic tie-break",
            ));
        }
        (TraversalMode::Bounded, Some(max_items), Some(tie_break)) => CoverageIdentity::Bounded {
            max_items,
            order_field: input.order_field.as_str().to_string(),
            order_direction: input.order_direction.as_str().to_string(),
            tie_break_field: tie_break.field.as_str().to_string(),
            tie_break_direction: tie_break.direction.as_str().to_string(),
        },
    };

    Ok(SelectionDocument {
        schema_version: SELECTION_SCHEMA_VERSION,
        provider_kind: input.provider_kind,
        provider_instance,
        collection,
        work_kind: input.work_kind,
        states: states.into_iter().collect(),
        draft_policy: input.draft_policy.as_str().to_string(),
        coverage,
    })
}

fn work_fact_identity(
    work: &[ProviderWorkFact],
    coordination: &[ProviderCoordination],
) -> Result<String, ProviderSnapshotIdentityBuildError> {
    let mut work_by_id = BTreeMap::new();
    for input in work {
        let item = canonical_work(input)?;
        if work_by_id.insert(item.id.clone(), item).is_some() {
            return Err(ProviderSnapshotIdentityBuildError::new(format!(
                "duplicate work id `{}`",
                input.id
            )));
        }
    }

    let mut exact_coordination_inputs = BTreeSet::new();
    let mut semantic_edges = BTreeSet::new();
    for input in coordination {
        let edge = semantic_edge(input)?;
        if !work_by_id.contains_key(&edge.from) {
            return Err(ProviderSnapshotIdentityBuildError::new(format!(
                "coordination edge references missing work `{}`",
                edge.from
            )));
        }
        if !work_by_id.contains_key(&edge.to) {
            return Err(ProviderSnapshotIdentityBuildError::new(format!(
                "coordination edge references missing work `{}`",
                edge.to
            )));
        }

        let exact = (
            input.kind,
            edge.from.clone(),
            edge.to.clone(),
            input.source.clone(),
        );
        if !exact_coordination_inputs.insert(exact) {
            return Err(ProviderSnapshotIdentityBuildError::new(format!(
                "duplicate coordination evidence for `{}` -> `{}`",
                edge.from, edge.to
            )));
        }
        semantic_edges.insert(edge);
    }

    digest_hex(&WorkFactDocument {
        schema_version: WORK_FACT_SCHEMA_VERSION,
        work: work_by_id.into_values().collect(),
        coordination_edges: semantic_edges.into_iter().collect(),
    })
}

fn canonical_collection_component(
    value: &str,
    label: &str,
) -> Result<String, ProviderSnapshotIdentityBuildError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ProviderSnapshotIdentityBuildError::new(format!(
            "{label} must be a non-empty canonical ASCII repository component"
        )));
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_host(value: &str) -> Result<String, ProviderSnapshotIdentityBuildError> {
    let value = value.strip_suffix('.').unwrap_or(value);
    if value.is_empty() {
        return Err(ProviderSnapshotIdentityBuildError::new(
            "provider instance must be a canonical host name",
        ));
    }
    for label in value.split('.') {
        if label.is_empty()
            || label.len() > 63
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || !label
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            || !label
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
        {
            return Err(ProviderSnapshotIdentityBuildError::new(
                "provider instance must be a canonical host name",
            ));
        }
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_collection(value: &str) -> Result<String, ProviderSnapshotIdentityBuildError> {
    let (owner, repository) = value.split_once('/').ok_or_else(|| {
        ProviderSnapshotIdentityBuildError::new("collection must be `owner/repository`")
    })?;
    if repository.contains('/') {
        return Err(ProviderSnapshotIdentityBuildError::new(
            "collection must contain exactly one `/` separator",
        ));
    }
    let owner = canonical_collection_component(owner, "collection owner")?;
    let repository = canonical_collection_component(repository, "collection repository")?;
    Ok(format!("{owner}/{repository}"))
}

fn canonical_work_id(raw: &str) -> Result<String, ProviderSnapshotIdentityBuildError> {
    let digits = raw.strip_prefix("pull/").ok_or_else(|| {
        ProviderSnapshotIdentityBuildError::new(format!(
            "work id `{raw}` must use canonical `pull/<number>` form"
        ))
    })?;
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ProviderSnapshotIdentityBuildError::new(format!(
            "work id `{raw}` must use a positive canonical decimal number"
        )));
    }
    Ok(format!("pull/{digits}"))
}

fn canonical_head_sha(raw: &str) -> Result<String, ProviderSnapshotIdentityBuildError> {
    if raw.len() != 40 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProviderSnapshotIdentityBuildError::new(
            "head sha must contain exactly 40 hexadecimal characters",
        ));
    }
    Ok(raw.to_ascii_lowercase())
}

fn canonical_path(raw: &str) -> Result<String, ProviderSnapshotIdentityBuildError> {
    if raw.is_empty() || raw.len() > MAX_PATH_BYTES {
        return Err(ProviderSnapshotIdentityBuildError::new(format!(
            "changed path must contain 1..={MAX_PATH_BYTES} bytes"
        )));
    }
    if raw.contains('\\') {
        return Err(ProviderSnapshotIdentityBuildError::new(format!(
            "changed path `{raw}` must use `/` separators"
        )));
    }
    if raw.chars().any(char::is_control) {
        return Err(ProviderSnapshotIdentityBuildError::new(format!(
            "changed path `{raw}` contains a control character"
        )));
    }

    let mut parts = 0usize;
    for part in raw.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(ProviderSnapshotIdentityBuildError::new(format!(
                "changed path `{raw}` must be a canonical relative path without traversal"
            )));
        }
        parts += 1;
    }
    if parts == 0 {
        return Err(ProviderSnapshotIdentityBuildError::new(format!(
            "changed path `{raw}` is not canonical"
        )));
    }
    Ok(raw.to_string())
}

fn validate_source(source: &str) -> Result<(), ProviderSnapshotIdentityBuildError> {
    if source.is_empty() || source.len() > MAX_SOURCE_BYTES {
        return Err(ProviderSnapshotIdentityBuildError::new(format!(
            "coordination source must contain 1..={MAX_SOURCE_BYTES} bytes"
        )));
    }
    if source.chars().any(char::is_control) {
        return Err(ProviderSnapshotIdentityBuildError::new(
            "coordination source contains a control character",
        ));
    }
    Ok(())
}

fn canonical_work(
    input: &ProviderWorkFact,
) -> Result<CanonicalWorkFact, ProviderSnapshotIdentityBuildError> {
    let id = canonical_work_id(&input.id)?;
    let head_sha = canonical_head_sha(&input.head_sha)?;
    let mut paths = BTreeSet::new();
    for raw_path in &input.changed_paths {
        let path = canonical_path(raw_path)?;
        if !paths.insert(path.clone()) {
            return Err(ProviderSnapshotIdentityBuildError::new(format!(
                "work `{id}` contains duplicate path `{path}`"
            )));
        }
    }

    Ok(CanonicalWorkFact {
        id,
        head_sha,
        activity: input.activity,
        changed_paths: paths.into_iter().collect(),
    })
}

fn semantic_edge(
    input: &ProviderCoordination,
) -> Result<SemanticCoordinationEdge, ProviderSnapshotIdentityBuildError> {
    validate_source(&input.source)?;
    let from = canonical_work_id(&input.from)?;
    let to = canonical_work_id(&input.to)?;
    if from == to {
        return Err(ProviderSnapshotIdentityBuildError::new(
            "coordination edge endpoints must be distinct",
        ));
    }
    Ok(SemanticCoordinationEdge {
        kind: input.kind,
        from,
        to,
    })
}

fn digest_hex<T: Serialize>(value: &T) -> Result<String, ProviderSnapshotIdentityBuildError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ProviderSnapshotIdentityBuildError::new(error.to_string()))?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(encoded)
}
