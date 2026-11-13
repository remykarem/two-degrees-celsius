//! S11 weather (Town v4): climate stress, drought, forest fires, and the forecast.
//! Rules: docs/causal-model.md section 11. Runs before production (S2) and nature (S7).
//!
//! RNG draw order per tick (it fixes every seeded outcome):
//! 1. Drought onset: one `chance` if no drought was active at the start of the tick and stress
//!    is above zero; on a hit, two `next_f64` draws (severity, then months).
//! 2. Fire spread: for each fire that survives this tick, in list order, for each neighbour in
//!    the order north, east, south, west that is a standing forest tile not already burning:
//!    one `chance`.
//! 3. Ignition: one `chance` if stress is above zero and a standing forest tile exists; on a hit,
//!    one `below(standing tiles)` picks the tile, in row-major order.
//! 4. Home damage, only if `home_damage_prob > 0`: one `chance` per home next to a burning tile.
//!
//! The forecast never draws.
//!
//! Helpers take the `Weather` to read explicitly, because within one tick the rules must see the
//! drought and fires as updated earlier in the same call, before the engine applies the deltas.

use crate::util::{apportion, labour_weights};
use libm::exp;
use sim_metrics::{climate_stress, commons_health};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::Effect as Fx;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;

/// Share (0..1) of the maximum effect that the current resilience spending buys.
/// A forest tile has four orthogonal neighbours (north, east, south, west): grid geometry.
const GRID_NEIGHBOURS: f64 = 4.0;

pub fn resilience_share(s: &State, p: &Params) -> f64 {
    let spend = s.policy.resilience_spending.dollars().max(0.0);
    1.0 - exp(-spend / p.weather.resilience_scale_usd)
}

/// Drought severity after resilience spending: the damage factor used by commons, farm and demand.
pub fn effective_drought(s: &State, p: &Params) -> f64 {
    let sev = s.weather.drought.map_or(0.0, |d| d.severity);
    sev * (1.0 - p.weather.resilience_drought_cut_max * resilience_share(s, p))
}

fn tile_of(m: &Map, i: usize) -> Tile {
    Tile {
        x: (i % m.width as usize) as u8,
        y: (i / m.width as usize) as u8,
    }
}

/// Is `t` within the coverage radius of a fire station?
pub fn covered(s: &State, p: &Params, t: Tile) -> bool {
    s.map.cells.iter().enumerate().any(|(i, c)| {
        c.kind == TileKind::FireStation
            && tile_of(&s.map, i).chebyshev(t) <= p.weather.coverage_radius
    })
}

/// Is a burning tile within `radius` (Chebyshev) of `t`?
pub fn burning_near(s: &State, t: Tile, radius: u32) -> bool {
    s.weather
        .fires
        .iter()
        .any(|f| f.tile.chebyshev(t) <= radius)
}

fn forest_tiles(m: &Map) -> Vec<Tile> {
    m.tiles()
        .filter(|t| m.kind_at(*t) == Some(TileKind::Forest))
        .collect()
}

fn is_standing(w: &Weather, t: Tile) -> bool {
    !w.fires.iter().any(|f| f.tile == t) && !w.regrowing.iter().any(|g| g.tile == t)
}

/// Dryness multiplier of fire odds: rises with a drought and with a degraded commons.
pub fn dryness(s: &State, p: &Params, w: &Weather) -> f64 {
    let c = &p.weather;
    1.0 + c.dry_commons * (1.0 - commons_health(s, p))
        + c.dry_drought * w.drought.map_or(0.0, |d| d.severity)
}

/// Monthly odds that a fire ignites somewhere, at climate stress `stress`.
pub fn ignition_p(s: &State, p: &Params, w: &Weather, stress: f64) -> f64 {
    if stress <= 0.0 {
        return 0.0;
    }
    let c = &p.weather;
    let forest = forest_tiles(&s.map);
    if forest.is_empty() {
        return 0.0;
    }
    let standing = forest.iter().filter(|t| is_standing(w, **t)).count() as f64;
    let share = standing / forest.len() as f64;
    let cut = c.resilience_ignition_cut_max * resilience_share(s, p);
    (c.fire_base * (1.0 + c.fire_stress_gain * stress) * dryness(s, p, w) * share * (1.0 - cut))
        .clamp(0.0, 1.0)
}

/// Odds that a fire reaches the standing forest tile `t` next month: the monthly ignition odds
/// shared over the standing tiles (ignition picks one uniformly), combined with the spread odds
/// from each burning orthogonal neighbour. `None` for a tile that is not standing forest. It
/// reads the state only and draws nothing; it ignores fires that burn out first, so it is the
/// odds as they stand now, not a prediction.
pub fn tile_fire_risk(s: &State, p: &Params, t: Tile) -> Option<f64> {
    let w = &s.weather;
    if s.map.kind_at(t) != Some(TileKind::Forest) || !is_standing(w, t) {
        return None;
    }
    let stress = climate_stress(s.nature.cumulative, p);
    let standing = forest_tiles(&s.map)
        .into_iter()
        .filter(|f| is_standing(w, *f))
        .count() as f64;
    let ignite = ignition_p(s, p, w, stress) / standing.max(1.0);
    let spread = spread_p(s, p, w, t);
    let mut stays = 1.0 - ignite;
    for f in &w.fires {
        if f.tile.x.abs_diff(t.x) + f.tile.y.abs_diff(t.y) == 1 {
            stays *= 1.0 - spread;
        }
    }
    Some((1.0 - stays).clamp(0.0, 1.0))
}

/// Monthly odds that a drought starts, at climate stress `stress`.
pub fn drought_p(p: &Params, stress: f64) -> f64 {
    if stress <= 0.0 {
        return 0.0;
    }
    let c = &p.weather;
    (c.drought_base * (1.0 + c.drought_stress_gain * stress)).clamp(0.0, 1.0)
}

/// Odds that fire spreads from a burning tile to the adjacent standing tile `to`.
pub fn spread_p(s: &State, p: &Params, w: &Weather, to: Tile) -> f64 {
    let c = &p.weather;
    let cut = if covered(s, p, to) {
        c.station_spread_cut
    } else {
        0.0
    };
    let d = w.drought.map_or(0.0, |d| d.severity);
    (c.spread_p * (1.0 + c.spread_drought * d) * (1.0 - cut)).clamp(0.0, 1.0)
}

/// Months a fire starting or spreading on `t` burns.
pub fn fire_duration(s: &State, p: &Params, t: Tile) -> u32 {
    let c = &p.weather;
    if covered(s, p, t) {
        ((c.duration_months as f64 * (1.0 - c.station_duration_cut)).round() as u32).max(1)
    } else {
        c.duration_months.max(1)
    }
}

/// Forecast (fire risk, drought risk), computed from the state only. It never draws random
/// numbers, so it cannot reveal what will happen, only how likely it is.
pub fn forecast(s: &State, p: &Params) -> (RiskLevel, RiskLevel) {
    let c = &p.weather;
    let w = &s.weather;
    let stress = climate_stress(s.nature.cumulative, p);
    let level = |x: f64, med: f64, high: f64| {
        if x >= high {
            RiskLevel::High
        } else if x >= med {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    };
    let quarter = p.run.ticks_per_turn as f64;
    let fire = if !w.fires.is_empty() {
        RiskLevel::High
    } else {
        let standing: Vec<Tile> = forest_tiles(&s.map)
            .into_iter()
            .filter(|t| is_standing(w, *t))
            .collect();
        let cov = if standing.is_empty() {
            0.0
        } else {
            standing.iter().filter(|t| covered(s, p, **t)).count() as f64 / standing.len() as f64
        };
        let d = w.drought.map_or(0.0, |d| d.severity);
        let spread = c.spread_p * (1.0 + c.spread_drought * d) * (1.0 - c.station_spread_cut * cov);
        let dur = c.duration_months as f64 * (1.0 - c.station_duration_cut * cov);
        // Four neighbours; each is reached with `spread` odds per burning month.
        let size = 1.0 + c.forecast_size_gain * GRID_NEIGHBOURS * spread * dur;
        level(
            quarter * ignition_p(s, p, w, stress) * size,
            c.forecast_fire_medium,
            c.forecast_fire_high,
        )
    };
    let drought = if w.drought.is_some() {
        RiskLevel::High
    } else {
        level(
            quarter * drought_p(p, stress),
            c.forecast_drought_medium,
            c.forecast_drought_high,
        )
    };
    (fire, drought)
}

// ---- Mechanisms (causality graph stage 1): declarations of the plain code below; they change no behaviour. ----

/// Burning tiles lose a share of the forest stock (`weather::run`).
pub const FIRE_BURNS_FOREST: MechanismSpec = MechanismSpec {
    id: M::FireBurnsForest,
    system: SystemId::Weather,
    inputs: &[Q::Fire, Q::ForestStock],
    outputs: &[Q::ForestStock],
    effects: &[
        Fx::down(Q::Fire, Q::ForestStock).unverified("the arithmetic is inline in weather::run over per-firm or per-household state and sim-model tests cannot build a State; no pure function to nudge (burning tiles on the map)"),
        Fx::up(Q::ForestStock, Q::ForestStock).unverified("the arithmetic is inline in weather::run over per-firm or per-household state and sim-model tests cannot build a State; no pure function to nudge (burning tiles on the map)"),
    ],
    params: &["weather.burn_frac"],
    rule: "burning_tiles_lose_forest",
};

/// Climate stress is 0 until the carbon budget runs low and rises to 1 as it runs out (`climate_stress`).
pub const CLIMATE_STRESS_CURVE: MechanismSpec = MechanismSpec {
    id: M::ClimateStressCurve,
    system: SystemId::Weather,
    inputs: &[Q::CumulativeEmissions],
    outputs: &[Q::ClimateStress],
    effects: &[Fx::up(Q::CumulativeEmissions, Q::ClimateStress)],
    params: &[
        "weather.onset",
        "weather.full",
        "weather.curve",
        "nature.budget_b0",
    ],
    rule: "stress_from_budget_used",
};

/// A drought starts by chance, more likely and more severe as stress rises, and ends when its months run out (`weather::run`).
pub const DROUGHT_ONSET: MechanismSpec = MechanismSpec {
    id: M::DroughtOnset,
    system: SystemId::Weather,
    inputs: &[Q::ClimateStress, Q::Drought],
    outputs: &[Q::Drought],
    effects: &[
        Fx::up(Q::ClimateStress, Q::Drought),
        Fx::up(Q::Drought, Q::Drought).unverified("needs a whole State (map, firms, households) which sim-model tests cannot build; no system-level probe was written (a drought in progress is weather state)"),
    ],
    params: &[
        "weather.drought_base",
        "weather.drought_stress_gain",
        "weather.sev_min",
        "weather.sev_span_base",
        "weather.sev_span_stress",
        "weather.dur_min",
        "weather.dur_span_base",
        "weather.dur_span_stress",
    ],
    rule: "drought_chance_by_stress",
};

/// Fires start by chance and spread to neighbouring forest, more in dry conditions, less with fire stations and resilience spending (`weather::run`).
pub const FIRE_IGNITION: MechanismSpec = MechanismSpec {
    id: M::FireIgnition,
    system: SystemId::Weather,
    inputs: &[
        Q::ClimateStress,
        Q::Drought,
        Q::CommonsStock,
        Q::ResilienceSpending,
        Q::Fire,
    ],
    outputs: &[Q::Fire],
    effects: &[
        Fx::up(Q::ClimateStress, Q::Fire).unverified("ignition_p reads the map, the weather and the commons on a State, which sim-model tests cannot build; no system-level probe was written"),
        Fx::up(Q::Drought, Q::Fire).unverified("ignition_p reads the map, the weather and the commons on a State, which sim-model tests cannot build; no system-level probe was written"),
        Fx::down(Q::CommonsStock, Q::Fire).unverified("ignition_p reads the map, the weather and the commons on a State, which sim-model tests cannot build; no system-level probe was written"),
        Fx::down(Q::ResilienceSpending, Q::Fire).unverified("ignition_p reads the map, the weather and the commons on a State, which sim-model tests cannot build; no system-level probe was written"),
        Fx::varies(Q::Fire, Q::Fire, "burning tiles cannot ignite again (fewer new fires) but they spread to their neighbours (more fire)").unverified("ignition_p reads the map, the weather and the commons on a State, which sim-model tests cannot build; no system-level probe was written"),
    ],
    params: &[
        "weather.fire_base",
        "weather.fire_stress_gain",
        "weather.dry_commons",
        "weather.dry_drought",
        "weather.resilience_ignition_cut_max",
        "weather.resilience_scale_usd",
        "weather.spread_p",
        "weather.spread_drought",
        "weather.duration_months",
        "weather.station_spread_cut",
        "weather.station_duration_cut",
        "weather.coverage_radius",
        "nature.commons_max",
    ],
    rule: "fire_chance_by_dryness",
};

pub fn run(s: &State, p: &Params, _shared: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let c = &p.weather;
    // Every draw below comes from the weather's own stream (`s.weather.rng`), never from the
    // shared one: the player's choices cannot shift when droughts and fires happen.
    let mut r = Rng::from_parts(s.weather.rng.state, s.weather.rng.inc);
    let stress = climate_stress(s.nature.cumulative, p);
    let mut wx = s.weather.clone();
    wx.stress = stress;

    // ---- Drought ----
    if let Some(dr) = wx.drought {
        if dr.months_left <= 1 {
            wx.drought = None;
            d.events.push(Event::DroughtEnded);
        } else {
            wx.drought = Some(Drought {
                severity: dr.severity,
                months_left: dr.months_left - 1,
            });
        }
    } else if stress > 0.0 && r.chance(drought_p(p, stress)) {
        let sev_u = r.next_f64();
        let dur_u = r.next_f64();
        let severity =
            (c.sev_min + sev_u * (c.sev_span_base + c.sev_span_stress * stress)).min(1.0);
        let months = (c.dur_min + (dur_u * (c.dur_span_base + c.dur_span_stress * stress)).floor())
            .max(1.0) as u32;
        wx.drought = Some(Drought {
            severity,
            months_left: months,
        });
        d.events.push(Event::DroughtStarted {
            severity: Rate(severity),
            months,
        });
    }

    // ---- Fires ----
    let forest = forest_tiles(&s.map);
    // Regrowth counts down; a tile is standing again when it ends.
    let mut regrowing: Vec<Regrowing> = wx
        .regrowing
        .iter()
        .filter(|g| g.months_left > 1)
        .map(|g| Regrowing {
            tile: g.tile,
            months_left: g.months_left - 1,
        })
        .collect();
    // Fires burn one more month or go out and leave a regrowing tile.
    let mut fires: Vec<Fire> = Vec::new();
    for f in &wx.fires {
        if f.months_left <= 1 {
            d.events.push(Event::FireOut { tile: f.tile });
            d.events.push(Event::ForestBurned { tile: f.tile });
            regrowing.push(Regrowing {
                tile: f.tile,
                months_left: c.regrow_months.max(1),
            });
        } else {
            fires.push(Fire {
                tile: f.tile,
                months_left: f.months_left - 1,
            });
        }
    }
    // What the spread and ignition rules read: this tick's drought, fires and regrowth.
    let mut view = Weather {
        stress,
        drought: wx.drought,
        fires: fires.clone(),
        regrowing: regrowing.clone(),
        rng: wx.rng,
    };
    let survivors = fires.len();
    for i in 0..survivors {
        let from = fires[i].tile;
        for (dx, dy) in [(0i32, -1i32), (1, 0), (0, 1), (-1, 0)] {
            let (nx, ny) = (from.x as i32 + dx, from.y as i32 + dy);
            if !(0..=u8::MAX as i32).contains(&nx) || !(0..=u8::MAX as i32).contains(&ny) {
                continue;
            }
            let to = Tile {
                x: nx as u8,
                y: ny as u8,
            };
            if s.map.kind_at(to) != Some(TileKind::Forest) || !is_standing(&view, to) {
                continue;
            }
            if r.chance(spread_p(s, p, &view, to)) {
                fires.push(Fire {
                    tile: to,
                    months_left: fire_duration(s, p, to),
                });
                view.fires = fires.clone();
                d.events.push(Event::FireSpread { from, to });
            }
        }
    }
    // Ignition.
    if stress > 0.0 {
        let standing: Vec<Tile> = forest
            .iter()
            .copied()
            .filter(|t| is_standing(&view, *t))
            .collect();
        if !standing.is_empty() && r.chance(ignition_p(s, p, &view, stress)) {
            let tile = standing[r.below(standing.len() as u32) as usize];
            fires.push(Fire {
                tile,
                months_left: fire_duration(s, p, tile),
            });
            d.events.push(Event::FireStarted { tile });
        }
    }

    // ---- Burning: forest stock lost this month ----
    let burning = fires.len() as f64;
    if burning > 0.0 && !forest.is_empty() {
        let tile_stock = s.nature.forest.0 / forest.len() as f64;
        let loss = (burning * c.burn_frac * tile_stock).min(s.nature.forest.0);
        let mut n = s.nature.clone();
        n.forest = Units((n.forest.0 - loss).max(0.0));
        d.changes.push(Delta::Nature(n));
        d.trace.push(TraceRecord::Emissions {
            source: EmissionSource::FireSinkLoss,
            tonnes: Tonnes(p.nature.sink_sigma * loss),
        });
    }

    // ---- Optional damage to homes next to a fire ----
    if c.home_damage_prob > 0.0 && !fires.is_empty() {
        let lw = labour_weights(s);
        for home in &s.homes {
            if !fires.iter().any(|f| f.tile.chebyshev(home.tile) <= 1) {
                continue;
            }
            if r.chance(c.home_damage_prob) {
                d.events.push(Event::HomeDamaged { home: home.id });
                let payer = match home.owner {
                    HomeOwner::Household(h) => Account::Household(h),
                    HomeOwner::Public => Account::Government,
                };
                // A payer pays at most what it holds (the estate keeps little cash).
                let held = match payer {
                    Account::Household(h) => s.households[h.idx()].savings,
                    _ => s.gov.cash,
                };
                let cost = Cents::from_dollars(c.home_repair_cost_usd).min(held.max(Cents::ZERO));
                for (h, part) in apportion(cost, &lw).into_iter().enumerate() {
                    d.transfer_as(
                        payer,
                        Account::Household(HouseholdId(h as u32)),
                        part,
                        FlowKind::Wage,
                        Purpose::RepairLabour,
                    );
                }
            }
        }
    }

    wx.fires = fires;
    wx.regrowing = regrowing;
    let (state, inc) = r.parts();
    wx.rng = RngStream { state, inc };
    d.changes.push(Delta::Weather(wx));
    d
}
