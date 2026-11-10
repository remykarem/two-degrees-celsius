//! Sample wire output for the web side (`docs/fixtures/`). Pure: returns the file contents;
//! `examples/fixtures.rs` writes them and a test checks the committed copies are current.
use crate::api::Api;
use crate::command::{Command, PolicyValue, Tile};
use crate::error::{ApiError, ApiResult};
use crate::keys::{InfraKind, LeverKey};
use serde::Serialize;
use std::collections::BTreeMap;

/// Seed of the fixture game.
pub const FIXTURE_SEED: u64 = 1;

/// The commands of `turn-result.json`: a policy, a road, a power plant, and one the Market
/// system refuses.
pub fn fixture_commands() -> Vec<Command> {
    vec![
        Command::SetPolicy {
            lever: LeverKey::CarbonPrice,
            value: PolicyValue::Number(50.0),
        },
        Command::Place {
            kind: InfraKind::Road,
            tile: Tile { x: 4, y: 6 },
        },
        Command::Place {
            kind: InfraKind::RenewablePlant,
            tile: Tile { x: 10, y: 5 },
        },
        Command::SetPolicy {
            lever: LeverKey::CampaignCap,
            value: PolicyValue::Number(0.5),
        },
    ]
}

fn pretty<T: Serialize>(v: &T) -> ApiResult<String> {
    let mut s = serde_json::to_string_pretty(v)
        .map_err(|e| ApiError::new(crate::error::code::BAD_STATE, e.to_string()))?;
    s.push('\n');
    Ok(s)
}

/// `validTiles(kind)` for every buildable kind on the starting map, keyed by kind.
fn valid_tiles_sample(api: &Api) -> BTreeMap<&'static str, Vec<Tile>> {
    InfraKind::ALL
        .iter()
        .map(|k| (k.as_str(), api.valid_tiles(*k)))
        .collect()
}

/// `(file name, contents)` for every fixture.
pub fn generate() -> ApiResult<Vec<(String, String)>> {
    let mut api = Api::init(FIXTURE_SEED)?;
    let snapshot = pretty(&api.snapshot())?;
    let levers = pretty(&api.levers())?;
    let actions = pretty(&api.actions())?;
    let tiles = pretty(&valid_tiles_sample(&api))?;
    let turn = api.submit_turn(&fixture_commands())?;
    // Contract v1.1: the same first turn, played live on a second game.
    let mut live = Api::init(FIXTURE_SEED)?;
    let turn_start = live.begin_turn(&fixture_commands())?;
    let month_result = live.advance_month()?;
    Ok(vec![
        ("snapshot-turn1.json".to_string(), snapshot),
        ("turn-result.json".to_string(), pretty(&turn)?),
        ("turn-start.json".to_string(), pretty(&turn_start)?),
        ("month-result.json".to_string(), pretty(&month_result)?),
        ("levers.json".to_string(), levers),
        ("actions.json".to_string(), actions),
        ("tiles.json".to_string(), tiles),
    ])
}
