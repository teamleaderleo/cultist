#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;

use applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ApplicabilityMutation {
    MoveRequiredRevision,
    DropRequiredRevisionContext,
    MoveUnrequiredRepository,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MutationVerdict {
    Survived,
    Killed,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ApplicabilityMutationReceipt {
    mutation: ApplicabilityMutation,
    before: ApplicabilityStatus,
    after: ApplicabilityStatus,
    verdict: MutationVerdict,
}

fn query(context: EvaluationContext) -> ApplicabilityQuery {
    ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            revision: Some("head-a".to_string()),
            ..EvidenceRequirements::default()
        },
        context,
    }
}

fn baseline_context() -> EvaluationContext {
    EvaluationContext {
        repository: Some("owner/repo".to_string()),
        revision: Some("head-a".to_string()),
        work: Some("#17".to_string()),
        path: None,
    }
}

fn mutate_context(
    context: &EvaluationContext,
    mutation: ApplicabilityMutation,
) -> EvaluationContext {
    let mut mutated = context.clone();
    match mutation {
        ApplicabilityMutation::MoveRequiredRevision => {
            mutated.revision = Some("head-b".to_string());
        }
        ApplicabilityMutation::DropRequiredRevisionContext => {
            mutated.revision = None;
        }
        ApplicabilityMutation::MoveUnrequiredRepository => {
            mutated.repository = Some("other/repo".to_string());
        }
    }
    mutated
}

fn mutation_receipt(mutation: ApplicabilityMutation) -> ApplicabilityMutationReceipt {
    let context = baseline_context();
    let before = evaluate_query(&query(context.clone())).unwrap().status;
    let after = evaluate_query(&query(mutate_context(&context, mutation)))
        .unwrap()
        .status;
    let verdict = if before == after {
        MutationVerdict::Survived
    } else {
        MutationVerdict::Killed
    };
    ApplicabilityMutationReceipt {
        mutation,
        before,
        after,
        verdict,
    }
}

#[test]
fn moving_exact_required_revision_kills_applicability_mutation() {
    assert_eq!(
        mutation_receipt(ApplicabilityMutation::MoveRequiredRevision),
        ApplicabilityMutationReceipt {
            mutation: ApplicabilityMutation::MoveRequiredRevision,
            before: ApplicabilityStatus::Applies,
            after: ApplicabilityStatus::Invalid,
            verdict: MutationVerdict::Killed,
        }
    );
}

#[test]
fn removing_required_revision_context_kills_applicability_mutation() {
    assert_eq!(
        mutation_receipt(ApplicabilityMutation::DropRequiredRevisionContext),
        ApplicabilityMutationReceipt {
            mutation: ApplicabilityMutation::DropRequiredRevisionContext,
            before: ApplicabilityStatus::Applies,
            after: ApplicabilityStatus::Unknown,
            verdict: MutationVerdict::Killed,
        }
    );
}

#[test]
fn moving_coordinate_that_evidence_did_not_require_survives_this_oracle() {
    assert_eq!(
        mutation_receipt(ApplicabilityMutation::MoveUnrequiredRepository),
        ApplicabilityMutationReceipt {
            mutation: ApplicabilityMutation::MoveUnrequiredRepository,
            before: ApplicabilityStatus::Applies,
            after: ApplicabilityStatus::Applies,
            verdict: MutationVerdict::Survived,
        }
    );
}
