use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[allow(dead_code)]
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[allow(dead_code)]
#[path = "../src/refinement_candidate_readiness.rs"]
mod refinement_candidate_readiness;
#[allow(dead_code)]
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;
#[allow(dead_code)]
#[path = "../src/refinement_observation_requirement.rs"]
mod refinement_observation_requirement;

use refinement_candidate_readiness::{
    MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES, evaluate_refinement_candidate_readiness,
    parse_refinement_candidate_readiness_request,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("refinement-candidate-readiness: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES {
        return Err(format!(
            "refinement candidate readiness request exceeds the {MAX_REFINEMENT_CANDIDATE_READINESS_REQUEST_BYTES}-byte limit"
        )
        .into());
    }

    let request = parse_refinement_candidate_readiness_request(&bytes)?;
    let evaluation = evaluate_refinement_candidate_readiness(&request)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}
