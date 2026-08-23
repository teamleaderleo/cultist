use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[allow(dead_code)]
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[allow(dead_code)]
#[path = "../src/evidence_planner.rs"]
mod evidence_planner;
#[allow(dead_code)]
#[path = "../src/justification.rs"]
mod justification;
#[allow(dead_code)]
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[allow(dead_code)]
#[path = "../src/observation_probe_bridge.rs"]
mod observation_probe_bridge;
#[allow(dead_code)]
#[path = "../src/rust_edit_class_source.rs"]
mod rust_edit_class_source;

use rust_edit_class_source::{RustEditClassSubject, collect_rust_edit_class_source};

const USAGE: &str = "usage: rust_edit_class_observation REPO_ROOT CURRENT_REPOSITORY SUBJECT_REPOSITORY REVISION FILE";

fn main() {
    if let Err(error) = run() {
        eprintln!("rust-edit-class-observation: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or(USAGE)?);
    let current_repository = args.next().ok_or(USAGE)?;
    let subject_repository = args.next().ok_or(USAGE)?;
    let revision = args.next().ok_or(USAGE)?;
    let path = args.next().ok_or(USAGE)?;
    if args.next().is_some() {
        return Err(USAGE.into());
    }

    let result = collect_rust_edit_class_source(
        Path::new(&root),
        &current_repository,
        &RustEditClassSubject {
            repository: subject_repository,
            revision,
            path,
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
