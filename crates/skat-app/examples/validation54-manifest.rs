// SPDX-License-Identifier: EUPL-1.2

//! Machine input for the private kit builder, never a learner preview.
//! Run only on the exact clean source of the packaged executable.
use skat_app::practice_catalogue::{RESERVED_CASES, ValidationSet};

fn main() {
    let cases: Vec<_> = RESERVED_CASES.into_iter().map(|case| {
        serde_json::json!({"id": case.id(), "version": case.version(),
            "set": if ValidationSet::Initial.cases().contains(&case) { "Initial" } else { "Delayed" },
            "position": case.position().expect("legal reserved fixture")})
    }).collect();
    println!("{}", serde_json::to_string(&cases).unwrap());
}
