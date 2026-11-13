// SPDX-License-Identifier: EUPL-1.2

//! Embeds the generated card faces from `assets/cards/` and the opponent
//! portraits from `assets/portraits/` into the binary.

use std::fmt::Write;
use std::path::Path;

fn main() {
    embed("cards", "CARD_FILES", "card_files.rs");
    embed("portraits", "PORTRAIT_FILES", "portrait_files.rs");
}

/// Writes `OUT_DIR/<out>` with a static `name` of every SVG below
/// `assets/<dir>/` as (relative path, bytes).
fn embed(dir: &str, name: &str, out: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets")
        .join(dir);
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root, &mut files);
    files.sort();
    let mut code = format!("/// SVGs below `assets/{dir}/` as (relative path, bytes).\n");
    let _ = writeln!(code, "pub static {name}: &[(&str, &[u8])] = &[");
    for (rel, abs) in &files {
        let _ = writeln!(code, "    ({rel:?}, include_bytes!({abs:?})),");
    }
    code.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join(out);
    std::fs::write(out, code).unwrap();
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            collect(root, &path, files);
        } else if path.extension().is_some_and(|e| e == "svg") {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let abs = path.to_string_lossy().into_owned();
            files.push((rel, abs));
        }
    }
}
