#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/provider_snapshot_applicability.rs"]
mod provider_snapshot_applicability;

use provider_snapshot_applicability::identity::{
    Activity, CoordinationKind, DraftPolicy, OrderDirection, OrderField, ProviderCoordination,
    ProviderKind, ProviderSelection, ProviderSnapshotFacts, ProviderState, ProviderWorkFact,
    TieBreak, TieBreakField, TraversalMode, WorkKind, build_provider_snapshot_identity,
};

const HEAD_604: &str = "63eece80df17a97a8544c4d716feca4fad1970ea";
const HEAD_608: &str = "515c60f694664f3b691bfd7f920e4740d75226d1";

fn baseline_selection() -> ProviderSelection {
    ProviderSelection {
        provider_kind: ProviderKind::Github,
        provider_instance: "github.com".to_string(),
        collection: "teamleaderleo/cultist".to_string(),
        work_kind: WorkKind::PullRequest,
        states: vec![ProviderState::Open],
        draft_policy: DraftPolicy::Include,
        traversal_mode: TraversalMode::Exhaustive,
        page_size: 100,
        max_items: None,
        order_field: OrderField::UpdatedAt,
        order_direction: OrderDirection::Desc,
        tie_break: None,
    }
}

fn bounded_selection(max_items: u32) -> ProviderSelection {
    let mut selection = baseline_selection();
    selection.traversal_mode = TraversalMode::Bounded;
    selection.max_items = Some(max_items);
    selection.tie_break = Some(TieBreak {
        field: TieBreakField::LocalWorkNumber,
        direction: OrderDirection::Asc,
    });
    selection
}

fn baseline_work() -> Vec<ProviderWorkFact> {
    vec![
        ProviderWorkFact {
            id: "pull/604".to_string(),
            head_sha: HEAD_604.to_string(),
            activity: Activity::ConfirmedActive,
            changed_paths: vec![
                "AGENTS.md".to_string(),
                "docs/agent-native-operating-mode.md".to_string(),
            ],
        },
        ProviderWorkFact {
            id: "pull/608".to_string(),
            head_sha: HEAD_608.to_string(),
            activity: Activity::ConfirmedActive,
            changed_paths: vec![
                "src/quarry/research_ir.py".to_string(),
                "tests/test_research_ir.py".to_string(),
            ],
        },
    ]
}

fn edge(source: &str) -> ProviderCoordination {
    ProviderCoordination {
        kind: CoordinationKind::DependsOn,
        from: "pull/604".to_string(),
        to: "pull/608".to_string(),
        source: source.to_string(),
    }
}

fn facts(
    selection: ProviderSelection,
    work: Vec<ProviderWorkFact>,
    coordination_edges: Vec<ProviderCoordination>,
) -> ProviderSnapshotFacts {
    ProviderSnapshotFacts {
        selection,
        work,
        coordination_edges,
    }
}

fn identity(facts: &ProviderSnapshotFacts) -> String {
    build_provider_snapshot_identity(facts)
        .unwrap()
        .as_str()
        .to_string()
}

fn baseline_facts() -> ProviderSnapshotFacts {
    facts(baseline_selection(), baseline_work(), Vec::new())
}

#[test]
fn baseline_identity_matches_reviewed_research_encoding() {
    assert_eq!(
        identity(&baseline_facts()),
        "sha256:c1fcc24846686703386dcf32d257ca6d752df4685ab57d873d86c17f07d9a62d"
    );
}

#[test]
fn provider_scope_case_trailing_dot_and_state_order_are_canonical() {
    let mut first = baseline_facts();
    first.selection.states = vec![ProviderState::Open, ProviderState::Closed];

    let mut second = baseline_facts();
    second.selection.provider_instance = "GITHUB.COM.".to_string();
    second.selection.collection = "TeamLeaderLeo/Cultist".to_string();
    second.selection.states = vec![ProviderState::Closed, ProviderState::Open];

    assert_eq!(identity(&first), identity(&second));
}

#[test]
fn exhaustive_transport_order_and_page_size_do_not_change_identity() {
    let first = baseline_facts();
    let mut second = baseline_facts();
    second.selection.page_size = 50;
    second.selection.order_field = OrderField::CreatedAt;
    second.selection.order_direction = OrderDirection::Asc;

    assert_eq!(identity(&first), identity(&second));
}

#[test]
fn bounded_page_size_is_transport_only_but_limit_and_order_are_identity() {
    let first = facts(bounded_selection(100), baseline_work(), Vec::new());

    let mut other_page = first.clone();
    other_page.selection.page_size = 20;
    assert_eq!(identity(&first), identity(&other_page));

    let different_limit = facts(bounded_selection(50), baseline_work(), Vec::new());
    assert_ne!(identity(&first), identity(&different_limit));

    let mut different_order = first.clone();
    different_order.selection.order_field = OrderField::CreatedAt;
    different_order.selection.order_direction = OrderDirection::Asc;
    assert_ne!(identity(&first), identity(&different_order));

    let mut different_tie_break = first.clone();
    different_tie_break.selection.tie_break = Some(TieBreak {
        field: TieBreakField::LocalWorkNumber,
        direction: OrderDirection::Desc,
    });
    assert_ne!(identity(&first), identity(&different_tie_break));
}

#[test]
fn work_path_and_edge_order_are_identity_invariant() {
    let first_edges = vec![
        edge("provider:pull/604"),
        ProviderCoordination {
            kind: CoordinationKind::Blocks,
            from: "pull/608".to_string(),
            to: "pull/604".to_string(),
            source: "provider:pull/608".to_string(),
        },
    ];
    let first = facts(baseline_selection(), baseline_work(), first_edges.clone());

    let mut second_work = baseline_work();
    second_work.reverse();
    second_work[0].changed_paths.reverse();
    let mut second_edges = first_edges;
    second_edges.reverse();
    let second = facts(baseline_selection(), second_work, second_edges);

    assert_eq!(identity(&first), identity(&second));
}

#[test]
fn equivalent_head_hex_case_preserves_identity() {
    let first = baseline_facts();
    let mut second = baseline_facts();
    second.work[0].head_sha = HEAD_604.to_ascii_uppercase();

    assert_eq!(identity(&first), identity(&second));
}

#[test]
fn membership_head_activity_and_path_movement_change_identity() {
    let baseline = baseline_facts();
    let baseline_identity = identity(&baseline);

    let mut membership = baseline.clone();
    membership.work.push(ProviderWorkFact {
        id: "pull/627".to_string(),
        head_sha: "769ded20439efe0567d4553141598cfd3965a013".to_string(),
        activity: Activity::ConfirmedActive,
        changed_paths: vec!["tests/test_research_610_strict_carrier.py".to_string()],
    });

    let mut head = baseline.clone();
    head.work[0].head_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();

    let mut activity = baseline.clone();
    activity.work[0].activity = Activity::Unresolved;

    let mut path = baseline.clone();
    path.work[0].changed_paths = vec![
        "AGENTS.md".to_string(),
        "src/new_collision_surface.rs".to_string(),
    ];

    for changed in [membership, head, activity, path] {
        assert_ne!(baseline_identity, identity(&changed));
    }
}

#[test]
fn coordination_provenance_is_omitted_while_semantics_change_identity() {
    let first = facts(
        baseline_selection(),
        baseline_work(),
        vec![edge("provider:pull/604")],
    );
    let moved_source = facts(
        baseline_selection(),
        baseline_work(),
        vec![edge("provider:reviewed-metadata")],
    );
    assert_eq!(identity(&first), identity(&moved_source));

    let multiple_sources = facts(
        baseline_selection(),
        baseline_work(),
        vec![
            edge("provider:pull/604"),
            edge("provider:reviewed-metadata"),
        ],
    );
    assert_eq!(identity(&first), identity(&multiple_sources));

    let changed_semantics = facts(
        baseline_selection(),
        baseline_work(),
        vec![ProviderCoordination {
            kind: CoordinationKind::HoldMergeWhile,
            from: "pull/604".to_string(),
            to: "pull/608".to_string(),
            source: "provider:pull/604".to_string(),
        }],
    );
    assert_ne!(identity(&first), identity(&changed_semantics));
}

#[test]
fn malformed_selection_contracts_fail_closed() {
    let mut malformed_collection = baseline_facts();
    malformed_collection.selection.collection = "teamleaderleo/cultist/extra".to_string();
    assert!(build_provider_snapshot_identity(&malformed_collection).is_err());

    let mut malformed_host = baseline_facts();
    malformed_host.selection.provider_instance = "https://github.com".to_string();
    assert!(build_provider_snapshot_identity(&malformed_host).is_err());

    let mut duplicate_states = baseline_facts();
    duplicate_states.selection.states = vec![ProviderState::Open, ProviderState::Open];
    assert!(build_provider_snapshot_identity(&duplicate_states).is_err());

    let mut empty_states = baseline_facts();
    empty_states.selection.states.clear();
    assert!(build_provider_snapshot_identity(&empty_states).is_err());

    let mut zero_page = baseline_facts();
    zero_page.selection.page_size = 0;
    assert!(build_provider_snapshot_identity(&zero_page).is_err());

    let mut exhaustive_with_bound = baseline_facts();
    exhaustive_with_bound.selection.max_items = Some(100);
    assert!(build_provider_snapshot_identity(&exhaustive_with_bound).is_err());

    let mut bounded_without_bound = baseline_facts();
    bounded_without_bound.selection.traversal_mode = TraversalMode::Bounded;
    assert!(build_provider_snapshot_identity(&bounded_without_bound).is_err());

    let mut bounded_without_tie = baseline_facts();
    bounded_without_tie.selection.traversal_mode = TraversalMode::Bounded;
    bounded_without_tie.selection.max_items = Some(100);
    assert!(build_provider_snapshot_identity(&bounded_without_tie).is_err());
}

#[test]
fn malformed_work_facts_and_coordination_fail_closed() {
    for malformed in ["#604", "pull/0604", "pull/0", "PULL/604", "pull/604 "] {
        let mut candidate = baseline_facts();
        candidate.work[0].id = malformed.to_string();
        assert!(
            build_provider_snapshot_identity(&candidate).is_err(),
            "accepted malformed work id `{malformed}`"
        );
    }

    for malformed in [
        "./src/lib.rs",
        "src//lib.rs",
        "src/../lib.rs",
        "src/./lib.rs",
        "src\\lib.rs",
        "/src/lib.rs",
        "src/lib.rs/",
    ] {
        let mut candidate = baseline_facts();
        candidate.work[0].changed_paths = vec![malformed.to_string()];
        assert!(
            build_provider_snapshot_identity(&candidate).is_err(),
            "accepted malformed path `{malformed}`"
        );
    }

    let mut bad_head = baseline_facts();
    bad_head.work[0].head_sha = "abc".to_string();
    assert!(build_provider_snapshot_identity(&bad_head).is_err());

    let mut duplicate_path = baseline_facts();
    duplicate_path.work[0].changed_paths = vec!["src/lib.rs".to_string(), "src/lib.rs".to_string()];
    assert!(build_provider_snapshot_identity(&duplicate_path).is_err());

    let mut duplicate_work = baseline_facts();
    duplicate_work.work.push(ProviderWorkFact {
        id: "pull/604".to_string(),
        head_sha: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        activity: Activity::Preparation,
        changed_paths: Vec::new(),
    });
    assert!(build_provider_snapshot_identity(&duplicate_work).is_err());

    let exact_duplicate = facts(
        baseline_selection(),
        baseline_work(),
        vec![edge("provider:pull/604"), edge("provider:pull/604")],
    );
    assert!(build_provider_snapshot_identity(&exact_duplicate).is_err());

    let missing_endpoint = facts(
        baseline_selection(),
        baseline_work(),
        vec![ProviderCoordination {
            kind: CoordinationKind::Supersedes,
            from: "pull/604".to_string(),
            to: "pull/999".to_string(),
            source: "provider:pull/604".to_string(),
        }],
    );
    assert!(build_provider_snapshot_identity(&missing_endpoint).is_err());

    let self_edge = facts(
        baseline_selection(),
        baseline_work(),
        vec![ProviderCoordination {
            kind: CoordinationKind::Blocks,
            from: "pull/604".to_string(),
            to: "pull/604".to_string(),
            source: "provider:pull/604".to_string(),
        }],
    );
    assert!(build_provider_snapshot_identity(&self_edge).is_err());
}
