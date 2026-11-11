//! Writes the recorded demo: `cargo run -p sim-api --example recording [file]`
//! (default `web/src/sim-port/replay/recording.generated.ts` at the repository root).
use std::path::PathBuf;

fn main() {
    let path: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../web/src/sim-port/replay/recording.generated.ts"
            ))
        });
    let text = match sim_api::recording::generate() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("recording failed: {e}");
            std::process::exit(1);
        }
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("create directory");
    }
    std::fs::write(&path, &text).expect("write recording");
    println!("wrote {} ({} bytes)", path.display(), text.len());
}
