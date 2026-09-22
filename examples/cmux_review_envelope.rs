#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/finding.rs"]
mod finding;
#[path = "../src/cmux_review.rs"]
mod cmux_review;

use cmux_review::{
    MAX_CMUX_REVIEW_RECEIPT_BYTES, parse_cmux_review_receipt, project_cmux_review_receipt,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("cmux-review-envelope: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_CMUX_REVIEW_RECEIPT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_CMUX_REVIEW_RECEIPT_BYTES {
        return Err(format!(
            "cmux review receipt exceeds the {MAX_CMUX_REVIEW_RECEIPT_BYTES}-byte limit"
        )
        .into());
    }

    let receipt = parse_cmux_review_receipt(&bytes)?;
    let envelope = project_cmux_review_receipt(&receipt)?;
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}
