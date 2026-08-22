#[allow(dead_code)]
#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;
#[allow(dead_code)]
#[path = "../src/render.rs"]
mod render;

use finding::AnalysisReport;
use render::{render_analysis_report, render_terse_analysis_report};

const SMOLRUNNER_SCAN: &str = include_str!("fixtures/smolrunner_scan_ed3b70e.json");

#[test]
fn retained_smolrunner_scan_executes_the_real_projection_size_ladder() {
    let report: AnalysisReport = serde_json::from_str(SMOLRUNNER_SCAN).unwrap();
    assert_eq!(report.analysis, "test-module-conventions");
    assert_eq!(report.findings.len(), 4);

    let minified_json = serde_json::to_string(&report).unwrap();
    let human = render_analysis_report(&report);
    let c1 = compact_ir::encode_report(&report).unwrap();
    let terse = render_terse_analysis_report(&report);

    // Exact receipt for the retained external scan under the current v1
    // report/render/C1 semantics. Changes here require deliberate remeasurement.
    assert_eq!(SMOLRUNNER_SCAN.len(), 4037);
    assert_eq!(minified_json.len(), 2672);
    assert_eq!(human.len(), 1915);
    assert_eq!(c1.len(), 1981);
    assert_eq!(terse.len(), 957);

    assert_eq!(compact_ir::decode_report(&c1).unwrap(), report);
    assert!(c1.len() < minified_json.len());
    assert!(terse.len() < c1.len());
    assert!(terse.len() < human.len());

    // External negative control: C1 is not universally smaller than the human
    // renderer. Its invariant is lossless typed replay, not "fewest bytes".
    assert!(human.len() < c1.len());
}

#[test]
fn external_terse_projection_drops_only_expandable_receipts_in_this_fixture() {
    let report: AnalysisReport = serde_json::from_str(SMOLRUNNER_SCAN).unwrap();
    let terse = render_terse_analysis_report(&report);
    let c1 = compact_ir::encode_report(&report).unwrap();

    // The current external report's questions and observed claims remain visible.
    assert!(terse.contains("F1 test-module-local-mix"));
    assert!(terse.contains("Q Is the local mix deliberate"));
    assert!(terse.contains("F4 test-module-one-off"));

    // Provenance/support receipts stay recoverable from lossless C1 but are not
    // retransmitted in the terse initial view.
    assert!(!terse.contains("Repository counts:"));
    assert!(!terse.contains("`mod publication_fault` is test-gated here."));
    assert!(c1.contains("Repository counts:"));
    assert!(c1.contains("`mod publication_fault` is test-gated here."));
}
