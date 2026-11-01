// SPDX-License-Identifier: EUPL-1.2

//! The committed files in `assets/cards/` must be exactly what the
//! generator produces. After changing the generator, run
//! `cargo run -p cardgen` and commit the result.

use std::path::Path;

#[test]
fn committed_cards_match_the_generator() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/cards");
    let files = cardgen::generate();
    assert_eq!(files.len(), 3 * (3 * 32 + 1));
    for (path, svg) in &files {
        let committed = std::fs::read_to_string(dir.join(path))
            .unwrap_or_else(|e| panic!("{path}: {e}; run `cargo run -p cardgen`"));
        assert!(
            committed == *svg,
            "{path} differs from the generator; run `cargo run -p cardgen`"
        );
        assert!(svg.contains("SPDX-License-Identifier: CC-BY-SA-4.0"));
    }
}
