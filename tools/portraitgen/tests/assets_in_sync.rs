// SPDX-License-Identifier: EUPL-1.2

//! The committed files in `assets/portraits/` must be exactly what the
//! generator produces, and nothing else. After changing the generator, run
//! `cargo run -p portraitgen` and commit the result.

use std::path::Path;

#[test]
fn committed_portraits_match_the_generator() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/portraits");
    let files = portraitgen::generate();
    assert_eq!(files.len(), 24);
    for (path, svg) in &files {
        let committed = std::fs::read_to_string(dir.join(path))
            .unwrap_or_else(|e| panic!("{path}: {e}; run `cargo run -p portraitgen`"));
        assert!(
            committed == *svg,
            "{path} differs from the generator; run `cargo run -p portraitgen`"
        );
        assert!(svg.contains("SPDX-License-Identifier: CC-BY-SA-4.0"));
    }
    let mut on_disk: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    on_disk.sort();
    let mut generated: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();
    generated.sort();
    assert_eq!(on_disk, generated, "stale files in assets/portraits/");
}

#[test]
fn every_portrait_is_distinct() {
    let files = portraitgen::generate();
    let mut bodies: Vec<_> = files.iter().map(|(_, svg)| svg).collect();
    bodies.sort();
    bodies.dedup();
    assert_eq!(bodies.len(), files.len());
}
