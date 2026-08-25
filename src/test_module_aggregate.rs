//! Diff-time clean-baseline aggregates for test-module precedent (#50).
//!
//! A small relevant diff combines fresh changed-file facts with cached
//! directory-scoped histograms over the clean (index-content-addressed)
//! remainder of the repository, so a warm analysis never walks or reparses
//! every repository fact row.
//!
//! Exactness contract:
//!
//! - scope aggregates cover only clean rows whose facts are identified by a
//!   Git blob id, keyed by a Merkle fingerprint over those child identities,
//!   the exact repository/scope coordinate, the extractor/schema generation,
//!   and the file-selection configuration;
//! - a row is clean only when Git status reports it untouched, so every
//!   changed, staged, renamed, or untracked path is already absent from the
//!   baseline and the overlay only ever adds fresh changed-file facts;
//! - any dirty/staged/untracked row outside the changed set forces
//!   deterministic fallback to the ordinary full scan, so overlays can never
//!   silently drop a contribution;
//! - missing, corrupt, or version-old cache entries recompute; cache failures
//!   degrade to fallback and never alter findings.
//!
//! Integrity seal: every aggregate entry carries a SHA-256 content digest,
//! computed with domain separation and length framing over its exact
//! serialized semantic payload (schema version, scope fingerprint, the
//! ordered name counts, and the total). Loads recompute and verify the digest
//! before returning any counts, and independently check the invariant that
//! the counted names sum to the stored total. A missing, malformed,
//! mismatched, or old seal is treated as a cache miss; current-schema entries
//! that fail verification are quarantined (deleted) so the deterministic
//! recompute can repair them, exactly like structurally broken entries. This
//! means well-formed but fabricated aggregates — including coherent
//! count/total fabrications that keep the older schema checks passing — can
//! no longer alter findings. Boundary: this is an unkeyed digest, not
//! authentication. It detects accidental corruption and stale or partial
//! writes; it does not defend a malicious same-user actor who deliberately
//! recomputes the digest over forged contents, and no HMAC claim is made.
//! The pre-existing per-file fact cache keeps its own trust model and is
//! unchanged by this contract.
//!
//! Privacy: scope entries persist only aggregated test-module name counts and
//! a total, plus fingerprint/schema metadata, in the user-local cache
//! directory already used for per-file Rust facts. No paths, source text, or
//! line numbers are retained at this layer.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::performance;
use crate::rust_facts::{
    CACHE_NAMESPACE, CACHE_SCHEMA_VERSION, FactCache, RustInput, cached_or_extracted_facts,
    platform_cache_dir, rust_inputs,
};
use crate::test_modules::{SKIPPED_DIRS, TestModuleOccurrence};

/// Version 2 introduces the integrity seal; version 1 entries carried no
/// digest and must never be trusted, so both the schema gate and the cache
/// namespace moved, leaving old entries unreachable.
const AGGREGATE_SCHEMA_VERSION: u32 = 2;
const SCOPE_CACHE_NAMESPACE: &str = "test-module-scopes-v2";
const SCOPE_FINGERPRINT_SCHEME: &[u8] = b"cultist-test-module-scope-v1";
const SCOPE_SEAL_DOMAIN: &[u8] = b"cultist-test-module-scope-seal-v2";

/// Exact repository-wide test-module precedent used to evaluate changed
/// declarations. `name_counts` and `total` include the changed occurrences.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub(crate) struct TestModulePrecedent {
    pub(crate) name_counts: BTreeMap<String, usize>,
    pub(crate) total: usize,
}

impl TestModulePrecedent {
    pub(crate) fn from_occurrences(occurrences: &[TestModuleOccurrence]) -> Self {
        let mut precedent = Self::default();
        for occurrence in occurrences {
            precedent.add(&occurrence.name);
        }
        precedent
    }

    fn add(&mut self, name: &str) {
        *self.name_counts.entry(name.to_string()).or_default() += 1;
        self.total += 1;
    }

    /// Precedent excluding exactly one occurrence, matching the legacy
    /// per-target iteration semantics: other same-name occurrences stay counted.
    pub(crate) fn excluding(&self, target_name: &str) -> (BTreeMap<String, usize>, usize) {
        let mut counts = self.name_counts.clone();
        if let Some(count) = counts.get_mut(target_name) {
            *count -= 1;
            if *count == 0 {
                counts.remove(target_name);
            }
        }
        (counts, self.total.saturating_sub(1))
    }
}

/// Attempts the cached clean-baseline overlay path. `Ok(None)` means the
/// current working state is outside the overlay's exactness envelope and the
/// caller must fall back to the ordinary full repository scan. Callers supply
/// their caches so product and test flows share one deterministic entry point.
pub(crate) fn overlay_precedent_with(
    fact_cache: Option<&FactCache>,
    scope_cache: Option<&ScopeCache>,
    root: &Path,
    changed_paths: &BTreeSet<PathBuf>,
    changed_occurrences: &[TestModuleOccurrence],
) -> Result<Option<TestModulePrecedent>, Box<dyn Error>> {
    let rows = rust_inputs(root, &BTreeSet::new(), SKIPPED_DIRS)?;

    // Dirty/staged/untracked rows have no content identity, so their facts
    // cannot come from the clean baseline. Any such row outside the changed
    // set would be silently dropped from the overlay; fall back instead so
    // counts stay exact.
    for row in &rows {
        if row.content_id.is_none() && !changed_paths.contains(&row.path) {
            return Ok(None);
        }
    }

    let row_paths: BTreeSet<_> = rows.iter().map(|row| row.path.clone()).collect();
    for path in changed_paths {
        if !row_paths.contains(path) {
            return Ok(None);
        }
    }

    let mut stats = ScopeStats::default();
    let clean_tree = CleanScopeTree::build(root, &rows);
    let mut precedent = resolve_scope(scope_cache, fact_cache, root, &clean_tree.root, &mut stats)?;

    for occurrence in changed_occurrences {
        precedent.add(&occurrence.name);
    }

    debug_assert_eq!(
        precedent.total,
        precedent.name_counts.values().sum::<usize>()
    );
    performance::record_baseline_scopes(stats.hits, stats.computed);
    Ok(Some(precedent))
}

#[derive(Debug, Default)]
struct ScopeStats {
    hits: usize,
    computed: usize,
}

/// Directories of clean rows arranged for hierarchical content addressing.
struct CleanScopeTree {
    root: ScopeNode,
}

struct ScopeNode {
    /// Repository-relative directory path; empty for the root scope.
    relative: PathBuf,
    /// Clean Rust files directly in this directory, by file name.
    files: BTreeMap<String, String>,
    children: BTreeMap<String, ScopeNode>,
}

impl CleanScopeTree {
    fn build(root: &Path, rows: &[RustInput]) -> Self {
        let mut tree = Self {
            root: ScopeNode {
                relative: PathBuf::new(),
                files: BTreeMap::new(),
                children: BTreeMap::new(),
            },
        };
        for row in rows {
            let Some(content_id) = row.content_id.as_deref() else {
                continue;
            };
            let Ok(relative) = row.path.strip_prefix(root) else {
                continue;
            };
            let mut components = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned());
            let file_name = components.next_back().unwrap_or_default();
            let mut node = &mut tree.root;
            for component in components {
                let child_relative = node.relative.join(&component);
                node = node.children.entry(component).or_insert_with(|| ScopeNode {
                    relative: child_relative,
                    files: BTreeMap::new(),
                    children: BTreeMap::new(),
                });
            }
            node.files.insert(file_name, content_id.to_string());
        }
        tree
    }
}

/// Resolves the exact aggregate for one scope, serving it from the
/// content-addressed cache when the fingerprint matches and recomputing it
/// deterministically from child identities otherwise.
fn resolve_scope(
    scope_cache: Option<&ScopeCache>,
    fact_cache: Option<&FactCache>,
    root: &Path,
    node: &ScopeNode,
    stats: &mut ScopeStats,
) -> Result<TestModulePrecedent, Box<dyn Error>> {
    let fingerprint = scope_fingerprint(node, root);

    if let Some((name_counts, total)) = scope_cache.and_then(|cache| cache.load(&fingerprint)) {
        stats.hits += 1;
        return Ok(TestModulePrecedent { name_counts, total });
    }

    let mut precedent = TestModulePrecedent::default();
    for (file_name, content_id) in &node.files {
        let path = node_file_path(root, &node.relative, file_name);
        let (facts, hit) = cached_or_extracted_facts(content_id, &path, fact_cache)?;
        performance::record_rust_scan(usize::from(!hit), usize::from(hit));
        for occurrence in &facts.test_modules {
            precedent.add(&occurrence.name);
        }
    }

    for child in node.children.values() {
        let child_precedent = resolve_scope(scope_cache, fact_cache, root, child, stats)?;
        for (name, count) in child_precedent.name_counts {
            *precedent.name_counts.entry(name).or_default() += count;
            precedent.total += count;
        }
    }

    if let Some(cache) = scope_cache {
        cache.store(&fingerprint, &precedent.name_counts, precedent.total);
    }
    stats.computed += 1;
    Ok(precedent)
}

fn node_file_path(root: &Path, relative_dir: &Path, file_name: &str) -> PathBuf {
    if relative_dir.as_os_str().is_empty() {
        root.join(file_name)
    } else {
        root.join(relative_dir).join(file_name)
    }
}

/// Binds the exact repository/scope coordinate, relevant child content
/// identities, the analyzer/extractor/schema generation, and the
/// file-selection configuration into one SHA-256 scope identity.
fn scope_fingerprint(node: &ScopeNode, root: &Path) -> String {
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, SCOPE_FINGERPRINT_SCHEME);
    hash_part(&mut hasher, CACHE_NAMESPACE.as_bytes());
    hash_part(&mut hasher, CACHE_SCHEMA_VERSION.to_string().as_bytes());
    hash_part(&mut hasher, AGGREGATE_SCHEMA_VERSION.to_string().as_bytes());
    hash_part(&mut hasher, &root_coordinate(root));
    hash_part(&mut hasher, &node.fingerprint_material(root));
    hex_digest(&hasher.finalize())
}

impl ScopeNode {
    /// Canonical byte material for this scope: its relative coordinate,
    /// selection configuration, direct clean files, and child fingerprints.
    fn fingerprint_material(&self, root: &Path) -> Vec<u8> {
        let mut material = Vec::new();
        push_framed(&mut material, &self.repository_coordinate_bytes());
        push_framed(&mut material, SKIPPED_DIRS.join("\u{1f}").as_bytes());
        for (file_name, content_id) in &self.files {
            push_framed(&mut material, file_name.as_bytes());
            push_framed(&mut material, content_id.as_bytes());
        }
        for (dir_name, child) in &self.children {
            push_framed(&mut material, dir_name.as_bytes());
            push_framed(&mut material, scope_fingerprint(child, root).as_bytes());
        }
        material
    }

    fn repository_coordinate_bytes(&self) -> Vec<u8> {
        self.relative.to_string_lossy().as_bytes().to_vec()
    }
}

/// The canonical repository path anchors every scope identity to its exact
/// repository coordinate, so entries never leak across checkouts.
fn root_coordinate(root: &Path) -> Vec<u8> {
    let canonical = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    canonical.to_string_lossy().as_bytes().to_vec()
}

fn hash_part(hasher: &mut Sha256, part: &[u8]) {
    hasher.update((part.len() as u64).to_le_bytes());
    hasher.update(part);
}

fn push_framed(material: &mut Vec<u8>, part: &[u8]) {
    material.extend_from_slice(&(part.len() as u64).to_le_bytes());
    material.extend_from_slice(part);
}

/// The integrity seal: a domain-separated, length-framed SHA-256 digest over
/// the exact semantic payload of one aggregate entry. `BTreeMap` iteration is
/// key-sorted, so the encoding is canonical for a given payload.
fn scope_seal(
    schema_version: u32,
    scope_fingerprint: &str,
    name_counts: &BTreeMap<String, usize>,
    total: usize,
) -> String {
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, SCOPE_SEAL_DOMAIN);
    hasher.update(schema_version.to_le_bytes());
    hash_part(&mut hasher, scope_fingerprint.as_bytes());
    hasher.update((name_counts.len() as u64).to_le_bytes());
    for (name, count) in name_counts {
        hash_part(&mut hasher, name.as_bytes());
        hasher.update((*count as u64).to_le_bytes());
    }
    hasher.update((total as u64).to_le_bytes());
    hex_digest(&hasher.finalize())
}

fn hex_digest(digest: &[u8]) -> String {
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Content-addressed store for scope aggregates, mirroring the per-file fact
/// cache controls (`CARGO_CULTIST_CACHE`, `CARGO_CULTIST_CACHE_DIR`).
pub(crate) struct ScopeCache {
    pub(crate) root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScopeEnvelope {
    schema_version: u32,
    scope_fingerprint: String,
    name_counts: BTreeMap<String, usize>,
    total: usize,
    /// Required integrity seal over the exact payload above; entries without
    /// it fail deserialization and are treated as corrupt.
    integrity_seal: String,
}

impl ScopeEnvelope {
    /// The checked arithmetic invariant: counted names must sum exactly to
    /// the stored total, with no overflow.
    fn counts_sum_to_total(&self) -> bool {
        self.name_counts
            .values()
            .try_fold(0usize, |sum, count| sum.checked_add(*count))
            == Some(self.total)
    }

    fn seal_matches(&self) -> bool {
        self.integrity_seal
            == scope_seal(
                self.schema_version,
                &self.scope_fingerprint,
                &self.name_counts,
                self.total,
            )
    }
}

impl ScopeCache {
    pub(crate) fn from_environment() -> Option<Self> {
        if std::env::var_os("CARGO_CULTIST_CACHE").is_some_and(|value| value == "0") {
            return None;
        }

        if let Some(path) = std::env::var_os("CARGO_CULTIST_CACHE_DIR") {
            return Some(Self {
                root: PathBuf::from(path).join(SCOPE_CACHE_NAMESPACE),
            });
        }

        let base = platform_cache_dir()?;
        Some(Self {
            root: base.join("cargo-cultist").join(SCOPE_CACHE_NAMESPACE),
        })
    }

    fn load(&self, fingerprint: &str) -> Option<(BTreeMap<String, usize>, usize)> {
        let path = self.path_for(fingerprint);
        let bytes = fs::read(&path).ok()?;
        let envelope: ScopeEnvelope = match serde_json::from_slice(&bytes) {
            Ok(envelope) => envelope,
            // Structurally broken data (including a missing seal field) is
            // quarantined so the next recompute can repair the entry.
            Err(_) => {
                let _ = fs::remove_file(path);
                return None;
            }
        };
        let current_schema = envelope.schema_version == AGGREGATE_SCHEMA_VERSION
            && envelope.scope_fingerprint == fingerprint;
        if !current_schema {
            // Old-schema or foreign-coordinate entries are simply ignored,
            // matching the historical stale-entry behavior.
            return None;
        }
        // A well-formed entry that fails its own integrity verification is
        // corrupt at the current contract: quarantine it like broken bytes so
        // the deterministic recompute below can rewrite it.
        if !envelope.seal_matches() || !envelope.counts_sum_to_total() {
            let _ = fs::remove_file(path);
            return None;
        }
        Some((envelope.name_counts, envelope.total))
    }

    fn store(&self, fingerprint: &str, name_counts: &BTreeMap<String, usize>, total: usize) {
        let path = self.path_for(fingerprint);
        if path.exists() || fs::create_dir_all(&self.root).is_err() {
            return;
        }
        let envelope = ScopeEnvelope {
            schema_version: AGGREGATE_SCHEMA_VERSION,
            scope_fingerprint: fingerprint.to_string(),
            name_counts: name_counts.clone(),
            total,
            integrity_seal: scope_seal(AGGREGATE_SCHEMA_VERSION, fingerprint, name_counts, total),
        };
        let Ok(bytes) = serde_json::to_vec(&envelope) else {
            return;
        };

        let temporary = self
            .root
            .join(format!(".{fingerprint}.{}.tmp", std::process::id()));
        if fs::write(&temporary, bytes).is_ok() {
            let _ = fs::rename(&temporary, &path);
        }
        let _ = fs::remove_file(temporary);
    }

    fn path_for(&self, fingerprint: &str) -> PathBuf {
        self.root.join(format!("{fingerprint}.json"))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cargo-cultist-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn scope(files: &[(&str, &str)], children: &[(&str, &str)]) -> ScopeNode {
        ScopeNode {
            relative: PathBuf::new(),
            files: files
                .iter()
                .map(|(name, oid)| ((*name).to_string(), (*oid).to_string()))
                .collect(),
            children: children
                .iter()
                .map(|(name, oid)| {
                    (
                        (*name).to_string(),
                        ScopeNode {
                            relative: PathBuf::from(*name),
                            files: BTreeMap::from([("inner.rs".to_string(), (*oid).to_string())]),
                            children: BTreeMap::new(),
                        },
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn fingerprint_binds_child_identity_config_and_coordinate() {
        let root_a = Path::new("/repo-a");
        let root_b = Path::new("/repo-b");
        let baseline = scope(&[("lib.rs", "aaa")], &[("sub", "bbb")]);
        let changed_file_oid = scope(&[("lib.rs", "ccc")], &[("sub", "bbb")]);
        let changed_child_oid = scope(&[("lib.rs", "aaa")], &[("sub", "ddd")]);

        assert_eq!(
            scope_fingerprint(&baseline, root_a),
            scope_fingerprint(&baseline, root_a)
        );
        assert_ne!(
            scope_fingerprint(&baseline, root_a),
            scope_fingerprint(&changed_file_oid, root_a)
        );
        assert_ne!(
            scope_fingerprint(&baseline, root_a),
            scope_fingerprint(&changed_child_oid, root_a)
        );
        // The exact repository coordinate participates in the identity.
        assert_ne!(
            scope_fingerprint(&baseline, root_a),
            scope_fingerprint(&baseline, root_b)
        );

        let mut relocated = scope(&[("lib.rs", "aaa")], &[("sub", "bbb")]);
        relocated.relative = PathBuf::from("elsewhere");
        assert_ne!(
            scope_fingerprint(&baseline, root_a),
            scope_fingerprint(&relocated, root_a)
        );
    }

    fn sealed_envelope(
        fingerprint: &str,
        counts: &BTreeMap<String, usize>,
        total: usize,
    ) -> ScopeEnvelope {
        ScopeEnvelope {
            schema_version: AGGREGATE_SCHEMA_VERSION,
            scope_fingerprint: fingerprint.to_string(),
            name_counts: counts.clone(),
            total,
            integrity_seal: scope_seal(AGGREGATE_SCHEMA_VERSION, fingerprint, counts, total),
        }
    }

    fn write_envelope(cache: &ScopeCache, fingerprint: &str, envelope: &ScopeEnvelope) {
        fs::write(
            cache.path_for(fingerprint),
            serde_json::to_vec(envelope).unwrap(),
        )
        .unwrap();
    }

    fn sample_counts() -> BTreeMap<String, usize> {
        BTreeMap::from([("tests".to_string(), 3)])
    }

    #[test]
    fn scope_cache_roundtrip_rejects_corrupt_and_foreign_entries() {
        let root = unique_temp_dir("scope-cache");
        let cache = ScopeCache { root: root.clone() };
        let counts = sample_counts();
        let fingerprint = scope_fingerprint(&scope(&[("lib.rs", "aaa")], &[]), Path::new("/repo"));
        cache.store(&fingerprint, &counts, 3);
        assert_eq!(cache.load(&fingerprint), Some((counts.clone(), 3)));

        fs::write(cache.path_for(&fingerprint), "broken json").unwrap();
        assert_eq!(cache.load(&fingerprint), None);

        cache.store(&fingerprint, &counts, 3);
        let mut stale: ScopeEnvelope =
            serde_json::from_slice(&fs::read(cache.path_for(&fingerprint)).unwrap()).unwrap();
        stale.schema_version = AGGREGATE_SCHEMA_VERSION.wrapping_sub(1);
        // Re-seal so only the schema gate can reject this entry.
        stale.integrity_seal = scope_seal(
            stale.schema_version,
            &stale.scope_fingerprint,
            &stale.name_counts,
            stale.total,
        );
        write_envelope(&cache, &fingerprint, &stale);
        assert_eq!(cache.load(&fingerprint), None);

        cache.store(&fingerprint, &counts, 3);
        let mut foreign: ScopeEnvelope =
            serde_json::from_slice(&fs::read(cache.path_for(&fingerprint)).unwrap()).unwrap();
        foreign.scope_fingerprint =
            scope_fingerprint(&scope(&[("lib.rs", "zzz")], &[]), Path::new("/repo"));
        // Re-seal so only the coordinate gate can reject this entry.
        foreign.integrity_seal = scope_seal(
            foreign.schema_version,
            &foreign.scope_fingerprint,
            &foreign.name_counts,
            foreign.total,
        );
        write_envelope(&cache, &fingerprint, &foreign);
        assert_eq!(cache.load(&fingerprint), None);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tampered_payloads_miss_and_are_quarantined_even_when_coherent() {
        let root = unique_temp_dir("scope-tamper");
        fs::create_dir_all(&root).unwrap();
        let cache = ScopeCache { root: root.clone() };
        let fingerprint = scope_fingerprint(&scope(&[("lib.rs", "aaa")], &[]), Path::new("/repo"));
        let entry_path = cache.path_for(&fingerprint);

        // Counts mutated under a valid-looking envelope with the old seal.
        let fabricated_counts = BTreeMap::from([("tests".to_string(), 41)]);
        let mut tampered = sealed_envelope(&fingerprint, &sample_counts(), 3);
        tampered.name_counts = fabricated_counts.clone();
        write_envelope(&cache, &fingerprint, &tampered);
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!entry_path.exists(), "tampered entry must be quarantined");

        // Total mutated alone, old seal intact.
        let mut tampered = sealed_envelope(&fingerprint, &sample_counts(), 3);
        tampered.total = 99;
        write_envelope(&cache, &fingerprint, &tampered);
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!entry_path.exists());

        // Coherent fabrication: counts and total agree with each other but
        // the payload was written by someone else; the stale seal must catch it.
        let mut coherent = sealed_envelope(&fingerprint, &sample_counts(), 3);
        coherent.name_counts = BTreeMap::from([
            ("tests".to_string(), 40),
            ("fabricated_tests".to_string(), 60),
        ]);
        coherent.total = 100;
        write_envelope(&cache, &fingerprint, &coherent);
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!entry_path.exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_malformed_or_old_seals_never_trust_the_entry() {
        let root = unique_temp_dir("scope-seal-shapes");
        fs::create_dir_all(&root).unwrap();
        let cache = ScopeCache { root: root.clone() };
        let fingerprint = scope_fingerprint(&scope(&[("lib.rs", "aaa")], &[]), Path::new("/repo"));
        let entry_path = cache.path_for(&fingerprint);
        let counts = sample_counts();

        // A version-1-shaped unsealed entry must not deserialize into the
        // current contract, even if dropped into the new namespace.
        let legacy = serde_json::json!({
            "schema_version": 1,
            "scope_fingerprint": fingerprint,
            "name_counts": counts,
            "total": 3,
        });
        fs::write(&entry_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!entry_path.exists(), "unsealed entry must be quarantined");

        // A current-schema entry whose seal field is missing is malformed.
        let mut unsealed = serde_json::to_value(sealed_envelope(&fingerprint, &counts, 3)).unwrap();
        unsealed
            .as_object_mut()
            .unwrap()
            .remove("integrity_seal")
            .unwrap();
        fs::write(&entry_path, serde_json::to_vec(&unsealed).unwrap()).unwrap();
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!entry_path.exists());

        // A garbled seal value is a mismatched digest, not a usable one.
        let mut garbled = sealed_envelope(&fingerprint, &counts, 3);
        garbled.integrity_seal = "0".repeat(63) + "z";
        write_envelope(&cache, &fingerprint, &garbled);
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!entry_path.exists());

        // Truncated serialization never loads.
        let valid = serde_json::to_vec(&sealed_envelope(&fingerprint, &counts, 3)).unwrap();
        fs::write(&entry_path, &valid[..valid.len() / 2]).unwrap();
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!entry_path.exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invariant_violation_misses_even_with_an_honestly_computed_seal() {
        let root = unique_temp_dir("scope-invariant");
        fs::create_dir_all(&root).unwrap();
        let cache = ScopeCache { root: root.clone() };
        let fingerprint = scope_fingerprint(&scope(&[("lib.rs", "aaa")], &[]), Path::new("/repo"));
        let counts = BTreeMap::from([("tests".to_string(), 1)]);

        // The seal honestly covers this exact payload, so only the checked
        // sum invariant can reject it.
        let inconsistent = ScopeEnvelope {
            integrity_seal: scope_seal(AGGREGATE_SCHEMA_VERSION, &fingerprint, &counts, 99),
            schema_version: AGGREGATE_SCHEMA_VERSION,
            scope_fingerprint: fingerprint.clone(),
            name_counts: counts,
            total: 99,
        };
        write_envelope(&cache, &fingerprint, &inconsistent);
        assert_eq!(cache.load(&fingerprint), None);
        assert!(!cache.path_for(&fingerprint).exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn quarantined_entries_are_repaired_by_deterministic_recompute() {
        let root = unique_temp_dir("scope-repair");
        fs::create_dir_all(&root).unwrap();
        let cache = ScopeCache { root: root.clone() };
        let fingerprint = scope_fingerprint(&scope(&[("lib.rs", "aaa")], &[]), Path::new("/repo"));
        let counts = sample_counts();

        let mut tampered = sealed_envelope(&fingerprint, &counts, 3);
        tampered.total += 7;
        write_envelope(&cache, &fingerprint, &tampered);
        assert_eq!(cache.load(&fingerprint), None);

        // The recompute path stores again once the corrupt entry is gone.
        cache.store(&fingerprint, &counts, 3);
        assert_eq!(
            cache.load(&fingerprint),
            Some((counts.clone(), 3)),
            "the repaired entry must carry a fresh valid seal"
        );

        // Restoring byte-identical semantics is stable across repairs.
        let repaired = fs::read(cache.path_for(&fingerprint)).unwrap();
        let replay = serde_json::to_vec(&sealed_envelope(&fingerprint, &counts, 3)).unwrap();
        assert_eq!(repaired, replay);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn concurrent_stores_stay_atomic_and_deterministic() {
        let root = unique_temp_dir("scope-concurrent");
        let cache = std::sync::Arc::new(ScopeCache { root: root.clone() });
        let base_node = scope(&[("lib.rs", "aaa")], &[]);

        let handles: Vec<_> = (0..8)
            .map(|index| {
                let cache = std::sync::Arc::clone(&cache);
                let node = ScopeNode {
                    relative: PathBuf::from(format!("scope-{index}")),
                    files: base_node.files.clone(),
                    children: BTreeMap::new(),
                };
                std::thread::spawn(move || {
                    let fingerprint = scope_fingerprint(&node, Path::new("/repo"));
                    let counts = BTreeMap::from([(format!("tests_{index}"), index + 1)]);
                    cache.store(&fingerprint, &counts, index + 1);
                    // Concurrent duplicate writes of one scope stay atomic.
                    cache.store(&fingerprint, &counts, index + 1);
                    fingerprint
                })
            })
            .collect();

        for handle in handles {
            let fingerprint = handle.join().unwrap();
            let loaded = cache.load(&fingerprint).expect("entry must survive races");
            let expected_total = loaded.1;
            let sum: usize = loaded.0.values().sum();
            assert_eq!(sum, expected_total, "racing writes must stay coherent");
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn overlay_arithmetic_matches_direct_counting() {
        let occurrences = vec![
            TestModuleOccurrence {
                name: "tests".to_string(),
                path: PathBuf::from("/repo/src/a.rs"),
                line: 1,
            },
            TestModuleOccurrence {
                name: "tests".to_string(),
                path: PathBuf::from("/repo/src/b.rs"),
                line: 2,
            },
            TestModuleOccurrence {
                name: "unit_tests".to_string(),
                path: PathBuf::from("/repo/src/c.rs"),
                line: 3,
            },
        ];
        let precedent = TestModulePrecedent::from_occurrences(&occurrences);
        assert_eq!(precedent.total, 3);

        let (excluding_target, total) = precedent.excluding("tests");
        assert_eq!(excluding_target.get("tests"), Some(&1));
        assert_eq!(excluding_target.get("unit_tests"), Some(&1));
        assert_eq!(total, 2);

        let (absent, total) = precedent.excluding("missing");
        assert!(!absent.contains_key("missing"));
        assert_eq!(total, 2);
    }

    #[test]
    fn clean_tree_groups_rows_by_directory_and_ignores_volatile_rows() {
        let root = PathBuf::from("/repo");
        let rows = vec![
            RustInput {
                path: root.join("root.rs"),
                content_id: Some("aaaa".to_string()),
            },
            RustInput {
                path: root.join("src/lib.rs"),
                content_id: Some("bbbb".to_string()),
            },
            RustInput {
                path: root.join("src/deep/util.rs"),
                content_id: None,
            },
        ];

        let tree = CleanScopeTree::build(&root, &rows);

        assert_eq!(tree.root.files.keys().collect::<Vec<_>>(), ["root.rs"]);
        let src = tree.root.children.get("src").unwrap();
        assert_eq!(src.files.keys().collect::<Vec<_>>(), ["lib.rs"]);
        // A directory holding only volatile rows never joins the tree.
        assert!(!src.children.contains_key("deep"));
        assert_eq!(src.relative, PathBuf::from("src"));
    }

    #[test]
    fn resolve_scope_computes_from_disk_and_serves_followups_from_cache() {
        let root = unique_temp_dir("resolve-scope");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(
            root.join("lib.rs"),
            "#[cfg(test)]\nmod lib_tests {}\n#[cfg(test)]\nmod shared {}\n",
        )
        .unwrap();
        fs::write(
            root.join("sub/inner.rs"),
            "#[cfg(test)]\nmod inner_tests {}\n",
        )
        .unwrap();

        let node = ScopeNode {
            relative: PathBuf::new(),
            files: BTreeMap::from([("lib.rs".to_string(), "deadbeef".to_string())]),
            children: BTreeMap::from([(
                "sub".to_string(),
                ScopeNode {
                    relative: PathBuf::from("sub"),
                    files: BTreeMap::from([("inner.rs".to_string(), "feedface".to_string())]),
                    children: BTreeMap::new(),
                },
            )]),
        };

        let cache_root = unique_temp_dir("resolve-scope-cache");
        let scope_cache = ScopeCache {
            root: cache_root.clone(),
        };
        let mut stats = ScopeStats::default();
        let resolved = resolve_scope(Some(&scope_cache), None, &root, &node, &mut stats).unwrap();
        assert_eq!(stats.computed, 2);
        assert_eq!(stats.hits, 0);
        assert_eq!(resolved.total, 3);
        assert_eq!(resolved.name_counts.get("shared"), Some(&1));

        let mut warm_stats = ScopeStats::default();
        let warmed =
            resolve_scope(Some(&scope_cache), None, &root, &node, &mut warm_stats).unwrap();
        assert_eq!(resolved, warmed);
        assert_eq!(warm_stats.hits, 1);
        assert_eq!(warm_stats.computed, 0);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(cache_root).unwrap();
    }
}
