#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/agent_experience_episode.rs"]
mod agent_experience_episode;

use agent_experience_episode::{
    MAX_AGENT_EXPERIENCE_BATCH_BYTES, parse_agent_experience_batch,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("agent-experience-episodes: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_AGENT_EXPERIENCE_BATCH_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_AGENT_EXPERIENCE_BATCH_BYTES {
        return Err(format!(
            "agent experience batch exceeds the {MAX_AGENT_EXPERIENCE_BATCH_BYTES}-byte limit"
        )
        .into());
    }

    let batch = parse_agent_experience_batch(&bytes)?;
    println!("{}", serde_json::to_string_pretty(&batch)?);
    Ok(())
}
