use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};

const SELECTION_SCHEMA_VERSION: u32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProviderKindInput {
    Github,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum WorkKindInput {
    PullRequest,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProviderStateInput {
    Open,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DraftPolicyInput {
    Include,
    Exclude,
    Only,
}

impl DraftPolicyInput {
    fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Exclude => "exclude",
            Self::Only => "only",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraversalModeInput {
    Exhaustive,
    Bounded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrderFieldInput {
    UpdatedAt,
    CreatedAt,
}

impl OrderFieldInput {
    fn as_str(self) -> &'static str {
        match self {
            Self::UpdatedAt => "updated_at",
            Self::CreatedAt => "created_at",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrderDirectionInput {
    Asc,
    Desc,
}

impl OrderDirectionInput {
    fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TieBreakFieldInput {
    LocalWorkNumber,
}

impl TieBreakFieldInput {
    fn as_str(self) -> &'static str {
        match self {
            Self::LocalWorkNumber => "local_work_number",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TieBreakInput {
    field: TieBreakFieldInput,
    direction: OrderDirectionInput,
}

#[derive(Clone, Debug)]
struct SelectionInput<'a> {
    provider_kind: ProviderKindInput,
    provider_instance: &'a str,
    collection: &'a str,
    work_kind: WorkKindInput,
    states: Vec<ProviderStateInput>,
    draft_policy: DraftPolicyInput,
    traversal_mode: TraversalModeInput,
    page_size: u32,
    max_items: Option<u32>,
    order_field: OrderFieldInput,
    order_direction: OrderDirectionInput,
    tie_break: Option<TieBreakInput>,
}

#[derive(Debug, Serialize)]
struct SelectionDocument {
    schema_version: u32,
    provider_kind: ProviderKindInput,
    provider_instance: String,
    collection: String,
    work_kind: WorkKindInput,
    states: Vec<ProviderStateInput>,
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

#[derive(Serialize)]
struct OpaqueQueryDocument {
    provider_kind: ProviderKindInput,
    provider_instance: String,
    collection: String,
    query: String,
}

fn canonical_collection_component(value: &str, label: &str) -> Result<String, String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!(
            "{label} must be a non-empty canonical ASCII repository component"
        ));
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_host(value: &str) -> Result<String, String> {
    let value = value.strip_suffix('.').unwrap_or(value);
    if value.is_empty() {
        return Err("provider instance must be a canonical host name".to_string());
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
            return Err("provider instance must be a canonical host name".to_string());
        }
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_query_label(value: &str) -> Result<String, String> {
    if value.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'+')
        })
    {
        return Err("query identity must be a non-empty printable query token".to_string());
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_collection(value: &str) -> Result<String, String> {
    let (owner, repository) = value
        .split_once('/')
        .ok_or_else(|| "collection must be `owner/repository`".to_string())?;
    if repository.contains('/') {
        return Err("collection must contain exactly one `/` separator".to_string());
    }
    let owner = canonical_collection_component(owner, "collection owner")?;
    let repository = canonical_collection_component(repository, "collection repository")?;
    Ok(format!("{owner}/{repository}"))
}

fn selection_document(input: &SelectionInput<'_>) -> Result<SelectionDocument, String> {
    if input.page_size == 0 {
        return Err("page size must be positive".to_string());
    }

    let provider_instance = canonical_host(input.provider_instance)?;
    let collection = canonical_collection(input.collection)?;

    let mut states = BTreeSet::new();
    for state in &input.states {
        if !states.insert(*state) {
            return Err(format!("duplicate provider state `{state:?}`"));
        }
    }
    if states.is_empty() {
        return Err("selection contract must contain at least one provider state".to_string());
    }

    let coverage = match (input.traversal_mode, input.max_items, input.tie_break) {
        (TraversalModeInput::Exhaustive, None, None) => CoverageIdentity::Exhaustive,
        (TraversalModeInput::Exhaustive, Some(_), _) => {
            return Err("exhaustive traversal must not declare max_items".to_string());
        }
        (TraversalModeInput::Exhaustive, None, Some(_)) => {
            return Err("exhaustive traversal must not declare a tie-break".to_string());
        }
        (TraversalModeInput::Bounded, None, _) => {
            return Err("bounded traversal requires max_items".to_string());
        }
        (TraversalModeInput::Bounded, Some(0), _) => {
            return Err("bounded max_items must be positive".to_string());
        }
        (TraversalModeInput::Bounded, Some(_), None) => {
            return Err("bounded traversal requires a deterministic tie-break".to_string());
        }
        (TraversalModeInput::Bounded, Some(max_items), Some(tie_break)) => {
            CoverageIdentity::Bounded {
                max_items,
                order_field: input.order_field.as_str().to_string(),
                order_direction: input.order_direction.as_str().to_string(),
                tie_break_field: tie_break.field.as_str().to_string(),
                tie_break_direction: tie_break.direction.as_str().to_string(),
            }
        }
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

fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(encoded)
}

fn selection_identity(input: &SelectionInput<'_>) -> Result<String, String> {
    digest(&selection_document(input)?)
}

fn opaque_query_identity(input: &SelectionInput<'_>, query: &str) -> Result<String, String> {
    digest(&OpaqueQueryDocument {
        provider_kind: input.provider_kind,
        provider_instance: canonical_host(input.provider_instance)?,
        collection: canonical_collection(input.collection)?,
        query: canonical_query_label(query)?,
    })
}

fn baseline() -> SelectionInput<'static> {
    SelectionInput {
        provider_kind: ProviderKindInput::Github,
        provider_instance: "github.com",
        collection: "teamleaderleo/cultist",
        work_kind: WorkKindInput::PullRequest,
        states: vec![ProviderStateInput::Open],
        draft_policy: DraftPolicyInput::Include,
        traversal_mode: TraversalModeInput::Exhaustive,
        page_size: 100,
        max_items: None,
        order_field: OrderFieldInput::UpdatedAt,
        order_direction: OrderDirectionInput::Desc,
        tie_break: None,
    }
}

fn bounded(max_items: u32) -> SelectionInput<'static> {
    let mut input = baseline();
    input.traversal_mode = TraversalModeInput::Bounded;
    input.max_items = Some(max_items);
    input.tie_break = Some(TieBreakInput {
        field: TieBreakFieldInput::LocalWorkNumber,
        direction: OrderDirectionInput::Asc,
    });
    input
}

#[test]
fn opaque_query_label_can_collide_across_materially_different_admission_rules() {
    let include_drafts = baseline();
    let mut exclude_drafts = baseline();
    exclude_drafts.draft_policy = DraftPolicyInput::Exclude;

    assert_eq!(
        opaque_query_identity(
            &include_drafts,
            "open-pull-requests+explicit-preparation:v1"
        )
        .unwrap(),
        opaque_query_identity(
            &exclude_drafts,
            "open-pull-requests+explicit-preparation:v1"
        )
        .unwrap()
    );
    assert_ne!(
        selection_identity(&include_drafts).unwrap(),
        selection_identity(&exclude_drafts).unwrap()
    );
}

#[test]
fn provider_collection_and_instance_are_part_of_population_identity() {
    let baseline = baseline();
    let mut other_repository = baseline.clone();
    other_repository.collection = "teamleaderleo/other";
    let mut other_instance = baseline.clone();
    other_instance.provider_instance = "github.example.com";

    assert_ne!(
        selection_identity(&baseline).unwrap(),
        selection_identity(&other_repository).unwrap()
    );
    assert_ne!(
        selection_identity(&baseline).unwrap(),
        selection_identity(&other_instance).unwrap()
    );
}

#[test]
fn provider_scope_case_trailing_dot_and_unordered_states_are_canonical() {
    let mut first = baseline();
    first.states = vec![ProviderStateInput::Open, ProviderStateInput::Closed];

    let mut second = baseline();
    second.provider_instance = "GITHUB.COM.";
    second.collection = "TeamLeaderLeo/Cultist";
    second.states = vec![ProviderStateInput::Closed, ProviderStateInput::Open];

    assert_eq!(
        selection_identity(&first).unwrap(),
        selection_identity(&second).unwrap()
    );
}

#[test]
fn materially_different_selected_states_change_identity() {
    let open_only = baseline();
    let mut open_and_closed = baseline();
    open_and_closed.states = vec![ProviderStateInput::Open, ProviderStateInput::Closed];

    assert_ne!(
        selection_identity(&open_only).unwrap(),
        selection_identity(&open_and_closed).unwrap()
    );
}

#[test]
fn draft_admission_policy_changes_identity() {
    let include = baseline();
    let mut only = baseline();
    only.draft_policy = DraftPolicyInput::Only;

    assert_ne!(
        selection_identity(&include).unwrap(),
        selection_identity(&only).unwrap()
    );
}

#[test]
fn exhaustive_transport_order_and_page_size_do_not_change_selection_identity() {
    let first = baseline();
    let mut second = baseline();
    second.page_size = 50;
    second.order_field = OrderFieldInput::CreatedAt;
    second.order_direction = OrderDirectionInput::Asc;

    assert_eq!(
        selection_identity(&first).unwrap(),
        selection_identity(&second).unwrap()
    );
}

#[test]
fn bounded_transport_page_size_does_not_change_selection_identity() {
    let mut first = bounded(100);
    first.page_size = 20;
    let mut second = bounded(100);
    second.page_size = 100;

    assert_eq!(
        selection_identity(&first).unwrap(),
        selection_identity(&second).unwrap()
    );
}

#[test]
fn bounded_population_limit_primary_order_and_tie_break_are_selection_relevant() {
    let first = bounded(100);
    let different_limit = bounded(50);
    let mut different_primary_order = first.clone();
    different_primary_order.order_field = OrderFieldInput::CreatedAt;
    different_primary_order.order_direction = OrderDirectionInput::Asc;
    let mut different_tie_break = first.clone();
    different_tie_break.tie_break = Some(TieBreakInput {
        field: TieBreakFieldInput::LocalWorkNumber,
        direction: OrderDirectionInput::Desc,
    });

    assert_ne!(
        selection_identity(&first).unwrap(),
        selection_identity(&different_limit).unwrap()
    );
    assert_ne!(
        selection_identity(&first).unwrap(),
        selection_identity(&different_primary_order).unwrap()
    );
    assert_ne!(
        selection_identity(&first).unwrap(),
        selection_identity(&different_tie_break).unwrap()
    );
}

#[test]
fn exhaustive_and_bounded_population_contracts_differ() {
    let exhaustive = baseline();
    let bounded = bounded(100);

    assert_ne!(
        selection_identity(&exhaustive).unwrap(),
        selection_identity(&bounded).unwrap()
    );
}

#[test]
fn malformed_scope_coverage_and_duplicate_states_fail_closed() {
    let mut malformed_collection = baseline();
    malformed_collection.collection = "teamleaderleo/cultist/extra";
    assert!(selection_identity(&malformed_collection).is_err());

    let mut malformed_host = baseline();
    malformed_host.provider_instance = "https://github.com";
    assert!(selection_identity(&malformed_host).is_err());

    let mut duplicate_states = baseline();
    duplicate_states.states = vec![ProviderStateInput::Open, ProviderStateInput::Open];
    let error = selection_identity(&duplicate_states).unwrap_err();
    assert!(error.contains("duplicate provider state"));

    let mut exhaustive_with_bound = baseline();
    exhaustive_with_bound.max_items = Some(100);
    assert!(selection_identity(&exhaustive_with_bound).is_err());

    let mut exhaustive_with_tie_break = baseline();
    exhaustive_with_tie_break.tie_break = Some(TieBreakInput {
        field: TieBreakFieldInput::LocalWorkNumber,
        direction: OrderDirectionInput::Asc,
    });
    assert!(selection_identity(&exhaustive_with_tie_break).is_err());

    let mut bounded_without_bound = baseline();
    bounded_without_bound.traversal_mode = TraversalModeInput::Bounded;
    assert!(selection_identity(&bounded_without_bound).is_err());

    let mut bounded_without_tie_break = bounded(100);
    bounded_without_tie_break.tie_break = None;
    assert!(selection_identity(&bounded_without_tie_break).is_err());

    let mut zero_bound = bounded(100);
    zero_bound.max_items = Some(0);
    assert!(selection_identity(&zero_bound).is_err());
}
