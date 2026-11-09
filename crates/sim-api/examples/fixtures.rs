//! Writes the wire fixtures: `cargo run -p sim-api --example fixtures [dir]`
//! (default `docs/fixtures` at the repository root).
use std::path::PathBuf;

fn main() {
    let dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/fixtures"))
        });
    let files = match sim_api::fixtures::generate() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("fixtures failed: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot create {}: {e}", dir.display());
        std::process::exit(1);
    }
    for (name, body) in files {
        let path = dir.join(&name);
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("cannot write {}: {e}", path.display());
            std::process::exit(1);
        }
        println!("wrote {}", path.display());
    }
}
