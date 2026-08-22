#![allow(dead_code)]

#[path = "../src/coordination_edges.rs"]
mod coordination_edges;

use serde_json::{Value, json};

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 40).collect()
}

fn work(id: &str, body: &str) -> Value {
    json!({
        "id": id,
        "kind": "pull_request",
        "source": format!("github:pull/{}", &id[1..]),
        "head_sha": sha('a'),
        "updated_at": "2026-08-22T18:30:00Z",
        "body": body,
    })
}

fn snapshot(first: Value, second: Value) -> String {
    serde_json::to_string(&json!({
        "schema_version": 1,
        "source": "test:unresolved-coordination-origin",
        "work": [first, second],
    }))
    .unwrap()
}

#[test]
fn unresolved_endpoint_erases_which_work_item_authored_the_clause() {
    let current_authored = coordination_edges::extract_snapshot(&snapshot(
        work("#748", "Do not merge while #999 is active\n"),
        work("#703", ""),
    ))
    .unwrap();

    let other_authored = coordination_edges::extract_snapshot(&snapshot(
        work("#748", ""),
        work("#703", "Do not merge while #999 is active\n"),
    ))
    .unwrap();

    assert!(current_authored.coordination_edges.is_empty());
    assert!(current_authored.source_receipts.is_empty());
    assert_eq!(current_authored.stats.unresolved_endpoints_ignored, 1);

    assert!(other_authored.coordination_edges.is_empty());
    assert!(other_authored.source_receipts.is_empty());
    assert_eq!(other_authored.stats.unresolved_endpoints_ignored, 1);

    assert_eq!(current_authored, other_authored);
}

#[test]
fn resolved_endpoint_preserves_authorship_as_a_control() {
    let current_authored = coordination_edges::extract_snapshot(&snapshot(
        work("#748", "Do not merge while #703 is active\n"),
        work("#703", ""),
    ))
    .unwrap();

    let other_authored = coordination_edges::extract_snapshot(&snapshot(
        work("#748", ""),
        work("#703", "Do not merge while #748 is active\n"),
    ))
    .unwrap();

    assert_ne!(current_authored, other_authored);
    assert_eq!(current_authored.coordination_edges.len(), 1);
    assert_eq!(current_authored.coordination_edges[0].from, "#748");
    assert_eq!(current_authored.coordination_edges[0].to, "#703");
    assert_eq!(
        current_authored.source_receipts[0].source,
        "github:pull/748"
    );

    assert_eq!(other_authored.coordination_edges.len(), 1);
    assert_eq!(other_authored.coordination_edges[0].from, "#703");
    assert_eq!(other_authored.coordination_edges[0].to, "#748");
    assert_eq!(other_authored.source_receipts[0].source, "github:pull/703");
}
