//! Writes the generated TS contract: `cargo run -p sim-api --features ts --example contract_ts [file]`
//! (default `web/src/contract/generated.ts`).
use std::path::PathBuf;

fn main() {
    let path: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../web/src/contract/generated.ts"
            ))
        });
    if let Err(e) = std::fs::write(&path, sim_api::contract_ts::generate()) {
        eprintln!("cannot write {}: {e}", path.display());
        std::process::exit(1);
    }
    println!("wrote {}", path.display());
}
