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

const AGGREGATE_SCHEMA_VERSION: u32 = 1;
const SCOPE_CACHE_NAMESPACE: &str = "test-module-scopes-v1";
const SCOPE_FINGERPRINT_SCHEME: &[u8] = b"cultist-test-module-scope-v1";

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
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
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
            Err(_) => {
                let _ = fs::remove_file(path);
                return None;
            }
        };
        let usable = envelope.schema_version == AGGREGATE_SCHEMA_VERSION
            && envelope.scope_fingerprint == fingerprint;
        if usable {
            Some((envelope.name_counts, envelope.total))
        } else {
            None
        }
    }

    fn store(&self, fingerprint: &str, name_counts: &BTreeMap<String, usize>, total: usize) {
        let path = self.path_for(fingerprint);
        if path.exists() || fs::create_dir_all(&self.root).is_err() {
            return;
        }
        let Ok(bytes) = serde_json::to_vec(&ScopeEnvelope {
            schema_version: AGGREGATE_SCHEMA_VERSION,
            scope_fingerprint: fingerprint.to_string(),
            name_counts: name_counts.clone(),
            total,
        }) else {
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

    #[test]
    fn scope_cache_roundtrip_rejects_corrupt_and_foreign_entries() {
        let root = unique_temp_dir("scope-cache");
        let cache = ScopeCache { root: root.clone() };
        let counts = BTreeMap::from([("tests".to_string(), 3)]);
        let fingerprint = scope_fingerprint(&scope(&[("lib.rs", "aaa")], &[]), Path::new("/repo"));
        cache.store(&fingerprint, &counts, 3);
        assert_eq!(cache.load(&fingerprint), Some((counts.clone(), 3)));

        fs::write(cache.path_for(&fingerprint), "broken json").unwrap();
        assert_eq!(cache.load(&fingerprint), None);

        cache.store(&fingerprint, &counts, 3);
        let mut stale: ScopeEnvelope =
            serde_json::from_slice(&fs::read(cache.path_for(&fingerprint)).unwrap()).unwrap();
        stale.schema_version = AGGREGATE_SCHEMA_VERSION.wrapping_sub(1);
        fs::write(
            cache.path_for(&fingerprint),
            serde_json::to_vec(&stale).unwrap(),
        )
        .unwrap();
        assert_eq!(cache.load(&fingerprint), None);

        cache.store(&fingerprint, &counts, 3);
        let mut foreign: ScopeEnvelope =
            serde_json::from_slice(&fs::read(cache.path_for(&fingerprint)).unwrap()).unwrap();
        foreign.scope_fingerprint =
            scope_fingerprint(&scope(&[("lib.rs", "zzz")], &[]), Path::new("/repo"));
        fs::write(
            cache.path_for(&fingerprint),
            serde_json::to_vec(&foreign).unwrap(),
        )
        .unwrap();
        assert_eq!(cache.load(&fingerprint), None);

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
