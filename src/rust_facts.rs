use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::process::Command;

use proc_macro2::Span;
use serde::{Deserialize, Serialize};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemFn, ItemMod, Meta, Token};
use walkdir::{DirEntry, WalkDir};

use crate::performance;

pub(crate) const CACHE_SCHEMA_VERSION: u32 = 1;
pub(crate) const CACHE_NAMESPACE: &str = "rust-syntax-v1";

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NamedRustFact {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustFileFacts {
    pub test_modules: Vec<NamedRustFact>,
    pub explicit_tests: Vec<NamedRustFact>,
    pub module_names: Vec<String>,
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustFactFile {
    pub path: PathBuf,
    pub facts: RustFileFacts,
}

#[derive(Debug, Default)]
pub struct RustFactScan {
    pub files: Vec<RustFactFile>,
    pub cache_hits: usize,
    pub parsed_files: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RustInput {
    pub(crate) path: PathBuf,
    pub(crate) content_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct FactCache {
    pub(crate) root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEnvelope {
    schema_version: u32,
    facts: RustFileFacts,
}

/// Loads content-addressed facts for one clean input, deterministically
/// extracting and storing them on a cache miss. The boolean result reports
/// whether the facts came from the cache.
pub(crate) fn cached_or_extracted_facts(
    content_id: &str,
    path: &Path,
    cache: Option<&FactCache>,
) -> Result<(RustFileFacts, bool), Box<dyn Error>> {
    if let Some(facts) = cache.and_then(|cache| cache.load(content_id)) {
        return Ok((facts, true));
    }
    let facts = extract_rust_file(path)?;
    if let Some(cache) = cache {
        cache.store(content_id, &facts);
    }
    Ok((facts, false))
}

pub fn scan_rust_repository(
    root: &Path,
    excluded_paths: &BTreeSet<PathBuf>,
    skipped_dirs: &[&str],
) -> Result<RustFactScan, Box<dyn Error>> {
    let inputs = rust_inputs(root, excluded_paths, skipped_dirs)?;
    let cache = FactCache::from_environment();
    scan_inputs(&inputs, cache.as_ref())
}

pub fn scan_rust_paths(paths: &[PathBuf]) -> Result<RustFactScan, Box<dyn Error>> {
    let inputs: Vec<_> = paths
        .iter()
        .filter(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("rs") && path.is_file()
        })
        .map(|path| RustInput {
            path: path.clone(),
            content_id: None,
        })
        .collect();
    scan_inputs(&inputs, None)
}

fn scan_inputs(
    inputs: &[RustInput],
    cache: Option<&FactCache>,
) -> Result<RustFactScan, Box<dyn Error>> {
    let mut scan = RustFactScan::default();

    for input in inputs {
        let cached = input
            .content_id
            .as_deref()
            .and_then(|content_id| cache.and_then(|cache| cache.load(content_id)));

        let facts = if let Some(facts) = cached {
            scan.cache_hits += 1;
            facts
        } else {
            scan.parsed_files += 1;
            let facts = extract_rust_file(&input.path)?;
            if let (Some(cache), Some(content_id)) = (cache, input.content_id.as_deref()) {
                cache.store(content_id, &facts);
            }
            facts
        };

        scan.files.push(RustFactFile {
            path: input.path.clone(),
            facts,
        });
    }

    scan.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(scan)
}

fn extract_rust_file(path: &Path) -> Result<RustFileFacts, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    Ok(extract_rust_source(&source))
}

fn extract_rust_source(source: &str) -> RustFileFacts {
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => {
            return RustFileFacts {
                parse_error: Some(error.to_string()),
                ..RustFileFacts::default()
            };
        }
    };

    let mut facts = RustFileFacts::default();
    let mut visitor = RustFactVisitor { facts: &mut facts };
    visitor.visit_file(&file);
    facts
}

pub(crate) fn rust_inputs(
    root: &Path,
    excluded_paths: &BTreeSet<PathBuf>,
    skipped_dirs: &[&str],
) -> Result<Vec<RustInput>, Box<dyn Error>> {
    if let Some(inputs) = git_rust_inputs(root, excluded_paths, skipped_dirs)? {
        return Ok(inputs);
    }

    let mut inputs = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_visit(entry, skipped_dirs))
    {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs")
            || excluded_paths.contains(entry.path())
        {
            continue;
        }
        inputs.push(RustInput {
            path: entry.path().to_path_buf(),
            content_id: None,
        });
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(inputs)
}

fn git_rust_inputs(
    root: &Path,
    excluded_paths: &BTreeSet<PathBuf>,
    skipped_dirs: &[&str],
) -> Result<Option<Vec<RustInput>>, Box<dyn Error>> {
    let probe = performance::git_command()
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if !probe.status.success() || probe.stdout != b"true\n" {
        return Ok(None);
    }

    let inventory = git_output(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ],
    )?;
    let Some(paths) = parse_nul_paths(&inventory) else {
        return Ok(None);
    };

    let staged = git_output(root, &["ls-files", "-s", "-z", "--", "*.rs"])?;
    let Some(index_ids) = parse_index_ids(&staged) else {
        return Ok(None);
    };

    let status = git_output(
        root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            "*.rs",
        ],
    )?;
    let Some(dirty_paths) = parse_dirty_paths(&status) else {
        return Ok(None);
    };

    let mut inputs = Vec::new();
    for relative in paths {
        if path_has_skipped_dir(&relative, skipped_dirs) {
            continue;
        }
        let path = root.join(&relative);
        if excluded_paths.contains(&path)
            || !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_file())
        {
            continue;
        }

        let content_id = if dirty_paths.contains(&relative) {
            None
        } else {
            index_ids.get(&relative).cloned()
        };
        inputs.push(RustInput { path, content_id });
    }

    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Some(inputs))
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = performance::git_command()
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.first().copied().unwrap_or("command"),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output.stdout)
}

fn parse_nul_paths(output: &[u8]) -> Option<Vec<PathBuf>> {
    output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| std::str::from_utf8(record).ok().map(PathBuf::from))
        .collect()
}

fn parse_index_ids(output: &[u8]) -> Option<BTreeMap<PathBuf, String>> {
    let mut ids = BTreeMap::new();
    for record in output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let record = std::str::from_utf8(record).ok()?;
        let (metadata, path) = record.split_once('\t')?;
        let mut fields = metadata.split_whitespace();
        let _mode = fields.next()?;
        let object_id = fields.next()?;
        let stage = fields.next()?;
        if stage == "0" && valid_content_id(object_id) {
            ids.insert(PathBuf::from(path), object_id.to_string());
        }
    }
    Some(ids)
}

fn parse_dirty_paths(output: &[u8]) -> Option<BTreeSet<PathBuf>> {
    let records: Vec<_> = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .collect();
    let mut dirty = BTreeSet::new();
    let mut index = 0;

    while index < records.len() {
        let record = std::str::from_utf8(records[index]).ok()?;
        let bytes = record.as_bytes();
        if bytes.len() < 4 || bytes[2] != b' ' {
            return None;
        }
        let status = &record[..2];
        dirty.insert(PathBuf::from(&record[3..]));

        if status
            .as_bytes()
            .iter()
            .any(|byte| matches!(byte, b'R' | b'C'))
        {
            index += 1;
            let other = std::str::from_utf8(records.get(index)?).ok()?;
            dirty.insert(PathBuf::from(other));
        }
        index += 1;
    }

    Some(dirty)
}

fn valid_content_id(value: &str) -> bool {
    (32..=128).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn should_visit(entry: &DirEntry, skipped_dirs: &[&str]) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    !entry
        .file_name()
        .to_str()
        .is_some_and(|name| skipped_dirs.contains(&name))
}

fn path_has_skipped_dir(path: &Path, skipped_dirs: &[&str]) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => name
            .to_str()
            .is_some_and(|name| skipped_dirs.contains(&name)),
        _ => false,
    })
}

impl FactCache {
    pub(crate) fn from_environment() -> Option<Self> {
        if env::var_os("CARGO_CULTIST_CACHE").is_some_and(|value| value == "0") {
            return None;
        }

        if let Some(path) = env::var_os("CARGO_CULTIST_CACHE_DIR") {
            return Some(Self {
                root: PathBuf::from(path).join(CACHE_NAMESPACE),
            });
        }

        let base = platform_cache_dir()?;
        Some(Self {
            root: base.join("cargo-cultist").join(CACHE_NAMESPACE),
        })
    }

    pub(crate) fn load(&self, content_id: &str) -> Option<RustFileFacts> {
        let path = self.path_for(content_id)?;
        let bytes = fs::read(&path).ok()?;
        let envelope: CacheEnvelope = match serde_json::from_slice(&bytes) {
            Ok(envelope) => envelope,
            Err(_) => {
                let _ = fs::remove_file(path);
                return None;
            }
        };
        (envelope.schema_version == CACHE_SCHEMA_VERSION).then_some(envelope.facts)
    }

    pub(crate) fn store(&self, content_id: &str, facts: &RustFileFacts) {
        let Some(path) = self.path_for(content_id) else {
            return;
        };
        if path.exists() || fs::create_dir_all(&self.root).is_err() {
            return;
        }
        let Ok(bytes) = serde_json::to_vec(&CacheEnvelope {
            schema_version: CACHE_SCHEMA_VERSION,
            facts: facts.clone(),
        }) else {
            return;
        };

        let temporary = self
            .root
            .join(format!(".{content_id}.{}.tmp", std::process::id()));
        if fs::write(&temporary, bytes).is_ok() {
            let _ = fs::rename(&temporary, &path);
        }
        let _ = fs::remove_file(temporary);
    }

    fn path_for(&self, content_id: &str) -> Option<PathBuf> {
        valid_content_id(content_id).then(|| self.root.join(format!("{content_id}.json")))
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn platform_cache_dir() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

#[cfg(target_os = "macos")]
pub(crate) fn platform_cache_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Caches"))
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn platform_cache_dir() -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
}

#[cfg(not(any(unix, target_os = "windows")))]
pub(crate) fn platform_cache_dir() -> Option<PathBuf> {
    None
}

struct RustFactVisitor<'a> {
    facts: &'a mut RustFileFacts,
}

impl<'ast> Visit<'ast> for RustFactVisitor<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if has_test_attr(&node.attrs) {
            self.facts.explicit_tests.push(NamedRustFact {
                name: node.sig.ident.to_string(),
                line: span_line(node.sig.ident.span()),
            });
        }
        visit::visit_item_fn(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        self.facts.module_names.push(node.ident.to_string());
        if is_test_module(&node.attrs) {
            self.facts.test_modules.push(NamedRustFact {
                name: node.ident.to_string(),
                line: span_line(node.ident.span()),
            });
        }
        visit::visit_item_mod(self, node);
    }
}

fn span_line(span: Span) -> usize {
    span.start().line
}

fn has_test_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

fn is_test_module(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .any(|attr| match &attr.meta {
            Meta::List(list) => parse_meta_list(list.tokens.clone())
                .is_some_and(|metas| metas.iter().any(|meta| meta_mentions_test(meta, false))),
            _ => false,
        })
}

fn parse_meta_list(tokens: proc_macro2::TokenStream) -> Option<Punctuated<Meta, Token![,]>> {
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(tokens)
        .ok()
}

fn meta_mentions_test(meta: &Meta, negated: bool) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test") && !negated,
        Meta::List(list) => {
            let nested_negated = if list.path.is_ident("not") {
                !negated
            } else {
                negated
            };
            parse_meta_list(list.tokens.clone()).is_some_and(|metas| {
                metas
                    .iter()
                    .any(|meta| meta_mentions_test(meta, nested_negated))
            })
        }
        Meta::NameValue(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "cargo-cultist-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: git {args:?}");
    }

    fn init_repo(name: &str) -> PathBuf {
        let root = unique_temp_dir(name);
        fs::create_dir_all(root.join("src")).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);
        root
    }

    #[test]
    fn extracts_shared_syntax_facts_in_one_parse() {
        let facts = extract_rust_source(
            r#"
            mod support {}
            #[cfg(test)]
            mod tests {
                #[test]
                fn works() {}
            }
            "#,
        );

        assert_eq!(facts.test_modules.len(), 1);
        assert_eq!(facts.test_modules[0].name, "tests");
        assert_eq!(facts.explicit_tests.len(), 1);
        assert_eq!(facts.explicit_tests[0].name, "works");
        assert!(facts.module_names.contains(&"support".to_string()));
        assert!(facts.module_names.contains(&"tests".to_string()));
        assert_eq!(facts.parse_error, None);
    }

    #[test]
    fn clean_git_blob_hits_cache_and_reuses_across_rename() {
        let root = init_repo("rust-fact-cache");
        let cache = FactCache {
            root: root.join("cache"),
        };
        fs::write(
            root.join("src/lib.rs"),
            "#[cfg(test)]\nmod tests { #[test] fn works() {} }\n",
        )
        .unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);

        let inputs = rust_inputs(&root, &BTreeSet::new(), &[".git", "target"]).unwrap();
        let first = scan_inputs(&inputs, Some(&cache)).unwrap();
        assert_eq!(first.parsed_files, 1);
        assert_eq!(first.cache_hits, 0);

        let second = scan_inputs(&inputs, Some(&cache)).unwrap();
        assert_eq!(second.parsed_files, 0);
        assert_eq!(second.cache_hits, 1);

        run_git(&root, &["mv", "src/lib.rs", "src/renamed.rs"]);
        run_git(&root, &["commit", "-q", "-m", "rename"]);
        let renamed_inputs = rust_inputs(&root, &BTreeSet::new(), &[".git", "target"]).unwrap();
        let renamed = scan_inputs(&renamed_inputs, Some(&cache)).unwrap();
        assert_eq!(renamed.parsed_files, 0);
        assert_eq!(renamed.cache_hits, 1);
        assert!(renamed.files[0].path.ends_with("src/renamed.rs"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dirty_git_file_bypasses_clean_blob_cache() {
        let root = init_repo("rust-fact-dirty");
        let cache = FactCache {
            root: root.join("cache"),
        };
        let source = root.join("src/lib.rs");
        fs::write(&source, "#[cfg(test)]\nmod tests {}\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);

        let inputs = rust_inputs(&root, &BTreeSet::new(), &[".git", "target"]).unwrap();
        let _ = scan_inputs(&inputs, Some(&cache)).unwrap();

        fs::write(&source, "#[cfg(test)]\nmod changed_tests {}\n").unwrap();
        let dirty_inputs = rust_inputs(&root, &BTreeSet::new(), &[".git", "target"]).unwrap();
        let dirty = scan_inputs(&dirty_inputs, Some(&cache)).unwrap();
        assert_eq!(dirty.cache_hits, 0);
        assert_eq!(dirty.parsed_files, 1);
        assert_eq!(dirty.files[0].facts.test_modules[0].name, "changed_tests");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_cache_entry_recomputes() {
        let root = init_repo("rust-fact-corrupt-cache");
        let cache = FactCache {
            root: root.join("cache"),
        };
        fs::write(root.join("src/lib.rs"), "#[cfg(test)]\nmod tests {}\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);

        let inputs = rust_inputs(&root, &BTreeSet::new(), &[".git", "target"]).unwrap();
        let first = scan_inputs(&inputs, Some(&cache)).unwrap();
        assert_eq!(first.parsed_files, 1);
        let content_id = inputs[0].content_id.as_deref().unwrap();
        let cache_path = cache.path_for(content_id).unwrap();
        fs::write(cache_path, "broken json").unwrap();

        let second = scan_inputs(&inputs, Some(&cache)).unwrap();
        assert_eq!(second.cache_hits, 0);
        assert_eq!(second.parsed_files, 1);

        fs::remove_dir_all(root).unwrap();
    }
}
