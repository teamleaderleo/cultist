#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/cmux_review.rs"]
mod cmux_review;
#[path = "../src/cmux_review_applicability.rs"]
mod cmux_review_applicability;
#[path = "../src/finding.rs"]
mod finding;

use cmux_review::MAX_CMUX_REVIEW_RECEIPT_BYTES;
use cmux_review_applicability::{
    CmuxReviewApplicabilityRequest, project_cmux_review_for_context,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("cmux-review-applicability: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_CMUX_REVIEW_RECEIPT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_CMUX_REVIEW_RECEIPT_BYTES {
        return Err("cmux review applicability request exceeds the input limit".into());
    }

    let request: CmuxReviewApplicabilityRequest = serde_json::from_slice(&bytes)?;
    let projection = project_cmux_review_for_context(&request)?;
    println!("{}", serde_json::to_string_pretty(&projection)?);
    Ok(())
}
