#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;

use std::collections::BTreeSet;

use applicability::ApplicabilityStatus;

const REPOSITORY_MAIN_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Debug)]
struct ProviderEnumeration {
    membership_read_completed_at: u64,
    observed_at: u64,
    work: BTreeSet<&'static str>,
}

fn work_set(ids: &[&'static str]) -> BTreeSet<&'static str> {
    ids.iter().copied().collect()
}

fn evaluate_population(
    required: &BTreeSet<&'static str>,
    current: Option<&BTreeSet<&'static str>>,
) -> ApplicabilityStatus {
    match current {
        Some(actual) if actual == required => ApplicabilityStatus::Applies,
        Some(_) => ApplicabilityStatus::Invalid,
        None => ApplicabilityStatus::Unknown,
    }
}

#[test]
fn completion_timestamp_can_postdate_provider_work_missing_from_enumeration() {
    let required_main = REPOSITORY_MAIN_A;
    let current_main = REPOSITORY_MAIN_A;
    assert_eq!(required_main, current_main);

    let enumeration = ProviderEnumeration {
        membership_read_completed_at: 100,
        observed_at: 120,
        work: work_set(&["pull/604", "pull/608"]),
    };
    let newly_opened_at = 110;
    let actual_provider_work = work_set(&["pull/604", "pull/608", "pull/627"]);

    assert!(enumeration.membership_read_completed_at < newly_opened_at);
    assert!(newly_opened_at < enumeration.observed_at);
    assert!(!enumeration.work.contains("pull/627"));
    assert!(actual_provider_work.contains("pull/627"));

    assert_eq!(
        evaluate_population(&enumeration.work, Some(&actual_provider_work)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn query_start_timestamp_can_precede_work_that_the_enumeration_includes() {
    let query_started_at = 100;
    let newly_opened_at = 110;
    let membership_read_completed_at = 120;

    let work_at_query_start = work_set(&["pull/604", "pull/608"]);
    let returned_work = work_set(&["pull/604", "pull/608", "pull/627"]);

    assert!(query_started_at < newly_opened_at);
    assert!(newly_opened_at < membership_read_completed_at);
    assert!(!work_at_query_start.contains("pull/627"));
    assert!(returned_work.contains("pull/627"));

    assert_eq!(
        evaluate_population(&work_at_query_start, Some(&returned_work)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn exact_requery_detects_population_movement_without_repository_revision_movement() {
    let required_main = REPOSITORY_MAIN_A;
    let current_main = REPOSITORY_MAIN_A;
    assert_eq!(required_main, current_main);

    let required_provider_work = work_set(&["pull/604", "pull/608"]);
    let current_provider_work = work_set(&["pull/604", "pull/608", "pull/627"]);

    assert_eq!(
        evaluate_population(&required_provider_work, Some(&current_provider_work)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn unavailable_exact_requery_stays_unknown_regardless_of_timestamp_metadata() {
    let required_provider_work = work_set(&["pull/604", "pull/608"]);
    let recorded_observed_at = 120;
    let current_wall_clock = 120;
    assert_eq!(recorded_observed_at, current_wall_clock);

    assert_eq!(
        evaluate_population(&required_provider_work, None),
        ApplicabilityStatus::Unknown
    );
}
