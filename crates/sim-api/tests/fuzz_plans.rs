//! Property-style fuzz: random but wire-valid command plans, many seeds, full 40-turn runs.
//! `strict_flows` makes a treasury or firm shortfall a hard `Violation`, so a plan that
//! provokes one is a real finding. This test never loosens a check: it asserts that no turn
//! returns a violation and nothing panics, and names the seed and turn when one does.
use sim_api::keys::{InfraKind, LeverKey, SystemKey};
use sim_api::*;
use sim_rng::Rng;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Seeds per run; `FUZZ_SEEDS=500 cargo test -p sim-api --test fuzz_plans` digs deeper.
fn seeds() -> u64 {
    std::env::var("FUZZ_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24)
}

fn random_command(api: &Api, rng: &mut Rng) -> Option<Command> {
    match rng.below(10) {
        0 => Some(Command::SwitchSystem {
            to: SystemKey::ALL[rng.below(3) as usize],
        }),
        1 | 2 => {
            let kind = InfraKind::ALL[rng.below(5) as usize];
            let tiles = api.valid_tiles(kind);
            if tiles.is_empty() {
                return None;
            }
            Some(Command::Place {
                kind,
                tile: tiles[rng.below(tiles.len() as u32) as usize],
            })
        }
        3 => {
            let snap = api.snapshot();
            let plants = &snap.plants;
            if plants.is_empty() {
                return None;
            }
            Some(Command::Remove {
                tile: plants[rng.below(plants.len() as u32) as usize].tile,
            })
        }
        _ => {
            let levers = api.levers();
            let info = &levers[rng.below(levers.len() as u32) as usize];
            let value = if info.key == LeverKey::Bailout {
                PolicyValue::Bool(rng.chance(0.5))
            } else if info.options.is_some() {
                if rng.chance(0.5) {
                    PolicyValue::Commons(CommonsAccessWire::Open)
                } else {
                    PolicyValue::Commons(CommonsAccessWire::Community {
                        cap: rng.next_f64() * 1.0, /* community cap is a multiple of maximum sustainable yield, 0 to 1 */
                    })
                }
            } else {
                // Mix of extremes and interior values: extremes find the edges.
                let t = match rng.below(4) {
                    0 => 0.0,
                    1 => 1.0,
                    _ => rng.next_f64(),
                };
                PolicyValue::Number(info.min + t * (info.max - info.min))
            };
            Some(Command::SetPolicy {
                lever: info.key,
                value,
            })
        }
    }
}

#[test]
fn random_valid_plans_never_violate_a_check_or_panic() {
    let mut findings: Vec<String> = Vec::new();
    let mut turns_played = 0u32;
    for seed in 1..=seeds() {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut api = Api::init(seed).expect("init");
            let mut rng = Rng::new(seed, 0xF022);
            for turn in 1..=40u32 {
                let n = rng.below(5);
                let cmds: Vec<Command> = (0..n)
                    .filter_map(|_| random_command(&api, &mut rng))
                    .collect();
                match api.submit_turn(&cmds) {
                    Ok(_) => {}
                    Err(e) if e.code == "game_over" => return (turn, None),
                    Err(e) => return (turn, Some(format!("{}: {}", e.code, e.message))),
                }
            }
            (40, None)
        }));
        match outcome {
            Ok((t, None)) => turns_played += t,
            Ok((t, Some(e))) => findings.push(format!("seed {seed}, turn {t}: {e}")),
            Err(_) => findings.push(format!("seed {seed}: PANIC")),
        }
    }
    assert!(
        findings.is_empty(),
        "random plans broke a check ({} turns played ok): {findings:#?}",
        turns_played
    );
    assert!(turns_played > 100, "the fuzz did not play any real turns");
}
