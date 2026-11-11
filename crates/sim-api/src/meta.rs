//! Static metadata: `LeverInfo` and `ActionInfo`. Costs and lead times come from
//! `params.turn.levers` (one source of truth); labels, ranges, arrows and availability come
//! from `content/levers.json`.
use crate::command::{numeric_change, Command, CommonsAccessWire, PolicyValue, COMMUNITY_CAP_MAX};
use crate::error::{code, ApiError, ApiResult};
use crate::keys::{BlockReason, InfraKind, LeverKey, MetricKey, SystemKey};
use serde::{Deserialize, Serialize};
use sim_params::Params;
use sim_types as st;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeverUnit {
    Usd,
    UsdPerMonth,
    UsdPerTonne,
    Fraction,
    TonnesPerMonth,
    Bool,
    Enum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arrow {
    pub metric: MetricKey,
    pub direction: Direction,
}

/// `cost = base x max(|change| / unit, minSize) x (1 + opposition)`; a hint only.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostModel {
    pub base: f64,
    pub unit: f64,
    pub min_size: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverInfo {
    pub key: LeverKey,
    pub label: String,
    /// Contract v1.6: the group the Policies tab lists the lever under (from `levers.json`).
    pub group: String,
    pub unit: LeverUnit,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub base_cost_pc: f64,
    pub lead_time: u32,
    pub cost_model: CostModel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero_means_none: Option<bool>,
    pub lobbied: bool,
    pub arrows: Vec<Arrow>,
    pub available_in: Vec<SystemKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Enum options that exist only in some systems (from `content/levers.json`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_available_in: Option<BTreeMap<String, Vec<SystemKey>>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionInfo {
    /// `switch_system`, `place:<kind>` or `remove:<kind>`.
    pub key: String,
    pub label: String,
    pub base_cost_pc: f64,
    pub lead_time: u32,
    pub cost_model: CostModel,
    /// Estimated dollars charged on activation; 0 if none. `preview` is the authority.
    pub money_cost: f64,
    pub available_in: Vec<SystemKey>,
    /// True for `place:*` and `remove:*`: the command carries a tile.
    pub needs_tile: bool,
    /// What the tile must be (from `content/levers.json`); absent for `switch_system`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_rule: Option<String>,
}

/// Game calendar (contract section 9, item 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCalendar {
    pub start_year: i32,
    pub first_turn: u32,
    pub last_turn: u32,
    pub turns_per_year: u32,
    /// Contract v1.8: monthly ticks in a turn (`params.run.ticks_per_turn`).
    pub ticks_per_turn: u32,
}

// ---- the content file ----

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GameFile {
    start_year: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LeverEntry {
    label: String,
    group: String,
    unit: LeverUnit,
    min: f64,
    max: f64,
    step: f64,
    #[serde(default)]
    zero_means_none: bool,
    lobbied: bool,
    arrows: Vec<Arrow>,
    available_in: Vec<SystemKey>,
    #[serde(default)]
    options: Option<Vec<String>>,
    #[serde(default)]
    option_available_in: Option<BTreeMap<String, Vec<SystemKey>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ActionEntry {
    label: String,
    available_in: Vec<SystemKey>,
    #[serde(default)]
    tile_rule: Option<String>,
}

/// `ledger` block of levers.json (contract v1.4).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerFile {
    /// Turns kept in full detail (older periods are rolled up to monthly summaries).
    detail_turns: u32,
}

/// `energy` block of levers.json (contract v1.4).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnergyFile {
    unit_label: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeversFile {
    game: GameFile,
    ledger: LedgerFile,
    energy: EnergyFile,
    levers: BTreeMap<LeverKey, LeverEntry>,
    actions: BTreeMap<String, ActionEntry>,
}

/// Why the wire layer refuses a command before the engine sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct Rejection {
    pub reason: BlockReason,
    pub why: String,
}

impl Rejection {
    pub fn invalid(why: String) -> Rejection {
        Rejection {
            reason: BlockReason::InvalidCommand,
            why,
        }
    }
    pub fn tile(t: crate::command::Tile) -> Rejection {
        Rejection {
            reason: BlockReason::TileInvalid,
            why: format!("tile ({}, {}) is not on the map", t.x, t.y),
        }
    }
}

/// All static metadata, built once per `Api`.
#[derive(Clone, Debug)]
pub struct Meta {
    pub levers: Vec<LeverInfo>,
    pub actions: Vec<ActionInfo>,
    pub game: GameCalendar,
    /// Turns of full ledger detail (at least 2: the trailing quarter can span two turns).
    pub ledger_detail_turns: u32,
    /// Label of the energy unit, shown per month ("MWh").
    pub energy_unit: String,
}

fn bad(msg: String) -> ApiError {
    ApiError::new(code::BAD_METADATA, msg)
}

fn action_param_key(key: &str) -> Option<String> {
    match key {
        "switch_system" => Some("switch_system".into()),
        _ => key.split_once(':').map(|(a, k)| format!("{a}_{k}")),
    }
}

/// Keys of every action, in display order.
pub fn action_keys() -> Vec<String> {
    let mut v = vec!["switch_system".to_string()];
    for verb in ["place", "remove"] {
        for k in InfraKind::ALL {
            v.push(format!("{verb}:{}", k.as_str()));
        }
    }
    v
}

impl Meta {
    pub fn load(levers_json: &str, params: &Params) -> ApiResult<Meta> {
        let file: LeversFile =
            serde_json::from_str(levers_json).map_err(|e| bad(format!("levers.json: {e}")))?;
        let mut levers = Vec::new();
        for key in LeverKey::ALL {
            let e = file
                .levers
                .get(&key)
                .ok_or_else(|| bad(format!("levers.json is missing '{}'", key.as_str())))?;
            let cfg =
                params.turn.levers.get(key.as_str()).ok_or_else(|| {
                    bad(format!("params turn.levers is missing '{}'", key.as_str()))
                })?;
            if !(e.min.is_finite() && e.max.is_finite() && e.step.is_finite() && e.min <= e.max) {
                return Err(bad(format!(
                    "levers.json: bad range for '{}'",
                    key.as_str()
                )));
            }
            if e.available_in.is_empty() {
                return Err(bad(format!(
                    "levers.json: '{}' is available nowhere",
                    key.as_str()
                )));
            }
            let is_enum = e.unit == LeverUnit::Enum;
            if is_enum != e.options.is_some() {
                return Err(bad(format!(
                    "levers.json: '{}' must have options exactly when its unit is enum",
                    key.as_str()
                )));
            }
            levers.push(LeverInfo {
                key,
                label: e.label.clone(),
                group: e.group.clone(),
                unit: e.unit,
                min: e.min,
                max: e.max,
                step: e.step,
                base_cost_pc: cfg.base_cost,
                lead_time: cfg.lead_time,
                cost_model: CostModel {
                    base: cfg.base_cost,
                    unit: cfg.unit,
                    min_size: cfg.min_size,
                },
                zero_means_none: e.zero_means_none.then_some(true),
                lobbied: e.lobbied,
                arrows: e.arrows.clone(),
                available_in: e.available_in.clone(),
                options: e.options.clone(),
                option_available_in: e.option_available_in.clone(),
            });
        }
        let mut actions = Vec::new();
        for key in action_keys() {
            let e = file
                .actions
                .get(&key)
                .ok_or_else(|| bad(format!("levers.json is missing action '{key}'")))?;
            let pk = action_param_key(&key).unwrap_or_default();
            let needs_tile = key.contains(':');
            let cfg = params
                .turn
                .levers
                .get(&pk)
                .ok_or_else(|| bad(format!("params turn.levers is missing '{pk}'")))?;
            let money = match key.as_str() {
                "place:public_housing" => params.government.public_housing_cost_usd,
                "place:fossil_plant" | "place:renewable_plant" => params.government.plant_cost_usd,
                "place:road" => params.government.road_cost_usd,
                "place:fire_station" => params.weather.fire_station_cost_usd,
                _ => 0.0,
            };
            actions.push(ActionInfo {
                key,
                label: e.label.clone(),
                base_cost_pc: cfg.base_cost,
                lead_time: cfg.lead_time,
                cost_model: CostModel {
                    base: cfg.base_cost,
                    unit: cfg.unit,
                    min_size: cfg.min_size,
                },
                money_cost: money,
                available_in: e.available_in.clone(),
                needs_tile,
                tile_rule: e.tile_rule.clone(),
            });
        }
        if file.actions.len() != actions.len() {
            return Err(bad("levers.json has an unknown action".into()));
        }
        let tpt = params.run.ticks_per_turn;
        if tpt != 3 {
            return Err(ApiError::new(
                code::UNSUPPORTED_PARAMS,
                format!("the wire format has exactly 3 months per turn, params say {tpt}"),
            ));
        }
        if !(2..=200).contains(&file.ledger.detail_turns) {
            return Err(bad(
                "levers.json: ledger.detail_turns must be between 2 and 200".into(),
            ));
        }
        if file.energy.unit_label.trim().is_empty() {
            return Err(bad("levers.json: energy.unit_label is empty".into()));
        }
        Ok(Meta {
            levers,
            actions,
            ledger_detail_turns: file.ledger.detail_turns,
            energy_unit: file.energy.unit_label.clone(),
            game: GameCalendar {
                start_year: file.game.start_year,
                first_turn: 1,
                last_turn: params.run.turns,
                turns_per_year: 12 / tpt,
                ticks_per_turn: tpt,
            },
        })
    }

    pub fn lever(&self, key: LeverKey) -> &LeverInfo {
        // `load` guarantees all 19 levers are present, in `LeverKey::ALL` order.
        self.levers
            .iter()
            .find(|l| l.key == key)
            .unwrap_or(&self.levers[0])
    }

    /// Validate a wire command (type, finiteness, range) and convert it to the sim command.
    /// The error is a human-readable reason; the caller maps it to `invalid_command`.
    pub fn to_sim(&self, c: &Command) -> Result<st::Command, Rejection> {
        match *c {
            Command::SwitchSystem { to } => Ok(st::Command::SwitchSystem(to.to_sim())),
            Command::Place { kind, tile } => match tile.to_sim() {
                Some(tile) => Ok(st::Command::Place {
                    kind: kind.to_sim(),
                    tile,
                }),
                None => Err(Rejection::tile(tile)),
            },
            Command::Remove { tile } => match tile.to_sim() {
                Some(tile) => Ok(st::Command::Remove { tile }),
                None => Err(Rejection::tile(tile)),
            },
            Command::SetPolicy { lever, value } => {
                let info = self.lever(lever);
                let name = lever.as_str();
                match (lever, value) {
                    (LeverKey::Bailout, PolicyValue::Bool(b)) => {
                        Ok(st::Command::SetPolicy(st::PolicyChange::Bailout(b)))
                    }
                    (LeverKey::CommonsAccess, PolicyValue::Commons(m)) => {
                        let access = match m {
                            CommonsAccessWire::Open => st::CommonsAccess::Open,
                            CommonsAccessWire::Community { cap } => {
                                if !(cap.is_finite() && (0.0..=COMMUNITY_CAP_MAX).contains(&cap)) {
                                    return Err(Rejection::invalid(format!(
                                        "{name}: community cap must be in 0..={COMMUNITY_CAP_MAX}"
                                    )));
                                }
                                st::CommonsAccess::Community { cap: st::Rate(cap) }
                            }
                        };
                        Ok(st::Command::SetPolicy(st::PolicyChange::CommonsAccess(
                            access,
                        )))
                    }
                    (_, PolicyValue::Number(v)) => {
                        if !v.is_finite() || v < info.min - 1e-9 || v > info.max + 1e-9 {
                            return Err(Rejection::invalid(format!(
                                "{name}: value {v} is outside {}..={}",
                                info.min, info.max
                            )));
                        }
                        numeric_change(lever, v)
                            .map(st::Command::SetPolicy)
                            .ok_or_else(|| {
                                Rejection::invalid(format!("{name} does not take a number"))
                            })
                    }
                    _ => Err(Rejection::invalid(format!("{name}: wrong value type"))),
                }
            }
        }
    }
}
