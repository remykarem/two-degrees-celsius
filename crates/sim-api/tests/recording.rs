//! The recorded demo (`web/src/sim-port/replay/recording.generated.ts`) is a run of the real
//! simulation on `content/demo_script.json`. This fails when the committed file is stale:
//! regenerate with `cargo run -p sim-api --example recording` (`npm run gen:recording`).
use std::path::PathBuf;

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/src/sim-port/replay/recording.generated.ts")
}

#[test]
fn the_committed_recording_is_current() {
    let want = sim_api::recording::generate().unwrap();
    let have = std::fs::read_to_string(path()).expect("recording.generated.ts is missing");
    assert!(
        have == want,
        "web/src/sim-port/replay/recording.generated.ts is stale; run `cargo run -p sim-api --example recording`"
    );
}

#[test]
fn the_script_plays_every_turn_and_the_recording_is_deterministic() {
    let a = sim_api::recording::record().unwrap();
    let b = sim_api::recording::record().unwrap();
    assert_eq!(a, b);
    assert_eq!(a.turns.len(), a.script.len());
    assert_eq!(a.schema_version, sim_api::SCHEMA_VERSION);
    for (t, cmds) in a.turns.iter().zip(&a.script) {
        assert_eq!(t.accepted.len() + t.rejected.len(), cmds.len());
    }
}
