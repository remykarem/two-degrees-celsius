//! Save and load: the full `State`, the RNG position, the params hash and the history.
//! The blob is a JSON string; floats round-trip exactly (`serde_json` `float_roundtrip`).
use crate::error::{code, ApiError, ApiResult};
use crate::history::HistoryPoint;
use crate::ledger::LedgerStore;
use crate::log::WrapperLog;
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::{
    Cents, Command, CommonsAccess, HomeOwner, Owner, State, SystemId, Tile, TileKind, MAX_FIRMS,
};

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
    ledger: LedgerStore,
    log: WrapperLog,
}

/// Everything `import` restores.
pub struct Imported {
    pub state: State,
    pub rng: Rng,
    pub initial_population: f64,
    pub clamped_total: Cents,
    pub history: Vec<HistoryPoint>,
    pub ledger: LedgerStore,
    pub log: WrapperLog,
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

pub fn export(
    sim: &Sim,
    history: &[HistoryPoint],
    ledger: &LedgerStore,
    log: &WrapperLog,
) -> ApiResult<String> {
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
        ledger: ledger.clone(),
        log: log.clone(),
    };
    serde_json::to_string(&blob)
        .map_err(|e| ApiError::new(code::BAD_STATE, format!("export failed: {e}")))
}

/// Largest magnitude, in cents, any integer in a save may have (10^15 cents is 10 trillion
/// dollars, far above anything the game produces). `Cents` arithmetic panics on i64 overflow and
/// a panic traps the wasm module, so a crafted save must be refused before it can reach it.
/// Summing a few thousand such values still fits in an i64. The RNG words are the only
/// legitimately huge integers and are not walked.
pub const MAX_SAVED_INTEGER: i64 = 1_000_000_000_000_000;

fn bound_integers(blob: &serde_json::Value) -> ApiResult<()> {
    fn walk(v: &serde_json::Value, path: &str) -> ApiResult<()> {
        match v {
            serde_json::Value::Number(n) => {
                let too_big = if let Some(i) = n.as_i64() {
                    i.unsigned_abs() > MAX_SAVED_INTEGER as u64
                } else {
                    n.as_u64().is_some()
                };
                if too_big {
                    return Err(bad(format!(
                        "saved number at {path} is out of range (more than {MAX_SAVED_INTEGER})"
                    )));
                }
                Ok(())
            }
            serde_json::Value::Array(a) => a
                .iter()
                .enumerate()
                .try_for_each(|(i, x)| walk(x, &format!("{path}[{i}]"))),
            serde_json::Value::Object(o) => o
                .iter()
                .try_for_each(|(k, x)| walk(x, &format!("{path}.{k}"))),
            _ => Ok(()),
        }
    }
    for key in ["clampedTotal", "state", "history", "ledger", "log"] {
        if let Some(v) = blob.get(key) {
            walk(v, key)?;
        }
    }
    Ok(())
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::new(code::BAD_STATE, msg)
}

pub fn import(blob: &str, params: &Params, detail_turns: u32) -> ApiResult<Imported> {
    let value: serde_json::Value =
        serde_json::from_str(blob).map_err(|e| bad(format!("save is not valid JSON: {e}")))?;
    let version = value.get("schemaVersion").and_then(|v| v.as_u64());
    if version != Some(u64::from(crate::SCHEMA_VERSION)) {
        return Err(ApiError::new(
            code::SCHEMA_MISMATCH,
            format!(
                "save has schemaVersion {:?}, this build reads {} (Town v5 added the transaction ledger and Town v5.2 the policy history and per-month emissions records to the saved game, so older saves cannot be loaded; start a new game)",
                version,
                crate::SCHEMA_VERSION
            ),
        ));
    }
    bound_integers(&value)?;
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
    let mut ledger = b.ledger;
    ledger
        .verify(&b.state)
        .map_err(|e| bad(format!("saved ledger is inconsistent: {e}")))?;
    ledger.set_detail_turns(detail_turns, b.state.turn.saturating_sub(1));
    b.log
        .verify(b.state.tick, b.state.turn)
        .map_err(|e| bad(format!("saved wrapper log is inconsistent: {e}")))?;
    Ok(Imported {
        state: b.state,
        rng: Rng::from_parts(b.rng_state, b.rng_inc),
        initial_population: pop,
        clamped_total: Cents(b.clamped_total),
        history: b.history,
        ledger,
        log: b.log,
    })
}

/// Indexing the engine does without bounds checks must be safe for any imported state: every
/// id and tile in the save must refer to something that exists.
fn validate_shape(s: &State, p: &Params) -> ApiResult<()> {
    if s.schema_version != sim_types::SCHEMA_VERSION {
        return Err(ApiError::new(
            code::SCHEMA_MISMATCH,
            "saved state has a different state schema version",
        ));
    }
    let nh = s.households.len();
    let np = s.persons.len();
    let nf = s.firms.len();
    let people_expected: usize = p.initial.households.iter().map(|h| h.people.len()).sum();
    if nh != p.initial.households.len() || np != people_expected || nf != p.initial.firms.len() {
        return Err(bad(
            "saved state has a different number of people, households or firms",
        ));
    }
    if nf != MAX_FIRMS {
        return Err(bad("saved state has the wrong number of firm slots"));
    }
    let (w, h) = (usize::from(s.map.width), usize::from(s.map.height));
    if w != usize::from(p.initial.map.width)
        || h != usize::from(p.initial.map.height)
        || s.map.cells.len() != w * h
    {
        return Err(bad("saved map has the wrong size"));
    }
    if s.persons.iter().enumerate().any(|(i, x)| x.id.idx() != i)
        || s.households
            .iter()
            .enumerate()
            .any(|(i, x)| x.id.idx() != i)
        || s.firms.iter().enumerate().any(|(i, x)| x.id.idx() != i)
    {
        return Err(bad("saved ids do not match their positions"));
    }
    let hh = |i: usize| i < nh;
    let tile_ok = |t: Tile| s.map.in_bounds(t);
    let home_exists = |id: u32| s.homes.iter().any(|x| x.id.0 == id);
    let plant_exists = |id: u32| s.plants.iter().any(|x| x.id.0 == id);
    let people_ok = s.persons.iter().all(|x| {
        hh(x.household.idx())
            && x.job.is_none_or(|f| f.idx() < nf)
            && x.staff_of.is_none_or(|o| hh(o.idx()))
    });
    let estate_ok = s.estate.as_ref().is_none_or(|e| {
        hh(e.household.idx())
            && home_exists(e.home.0)
            && tile_ok(e.hangar)
            && e.grounds.iter().chain(&e.runway).all(|t| tile_ok(*t))
            && e.pilot.is_none_or(|x| x.idx() < np)
            && e.jet.owner.idx() < nh
    });
    let households_ok = s.households.iter().all(|x| {
        x.members.iter().all(|m| m.idx() < np) && x.home.is_none_or(|id| home_exists(id.0))
    });
    let homes_ok = s.homes.iter().all(|x| {
        tile_ok(x.tile)
            && x.household.is_none_or(|i| hh(i.idx()))
            && match x.owner {
                HomeOwner::Household(o) => hh(o.idx()),
                HomeOwner::Public => true,
            }
    }) && s.homes.iter().all(|x| x.id.0 < s.next_home_id);
    let plants_ok = s
        .plants
        .iter()
        .all(|x| tile_ok(x.tile) && x.id.0 < s.next_plant_id);
    let firms_ok = s.firms.iter().all(|f| {
        tile_ok(f.tile)
            && hh(f.founder.idx())
            && match f.owner {
                Owner::Household(o) => hh(o.idx()),
                _ => true,
            }
    });
    let cells_ok = s.map.cells.iter().all(|c| match (c.kind, c.id) {
        (TileKind::Home | TileKind::PublicHousing, Some(id)) => home_exists(id),
        (TileKind::Firm, Some(id)) => (id as usize) < nf,
        (TileKind::FossilPlant | TileKind::RenewablePlant, Some(id)) => plant_exists(id),
        _ => true,
    });
    let pending_ok = s.pending.iter().all(|c| match c.command {
        Command::Place { tile, .. } | Command::Remove { tile } => tile_ok(tile),
        _ => true,
    }) && s.gov.pending_build.iter().all(|(_, t)| tile_ok(*t));
    let owners_ok = hh(s.bank.owner.idx())
        && match s.nature.commons_access {
            CommonsAccess::Enclosed(o) => hh(o.idx()),
            _ => true,
        };
    if !(people_ok
        && estate_ok
        && households_ok
        && homes_ok
        && plants_ok
        && firms_ok
        && cells_ok
        && pending_ok
        && owners_ok)
    {
        return Err(bad(
            "saved state refers to a person, household, home, plant, firm or tile that does not exist",
        ));
    }
    Ok(())
}
