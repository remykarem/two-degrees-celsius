//! Save and load: the full `State`, the RNG position, the params hash and the history.
//! The blob is a JSON string; floats round-trip exactly (`serde_json` `float_roundtrip`).
use crate::error::{code, ApiError, ApiResult};
use crate::history::HistoryPoint;
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::{Cents, Owner, State, SystemId, MAX_FIRMS};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveBlob {
    schema_version: u32,
    params_hash: String,
    rng_state: u64,
    rng_inc: u64,
    initial_population: f64,
    clamped_total: i64,
    state: State,
    history: Vec<HistoryPoint>,
}

/// Everything `import` restores.
pub struct Imported {
    pub state: State,
    pub rng: Rng,
    pub initial_population: f64,
    pub clamped_total: Cents,
    pub history: Vec<HistoryPoint>,
}

/// FNV-1a over the canonical JSON of the params: stable across platforms and runs.
pub fn params_hash(p: &Params) -> ApiResult<String> {
    let json = serde_json::to_string(p)
        .map_err(|e| ApiError::new(code::BAD_STATE, format!("params hash: {e}")))?;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in json.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Ok(format!("{h:016x}"))
}

pub fn export(sim: &Sim, history: &[HistoryPoint]) -> ApiResult<String> {
    let (rng_state, rng_inc) = sim.rng.parts();
    let blob = SaveBlob {
        schema_version: crate::SCHEMA_VERSION,
        params_hash: params_hash(&sim.params)?,
        rng_state,
        rng_inc,
        initial_population: sim.initial_population,
        clamped_total: sim.clamped_total.0,
        state: sim.state.clone(),
        history: history.to_vec(),
    };
    serde_json::to_string(&blob)
        .map_err(|e| ApiError::new(code::BAD_STATE, format!("export failed: {e}")))
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::new(code::BAD_STATE, msg)
}

pub fn import(blob: &str, params: &Params) -> ApiResult<Imported> {
    let value: serde_json::Value =
        serde_json::from_str(blob).map_err(|e| bad(format!("save is not valid JSON: {e}")))?;
    let version = value.get("schemaVersion").and_then(|v| v.as_u64());
    if version != Some(u64::from(crate::SCHEMA_VERSION)) {
        return Err(ApiError::new(
            code::SCHEMA_MISMATCH,
            format!(
                "save has schemaVersion {:?}, this build reads {}",
                version,
                crate::SCHEMA_VERSION
            ),
        ));
    }
    let b: SaveBlob =
        serde_json::from_value(value).map_err(|e| bad(format!("save is malformed: {e}")))?;
    if b.params_hash != params_hash(params)? {
        return Err(ApiError::new(
            code::PARAMS_MISMATCH,
            "save was made with different game content (params hash differs)",
        ));
    }
    validate_shape(&b.state, params)?;
    let pop = b.initial_population;
    sim_checks::check_state(&b.state, b.state.tick, SystemId::Commands, pop)
        .map_err(|v| bad(format!("saved state fails its checks: {v}")))?;
    if b.history.len() as u64 > u64::from(b.state.turn) {
        return Err(bad(
            "saved history is longer than the number of turns played",
        ));
    }
    Ok(Imported {
        state: b.state,
        rng: Rng::from_parts(b.rng_state, b.rng_inc),
        initial_population: pop,
        clamped_total: Cents(b.clamped_total),
        history: b.history,
    })
}

/// Indexing the engine does without bounds checks must be safe for any imported state.
fn validate_shape(s: &State, p: &Params) -> ApiResult<()> {
    if s.schema_version != sim_types::SCHEMA_VERSION {
        return Err(ApiError::new(
            code::SCHEMA_MISMATCH,
            "saved state has a different state schema version",
        ));
    }
    let nh = s.households.len();
    if nh != p.initial.cohorts.len() || s.firms.len() != p.initial.firms.len() {
        return Err(bad(
            "saved state has a different number of households or firms",
        ));
    }
    if s.firms.len() != MAX_FIRMS {
        return Err(bad("saved state has the wrong number of firm slots"));
    }
    if s.households
        .iter()
        .any(|h| h.employment.len() != s.firms.len())
    {
        return Err(bad("saved employment table does not match the firm slots"));
    }
    if s.households
        .iter()
        .enumerate()
        .any(|(i, h)| h.id.idx() != i)
        || s.firms.iter().enumerate().any(|(i, f)| f.id.idx() != i)
    {
        return Err(bad("saved ids do not match their positions"));
    }
    let owner_ok = |i: usize| i < nh;
    let firm_ok = s.firms.iter().all(|f| {
        owner_ok(f.founder.idx())
            && match f.owner {
                Owner::Household(h) => owner_ok(h.idx()),
                _ => true,
            }
    });
    if !(firm_ok && owner_ok(s.bank.owner.idx()) && owner_ok(s.housing.owner.idx())) {
        return Err(bad("saved state refers to a household that does not exist"));
    }
    Ok(())
}
