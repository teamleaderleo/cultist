use std::error::Error;
use std::io::{self, Read};

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/provider_snapshot_applicability.rs"]
mod provider_snapshot_applicability;

use provider_snapshot_applicability::identity::{
    Activity, CoordinationKind, DraftPolicy, OrderDirection, OrderField, ProviderCoordination,
    ProviderKind, ProviderSelection, ProviderSnapshotFacts, ProviderState, ProviderWorkFact,
    TraversalMode, WorkKind, build_provider_snapshot_identity,
};

const REQUEST_SCHEMA_VERSION: u32 = 1;
const OUTPUT_SCHEMA_VERSION: u32 = 1;
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_SNAPSHOTS: usize = 4;

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireActivity {
    #[default]
    ConfirmedActive,
    Preparation,
    Unresolved,
}

impl From<WireActivity> for Activity {
    fn from(value: WireActivity) -> Self {
        match value {
            WireActivity::ConfirmedActive => Self::ConfirmedActive,
            WireActivity::Preparation => Self::Preparation,
            WireActivity::Unresolved => Self::Unresolved,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireCoordinationKind {
    DependsOn,
    Blocks,
    HoldMergeWhile,
    Supersedes,
}

impl From<WireCoordinationKind> for CoordinationKind {
    fn from(value: WireCoordinationKind) -> Self {
        match value {
            WireCoordinationKind::DependsOn => Self::DependsOn,
            WireCoordinationKind::Blocks => Self::Blocks,
            WireCoordinationKind::HoldMergeWhile => Self::HoldMergeWhile,
            WireCoordinationKind::Supersedes => Self::Supersedes,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireWork {
    number: u64,
    head_sha: String,
    #[serde(default)]
    activity: WireActivity,
    changed_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCoordination {
    kind: WireCoordinationKind,
    from_number: u64,
    to_number: u64,
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSnapshot {
    provider_instance: String,
    collection: String,
    work: Vec<WireWork>,
    #[serde(default)]
    coordination_edges: Vec<WireCoordination>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    schema_version: u32,
    snapshots: Vec<WireSnapshot>,
}

#[derive(Serialize)]
struct WireReceipt {
    provider_snapshot_identity: String,
}

#[derive(Serialize)]
struct WireOutput {
    schema_version: u32,
    snapshots: Vec<WireReceipt>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("github-provider-snapshot: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(format!(
            "GitHub provider snapshot request exceeds the {MAX_REQUEST_BYTES}-byte limit"
        )
        .into());
    }

    let request: WireRequest = serde_json::from_slice(&bytes)?;
    if request.schema_version != REQUEST_SCHEMA_VERSION {
        return Err(format!(
            "unsupported GitHub provider snapshot request schema {}; expected {REQUEST_SCHEMA_VERSION}",
            request.schema_version
        )
        .into());
    }
    if request.snapshots.is_empty() || request.snapshots.len() > MAX_SNAPSHOTS {
        return Err(format!(
            "GitHub provider snapshot request must contain 1..={MAX_SNAPSHOTS} snapshots"
        )
        .into());
    }

    let snapshots = request
        .snapshots
        .into_iter()
        .map(fingerprint_snapshot)
        .collect::<Result<Vec<_>, _>>()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&WireOutput {
            schema_version: OUTPUT_SCHEMA_VERSION,
            snapshots,
        })?
    );
    Ok(())
}

fn fingerprint_snapshot(snapshot: WireSnapshot) -> Result<WireReceipt, Box<dyn Error>> {
    let work = snapshot
        .work
        .into_iter()
        .map(|work| ProviderWorkFact {
            id: format!("pull/{}", work.number),
            head_sha: work.head_sha,
            activity: work.activity.into(),
            changed_paths: work.changed_paths,
        })
        .collect();
    let coordination_edges = snapshot
        .coordination_edges
        .into_iter()
        .map(|edge| ProviderCoordination {
            kind: edge.kind.into(),
            from: format!("pull/{}", edge.from_number),
            to: format!("pull/{}", edge.to_number),
            source: edge.source,
        })
        .collect();

    let identity = build_provider_snapshot_identity(&ProviderSnapshotFacts {
        selection: ProviderSelection {
            provider_kind: ProviderKind::Github,
            provider_instance: snapshot.provider_instance,
            collection: snapshot.collection,
            work_kind: WorkKind::PullRequest,
            states: vec![ProviderState::Open],
            draft_policy: DraftPolicy::Include,
            traversal_mode: TraversalMode::Exhaustive,
            page_size: 100,
            max_items: None,
            order_field: OrderField::UpdatedAt,
            order_direction: OrderDirection::Desc,
            tie_break: None,
        },
        work,
        coordination_edges,
    })?;

    Ok(WireReceipt {
        provider_snapshot_identity: identity.as_str().to_string(),
    })
}
