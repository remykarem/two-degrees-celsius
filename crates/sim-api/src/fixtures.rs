//! Sample wire output for the web side (`docs/fixtures/`). Pure: returns the file contents;
//! `examples/fixtures.rs` writes them and a test checks the committed copies are current.
use crate::api::Api;
use crate::command::{Command, PolicyValue};
use crate::error::{ApiError, ApiResult};
use crate::keys::{InfraKind, LeverKey};
use serde::Serialize;

/// Seed of the fixture game.
pub const FIXTURE_SEED: u64 = 1;

/// The commands of `turn-result.json`: a policy, a build and one the Market system refuses.
pub fn fixture_commands() -> Vec<Command> {
    vec![
        Command::SetPolicy {
            lever: LeverKey::CarbonPrice,
            value: PolicyValue::Number(50.0),
        },
        Command::Place {
            kind: InfraKind::RenewablePlant,
            count: 1,
            tile: None,
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

/// `(file name, contents)` for every fixture.
pub fn generate() -> ApiResult<Vec<(String, String)>> {
    let mut api = Api::init(FIXTURE_SEED)?;
    let snapshot = pretty(&api.snapshot())?;
    let levers = pretty(&api.levers())?;
    let actions = pretty(&api.actions())?;
    let turn = api.submit_turn(&fixture_commands())?;
    Ok(vec![
        ("snapshot-turn1.json".to_string(), snapshot),
        ("turn-result.json".to_string(), pretty(&turn)?),
        ("levers.json".to_string(), levers),
        ("actions.json".to_string(), actions),
    ])
}
