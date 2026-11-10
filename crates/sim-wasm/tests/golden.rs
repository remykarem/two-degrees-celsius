//! Golden run through the exact strings the browser receives (`WasmSim`, native build).
//! `tests/golden/plan.json` is a scripted game; `tests/golden/seed7.json` records, per turn, an
//! FNV-1a 64 hash of the whole JSON reply plus a few headline numbers. The web test
//! (`web/tests/wasm-golden.test.ts`) replays the same plan in the wasm build and must match.
//! Refresh with `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`.
use serde_json::{json, Value};
use sim_wasm::WasmSim;
use std::path::PathBuf;

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

fn pick(v: &Value, path: &[&str]) -> Value {
    path.iter().fold(v.clone(), |acc, k| acc[*k].clone())
}

fn run(plan: &Value) -> Value {
    let mut sim = WasmSim::new();
    let init = sim.init(plan["seed"].as_f64().unwrap(), None);
    let mut out = json!({
        "init": fnv(&init),
        "levers": fnv(&sim.levers()),
        "actions": fnv(&sim.actions()),
    });
    let mut turns = Vec::new();
    for (i, cmds) in plan["turns"].as_array().unwrap().iter().enumerate() {
        let turn = i as u64 + 1;
        if let Some(id) = plan["cancelBeforeTurn"][turn.to_string()].as_u64() {
            out[format!("cancel{turn}")] = json!(fnv(&sim.cancel_pending(id as u32)));
        }
        let reply = sim.submit_turn(&cmds.to_string());
        let v: Value = serde_json::from_str(&reply).unwrap();
        let snap = pick(&v, &["value", "snapshot"]);
        turns.push(json!({
            "turn": turn,
            "ok": v["ok"],
            "fnv": fnv(&reply),
            "gdpRealAnnual": pick(&snap, &["indicators", "gdpRealAnnual"]),
            "legitimacy": pick(&snap, &["government", "legitimacy"]),
            "temperature": pick(&snap, &["indicators", "temperature"]),
        }));
        if turn == plan["exportAfterTurn"].as_u64().unwrap() {
            let blob = sim.export_state();
            out["export"] = json!(fnv(&blob));
            let v: Value = serde_json::from_str(&blob).unwrap();
            let imported = sim.import_state(v["value"].as_str().unwrap());
            out["import"] = json!(fnv(&imported));
        }
    }
    out["turns"] = Value::Array(turns);
    out["finalSnapshot"] = json!(fnv(&sim.snapshot()));
    out
}

#[test]
fn golden_run_matches_the_committed_file() {
    let plan: Value =
        serde_json::from_str(&std::fs::read_to_string(dir().join("plan.json")).unwrap()).unwrap();
    let got = run(&plan);
    assert_eq!(got, run(&plan), "two native runs differ");
    let path = dir().join("seed7.json");
    let mut text = serde_json::to_string_pretty(&got).unwrap();
    text.push('\n');
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, &text).unwrap();
    }
    let want = std::fs::read_to_string(&path).expect("run with UPDATE_GOLDEN=1 to create it");
    assert!(
        want == text,
        "golden mismatch: run with UPDATE_GOLDEN=1 if the change is intended"
    );
}

#[test]
fn bad_input_gives_error_results_not_panics() {
    let mut sim = WasmSim::new();
    assert!(sim.snapshot().contains(r#""ok":false"#));
    assert!(sim.init(-1.0, None).contains("bad_request"));
    assert!(sim.init(f64::NAN, None).contains("bad_request"));
    assert!(sim
        .init(1.0, Some("{not json".into()))
        .contains(r#""ok":false"#));
    assert!(sim.init(1.0, None).contains(r#""ok":true"#));
    assert!(sim.submit_turn("nonsense").contains("bad_request"));
    assert!(sim.preview("{}").contains("bad_request"));
    assert!(sim.cancel_pending(999).contains("no_such_pending"));
    assert!(sim.import_state("garbage").contains(r#""ok":false"#));
    let big = r#"[{"type":"place","kind":"fossil_plant","tile":{"x":2147483647,"y":-5}}]"#;
    let reply = sim.submit_turn(big);
    assert!(reply.contains(r#""rejected""#) && reply.contains("tile_invalid"));
    // A place command needs a tile.
    assert!(sim
        .submit_turn(r#"[{"type":"place","kind":"road"}]"#)
        .contains("bad_request"));
}

#[test]
fn valid_tiles_helper_returns_tiles_or_an_error() {
    let mut sim = WasmSim::new();
    assert!(sim.valid_tiles("road").contains(r#""ok":false"#));
    sim.init(7.0, None);
    for kind in ["public_housing", "fossil_plant", "renewable_plant", "road"] {
        let v: Value = serde_json::from_str(&sim.valid_tiles(kind)).unwrap();
        assert_eq!(v["ok"], true, "{kind}");
        let tiles = v["value"].as_array().unwrap();
        assert!(tiles.len() > 50 && tiles[0]["x"].is_number(), "{kind}");
    }
    assert!(sim.valid_tiles("castle").contains("bad_request"));
    assert!(sim.valid_tiles("").contains("bad_request"));
}

/// The same plan played with `beginTurn` / `advanceMonth` x3 / `finishTurn`. Each reply is
/// hashed; `tests/golden/seed7-live.json` records them (the web test replays it in wasm).
/// The `finishTurn` reply must hash exactly like the `submitTurn` reply of `seed7.json`.
fn run_live(plan: &Value) -> Value {
    let mut sim = WasmSim::new();
    sim.init(plan["seed"].as_f64().unwrap(), None);
    let mut turns = Vec::new();
    let mut midturn_export = Value::Null;
    for (i, cmds) in plan["turns"].as_array().unwrap().iter().enumerate() {
        let turn = i as u64 + 1;
        if let Some(id) = plan["cancelBeforeTurn"][turn.to_string()].as_u64() {
            sim.cancel_pending(id as u32);
        }
        let begin = sim.begin_turn(&cmds.to_string());
        if turn == 1 {
            // Saving inside a turn is refused.
            let e: Value = serde_json::from_str(&sim.export_state()).unwrap();
            midturn_export = e["error"]["code"].clone();
        }
        let months: Vec<String> = (0..3).map(|_| fnv(&sim.advance_month())).collect();
        let finish = sim.finish_turn();
        turns.push(json!({
            "turn": turn,
            "begin": fnv(&begin),
            "months": months,
            "finish": fnv(&finish),
        }));
        if turn == plan["exportAfterTurn"].as_u64().unwrap() {
            let blob = sim.export_state();
            let v: Value = serde_json::from_str(&blob).unwrap();
            sim.import_state(v["value"].as_str().unwrap());
        }
    }
    json!({
        "midTurnExportError": midturn_export,
        "turns": turns,
        "finalSnapshot": fnv(&sim.snapshot()),
    })
}

#[test]
fn live_turn_golden_matches_and_finish_equals_submit() {
    let plan: Value =
        serde_json::from_str(&std::fs::read_to_string(dir().join("plan.json")).unwrap()).unwrap();
    let got = run_live(&plan);
    assert_eq!(got["midTurnExportError"], "turn_state");
    // finishTurn replies hash like submitTurn replies, and the final snapshot is the same.
    let whole: Value =
        serde_json::from_str(&std::fs::read_to_string(dir().join("seed7.json")).unwrap()).unwrap();
    for (l, w) in got["turns"]
        .as_array()
        .unwrap()
        .iter()
        .zip(whole["turns"].as_array().unwrap())
    {
        assert_eq!(l["finish"], w["fnv"], "turn {}", l["turn"]);
    }
    assert_eq!(got["finalSnapshot"], whole["finalSnapshot"]);
    let path = dir().join("seed7-live.json");
    let mut text = serde_json::to_string_pretty(&got).unwrap();
    text.push('\n');
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, &text).unwrap();
    }
    let want = std::fs::read_to_string(&path).expect("run with UPDATE_GOLDEN=1 to create it");
    assert!(
        want == text,
        "live golden mismatch: UPDATE_GOLDEN=1 if intended"
    );
}

#[test]
fn live_turn_state_errors_through_the_wasm_strings() {
    let mut sim = WasmSim::new();
    assert!(sim.begin_turn("[]").contains("bad_request")); // not started
    sim.init(7.0, None);
    assert!(sim.advance_month().contains("turn_state"));
    assert!(sim.finish_turn().contains("turn_state"));
    assert!(sim.begin_turn("nonsense").contains("bad_request"));
    assert!(sim.begin_turn("[]").contains(r#""ok":true"#));
    assert!(sim.begin_turn("[]").contains("turn_state"));
    assert!(sim.submit_turn("[]").contains("turn_state"));
    assert!(sim.export_state().contains("turn_state"));
    assert!(sim
        .snapshot()
        .contains(r#""turnPhase":{"open":true,"monthsDone":0}"#));
}
