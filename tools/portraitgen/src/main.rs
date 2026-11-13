// SPDX-License-Identifier: EUPL-1.2

//! Writes the generated opponent portraits to `assets/portraits/` (or the
//! directory given as the first argument). Run with `cargo run -p
//! portraitgen`.

use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let out = std::env::args_os().nth(1).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/portraits"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&out)?;
    let files = portraitgen::generate();
    for (path, svg) in &files {
        std::fs::write(out.join(path), svg)?;
    }
    println!("wrote {} files to {}", files.len(), out.display());
    Ok(())
}
