//! The generated TS contract (`web/src/contract/generated.ts`) must match the Rust wire types.
//! Regenerate with `cargo run -p sim-api --features ts --example contract_ts` (`npm run gen:contract`).
#![cfg(feature = "ts")]

use std::collections::BTreeSet;

const GENERATED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../web/src/contract/generated.ts"
);

#[test]
fn committed_contract_is_up_to_date() {
    let want = sim_api::contract_ts::generate();
    let have = std::fs::read_to_string(GENERATED).expect("web/src/contract/generated.ts exists");
    assert!(
        want == have,
        "web/src/contract/generated.ts is stale: run `cd web && npm run gen:contract`"
    );
}

/// Every wire type that derives `TS` is in the generator's list (a missing one would leave a
/// dangling name in the generated file), and every name used there is defined there.
#[test]
fn every_ts_type_is_listed_and_every_name_resolves() {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut derives = 0;
    for entry in std::fs::read_dir(src).expect("src dir") {
        let path = entry.expect("entry").path();
        if path.file_name().is_some_and(|n| n == "contract_ts.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read");
        derives += text.matches("derive(ts_rs::TS))]").count();
    }
    assert_eq!(
        derives,
        sim_api::contract_ts::decls().len(),
        "a type derives TS but is not listed in contract_ts.rs (or the reverse)"
    );

    let out = sim_api::contract_ts::generate();
    let defined: BTreeSet<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("export type "))
        .filter_map(|l| l.split([' ', '<']).next())
        .collect();
    let builtin = [
        "Array", "Record", "Partial", "Omit", "Pick", "Exclude", "Extract", "Date",
    ];
    // Strip comments, then check each capitalised identifier outside string literals.
    let mut code = String::new();
    let mut in_comment = false;
    for line in out.lines() {
        let t = line.trim_start();
        if in_comment {
            if t.contains("*/") {
                in_comment = false;
            }
            continue;
        }
        if t.starts_with("/**") && !t.contains("*/") {
            in_comment = true;
            continue;
        }
        if t.starts_with("//") || t.starts_with("/*") {
            continue;
        }
        code.push_str(line);
        code.push('\n');
    }
    let mut in_str = false;
    let mut word = String::new();
    let mut unknown = BTreeSet::new();
    for ch in code.chars().chain(std::iter::once(' ')) {
        if ch == '"' {
            in_str = !in_str;
            word.clear();
            continue;
        }
        if in_str {
            continue;
        }
        if ch.is_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            if word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && !defined.contains(word.as_str())
                && !builtin.contains(&word.as_str())
            {
                unknown.insert(word.clone());
            }
            word.clear();
        }
    }
    assert!(
        unknown.is_empty(),
        "names used but not defined: {unknown:?}"
    );
}
