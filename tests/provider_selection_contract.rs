use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};

const SELECTION_SCHEMA_VERSION: u32 = 0;

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

#[derive(Clone, Debug)]
struct SelectionInput<'a> {
    provider_kind: &'a str,
    provider_instance: &'a str,
    collection: &'a str,
    work_kind: &'a str,
    states: Vec<&'a str>,
    draft_policy: DraftPolicyInput,
    traversal_mode: TraversalModeInput,
    page_size: u32,
    order_field: OrderFieldInput,
    order_direction: OrderDirectionInput,
}

#[derive(Debug, Serialize)]
struct SelectionDocument {
    schema_version: u32,
    provider_kind: String,
    provider_instance: String,
    collection: String,
    work_kind: String,
    states: Vec<String>,
    draft_policy: String,
    coverage: CoverageIdentity,
}

#[derive(Debug, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum CoverageIdentity {
    Exhaustive,
    Bounded {
        limit: u32,
        order_field: String,
        order_direction: String,
    },
}

#[derive(Serialize)]
struct OpaqueQueryDocument {
    provider_kind: String,
    provider_instance: String,
    collection: String,
    query: String,
}

fn canonical_symbol(value: &str, label: &str) -> Result<String, String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!(
            "{label} must be a non-empty canonical ASCII symbol"
        ));
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_host(value: &str) -> Result<String, String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
    {
        return Err("provider instance must be a canonical host name".to_string());
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
    let owner = canonical_symbol(owner, "collection owner")?;
    let repository = canonical_symbol(repository, "collection repository")?;
    Ok(format!("{owner}/{repository}"))
}

fn selection_document(input: &SelectionInput<'_>) -> Result<SelectionDocument, String> {
    if input.page_size == 0 {
        return Err("page size must be positive".to_string());
    }

    let provider_kind = canonical_symbol(input.provider_kind, "provider kind")?;
    let provider_instance = canonical_host(input.provider_instance)?;
    let collection = canonical_collection(input.collection)?;
    let work_kind = canonical_symbol(input.work_kind, "work kind")?;

    let mut states = BTreeSet::new();
    for raw_state in &input.states {
        let state = canonical_symbol(raw_state, "provider state")?;
        if !states.insert(state.clone()) {
            return Err(format!("duplicate provider state `{state}`"));
        }
    }
    if states.is_empty() {
        return Err("selection contract must contain at least one provider state".to_string());
    }

    let coverage = match input.traversal_mode {
        TraversalModeInput::Exhaustive => CoverageIdentity::Exhaustive,
        TraversalModeInput::Bounded => CoverageIdentity::Bounded {
            limit: input.page_size,
            order_field: input.order_field.as_str().to_string(),
            order_direction: input.order_direction.as_str().to_string(),
        },
    };

    Ok(SelectionDocument {
        schema_version: SELECTION_SCHEMA_VERSION,
        provider_kind,
        provider_instance,
        collection,
        work_kind,
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
        provider_kind: canonical_symbol(input.provider_kind, "provider kind")?,
        provider_instance: canonical_host(input.provider_instance)?,
        collection: canonical_collection(input.collection)?,
        query: canonical_query_label(query)?,
    })
}

fn baseline() -> SelectionInput<'static> {
    SelectionInput {
        provider_kind: "github",
        provider_instance: "github.com",
        collection: "teamleaderleo/cultist",
        work_kind: "pull_request",
        states: vec!["open"],
        draft_policy: DraftPolicyInput::Include,
        traversal_mode: TraversalModeInput::Exhaustive,
        page_size: 100,
        order_field: OrderFieldInput::UpdatedAt,
        order_direction: OrderDirectionInput::Desc,
    }
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
fn provider_scope_case_and_unordered_state_order_are_canonical() {
    let mut first = baseline();
    first.states = vec!["open", "closed"];

    let mut second = baseline();
    second.provider_kind = "GitHub";
    second.provider_instance = "GITHUB.COM";
    second.collection = "TeamLeaderLeo/Cultist";
    second.states = vec!["CLOSED", "OPEN"];

    assert_eq!(
        selection_identity(&first).unwrap(),
        selection_identity(&second).unwrap()
    );
}

#[test]
fn materially_different_selected_states_change_identity() {
    let open_only = baseline();
    let mut open_and_closed = baseline();
    open_and_closed.states = vec!["open", "closed"];

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
fn bounded_population_size_and_order_are_selection_relevant() {
    let mut first = baseline();
    first.traversal_mode = TraversalModeInput::Bounded;
    first.page_size = 50;

    let mut different_limit = first.clone();
    different_limit.page_size = 100;
    let mut different_order = first.clone();
    different_order.order_field = OrderFieldInput::CreatedAt;
    different_order.order_direction = OrderDirectionInput::Asc;

    assert_ne!(
        selection_identity(&first).unwrap(),
        selection_identity(&different_limit).unwrap()
    );
    assert_ne!(
        selection_identity(&first).unwrap(),
        selection_identity(&different_order).unwrap()
    );
}

#[test]
fn exhaustive_and_bounded_population_contracts_differ() {
    let exhaustive = baseline();
    let mut bounded = baseline();
    bounded.traversal_mode = TraversalModeInput::Bounded;

    assert_ne!(
        selection_identity(&exhaustive).unwrap(),
        selection_identity(&bounded).unwrap()
    );
}

#[test]
fn malformed_scope_and_duplicate_states_fail_closed() {
    let mut malformed_collection = baseline();
    malformed_collection.collection = "teamleaderleo/cultist/extra";
    assert!(selection_identity(&malformed_collection).is_err());

    let mut malformed_host = baseline();
    malformed_host.provider_instance = "https://github.com";
    assert!(selection_identity(&malformed_host).is_err());

    let mut duplicate_states = baseline();
    duplicate_states.states = vec!["open", "OPEN"];
    let error = selection_identity(&duplicate_states).unwrap_err();
    assert!(error.contains("duplicate provider state"));
}
