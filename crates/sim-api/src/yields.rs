//! The sustainable-yield view (contract v2.6, `docs/features/farm-from-nature.md` section 3).
//!
//! For each natural stock the farm draws on (the commons and the forest): how much the stock
//! regrows this month at its present size, the best it could ever regrow (maximum sustainable
//! yield), how much was taken, whether taking is over the sustainable line, and how many months
//! until the stock collapses if nothing changes. Every figure is a pure function of the end state
//! via `sim-metrics`; every label and sentence comes from `content/ui.json` (`nature_yield`).
//! Nothing here changes the simulation, and no figure is a bare percentage: the ratio is written
//! "X taken for every Y regrown".
//!
//! "At this stock" figures look forward one month: regrowth is what the stock would regrow from
//! its present size (with the present drought), "taken" is what the farm took this month, so
//! `regrowth - taken` is the change next month if nothing changes. That is what the
//! `parts` reconcile (`reconciles`), and what the months-to-collapse row is computed from.
use crate::breakdowns::Part;
use crate::error::{code, ApiError, ApiResult};
use crate::history::HistoryPoint;
use crate::markets::{close, fill};
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_params::Params;
use sim_types::State;
use std::collections::BTreeMap;

/// The stocks the view covers.
pub const STOCK_IDS: [&str; 2] = ["commons", "forest"];

/// The rows a stock can show, in display order (content gives each a label).
pub fn row_ids(stock: &str) -> &'static [&'static str] {
    match stock {
        "commons" => &[
            "stock",
            "regrowth",
            "msy",
            "taken",
            "ratio",
            "over",
            "months",
            "food_cap",
            "food_wanted",
        ],
        "forest" => &[
            "stock", "regrowth", "msy", "taken", "ratio", "over", "months", "food_cap", "sink",
        ],
        _ => &[],
    }
}

/// Words shared by both stocks. Placeholders are named in each field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YieldAccess {
    pub label: String,
    pub mechanism: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct YieldWords {
    /// What each access rule is called, and which coordination mechanism it leans on (an id of
    /// `markets.mechanisms`; open and enclosed are a market, community is community rules).
    pub access: BTreeMap<String, YieldAccess>,
    pub intro: String,
    /// `{taken}`, `{regrown}`.
    pub ratio: String,
    pub ratio_none: String,
    pub over_yes: String,
    pub over_no: String,
    pub months_none: String,
    /// `{months}`.
    pub months: String,
    pub no_limit: String,
    pub series_label_stock: String,
    pub series_label_regrowth: String,
    pub series_label_taken: String,
    pub change_label: String,
    pub parts_regrowth: String,
    pub parts_taken: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct YieldStockDef {
    pub id: String,
    pub title: String,
    pub blurb: String,
    /// The unit of the stock itself, plural ("units of commons").
    pub stock_unit: String,
    /// What a month of regrowth or taking is measured in ("units of commons a month").
    pub flow_unit: String,
    pub food_unit: String,
    pub months_unit: String,
    pub tonnes_unit: String,
    pub formula: String,
    /// Row id to label (see [`row_ids`]).
    pub rows: BTreeMap<String, String>,
    /// `{taken}`, `{regrowth}`, `{months}`.
    pub verdict_over: String,
    pub verdict_under: String,
    pub verdict_empty: String,
    pub how_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YieldContent {
    pub words: YieldWords,
    pub stocks: Vec<YieldStockDef>,
}

impl YieldContent {
    pub fn validated(self) -> ApiResult<Self> {
        let at = |m: &str| {
            ApiError::new(
                code::BAD_METADATA,
                format!("content/ui.json nature_yield: {m}"),
            )
        };
        for id in STOCK_IDS {
            let n = self.stocks.iter().filter(|d| d.id == id).count();
            if n != 1 {
                return Err(at(&format!("stocks must list {id} exactly once")));
            }
        }
        for id in ["open", "enclosed", "community"] {
            if !self.words.access.contains_key(id) {
                return Err(at(&format!("words.access must define {id}")));
            }
        }
        for d in &self.stocks {
            if !STOCK_IDS.contains(&d.id.as_str()) {
                return Err(at(&format!("unknown stock {}", d.id)));
            }
            for r in row_ids(&d.id) {
                if !d.rows.contains_key(*r) {
                    return Err(at(&format!("{} has no label for row {r}", d.id)));
                }
            }
            if let Some(k) = d
                .rows
                .keys()
                .find(|k| !row_ids(&d.id).contains(&k.as_str()))
            {
                return Err(at(&format!("{} has an unknown row {k}", d.id)));
            }
        }
        Ok(self)
    }
}

/// One labelled row. `value` is `None` when there is no figure (no limit, no collapse coming);
/// `text` is the figure written out with its unit, ready to show.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YieldRow {
    pub key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<f64>,
    pub unit: String,
    pub text: String,
}

/// One point of the series (one per finished turn, then the month in progress).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct YieldPoint {
    pub stock: f64,
    pub regrowth: f64,
    pub taken: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YieldView {
    pub id: String,
    pub title: String,
    pub blurb: String,
    pub formula: String,
    /// "1.1 taken for every 1 regrown".
    pub headline: String,
    pub over_line: bool,
    /// One sentence on the state of the stock, from the figures.
    pub verdict: String,
    pub months_to_collapse: Option<f64>,
    pub rows: Vec<YieldRow>,
    /// Regrowth and taken (taken is negative); they sum to `change`.
    pub parts: Vec<Part>,
    pub change: f64,
    pub change_unit: String,
    pub change_label: String,
    pub series: Vec<YieldPoint>,
    /// Labels of the series lines, in the order stock, regrowth, taken.
    pub series_labels: Vec<String>,
    pub how_id: String,
    /// Commons only: how it is run now and the coordination mechanism that implies (label from the
    /// Markets mechanisms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub access: Option<YieldTag>,
    pub reconciles: bool,
}

/// The access rule of the commons and its mechanism tag.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YieldTag {
    /// `open`, `enclosed` or `community`.
    pub id: String,
    pub label: String,
    /// An id of the Markets mechanisms (`market`, `commons`, ...).
    pub mechanism: String,
    pub mechanism_label: String,
}

/// The figures of one stock at one moment (shared by the view and the history point).
#[derive(Clone, Copy, Debug)]
pub struct Figures {
    pub stock: f64,
    pub regrowth: f64,
    pub msy: f64,
    pub taken: f64,
}

/// Commons figures from the end state.
pub fn commons_figures(s: &State, p: &Params) -> Figures {
    let drought = sim_model::weather::effective_drought(s, p);
    let df = sim_metrics::commons_drought_factor(p, s.nature.commons_access, drought);
    Figures {
        stock: s.nature.commons.0,
        regrowth: sim_metrics::commons_regrowth(p, s.nature.commons.0, df),
        msy: sim_metrics::logistic_msy(p.nature.commons_growth, p.nature.commons_max),
        taken: s.nature.harvest_commons.0,
    }
}

/// Forest figures from the end state.
pub fn forest_figures(s: &State, p: &Params) -> Figures {
    Figures {
        stock: s.nature.forest.0,
        regrowth: sim_metrics::forest_regrowth(p, s.nature.forest.0),
        msy: sim_metrics::logistic_msy(p.nature.forest_growth, p.nature.forest_max),
        taken: s.nature.harvest_forest.0,
    }
}

fn num(v: f64) -> String {
    if v.abs() >= 100.0 || (v - v.round()).abs() < 0.05 {
        crate::markets::thousands(v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

/// Months until the stock reaches `floor` at a net loss of `loss` a month; `None` if it is not losing.
fn months_until(stock: f64, floor: f64, loss: f64) -> Option<f64> {
    (loss > 1e-9).then(|| ((stock - floor) / loss).max(0.0))
}

fn point(f: Figures) -> YieldPoint {
    YieldPoint {
        stock: f.stock,
        regrowth: f.regrowth,
        taken: f.taken,
    }
}

fn history_series(
    history: &[HistoryPoint],
    pick: impl Fn(&HistoryPoint) -> Option<YieldPoint>,
    now: Option<YieldPoint>,
) -> Vec<YieldPoint> {
    let mut v: Vec<YieldPoint> = history.iter().filter_map(pick).collect();
    v.extend(now);
    v
}

fn tag(
    w: &YieldWords,
    mechanisms: &[crate::markets::MechanismDef],
    access: sim_types::CommonsAccess,
) -> Option<YieldTag> {
    let id = match access {
        sim_types::CommonsAccess::Open => "open",
        sim_types::CommonsAccess::Enclosed(_) => "enclosed",
        sim_types::CommonsAccess::Community { .. } => "community",
    };
    let a = w.access.get(id)?;
    Some(YieldTag {
        id: id.to_string(),
        label: a.label.clone(),
        mechanism: a.mechanism.clone(),
        mechanism_label: mechanisms
            .iter()
            .find(|m| m.id == a.mechanism)
            .map(|m| m.label.clone())
            .unwrap_or_default(),
    })
}

/// Both stocks, commons first. `food_wanted` is the Markets food card's demand for the month.
pub fn build(
    sim: &Sim,
    history: &[HistoryPoint],
    content: &YieldContent,
    mechanisms: &[crate::markets::MechanismDef],
    food_wanted: Option<f64>,
) -> Vec<YieldView> {
    let live = sim.turn_phase.open && sim.turn_phase.months_done > 0;
    let now = live || history.is_empty();
    let (s, p) = (&sim.state, &sim.params);
    content
        .stocks
        .iter()
        .map(|def| {
            let commons = def.id == "commons";
            let f = if commons {
                commons_figures(s, p)
            } else {
                forest_figures(s, p)
            };
            let floor = if commons {
                p.nature.commons_collapse_share * p.nature.commons_max
            } else {
                0.0
            };
            let over = f.taken > f.regrowth + 1e-9;
            let months = if over {
                months_until(f.stock, floor, f.taken - f.regrowth)
            } else {
                None
            };
            let w = &content.words;
            let ratio_text = if f.taken <= 1e-9 {
                fill(
                    &w.ratio,
                    &[("taken", "0".into()), ("regrown", num(f.regrowth))],
                )
            } else if f.regrowth > 1e-9 {
                fill(
                    &w.ratio,
                    &[
                        ("taken", num(f.taken / f.regrowth)),
                        ("regrown", "1".into()),
                    ],
                )
            } else {
                w.ratio_none.clone()
            };
            let cap = if commons {
                sim_metrics::food_cap_commons(
                    p,
                    f.stock,
                    s.nature.commons_access,
                    s.policy.commons_harvest.0,
                )
            } else {
                sim_metrics::food_cap_forest(p, f.stock)
            };
            let row = |key: &str, value: Option<f64>, unit: &str, text: String| YieldRow {
                key: key.to_string(),
                label: def.rows.get(key).cloned().unwrap_or_default(),
                value,
                unit: unit.to_string(),
                text,
            };
            let flow = |v: f64| format!("{} {}", num(v), def.flow_unit);
            let mut rows = Vec::new();
            for key in row_ids(&def.id) {
                rows.push(match *key {
                    "stock" => row(
                        key,
                        Some(f.stock),
                        &def.stock_unit,
                        format!("{} {}", num(f.stock), def.stock_unit),
                    ),
                    "regrowth" => row(key, Some(f.regrowth), &def.flow_unit, flow(f.regrowth)),
                    "msy" => row(key, Some(f.msy), &def.flow_unit, flow(f.msy)),
                    "taken" => row(key, Some(f.taken), &def.flow_unit, flow(f.taken)),
                    "ratio" => row(
                        key,
                        (f.regrowth > 1e-9).then(|| f.taken / f.regrowth),
                        "ratio",
                        ratio_text.clone(),
                    ),
                    "over" => row(
                        key,
                        Some(if over { 1.0 } else { 0.0 }),
                        "flag",
                        if over {
                            w.over_yes.clone()
                        } else {
                            w.over_no.clone()
                        },
                    ),
                    "months" => row(
                        key,
                        months,
                        &def.months_unit,
                        months.map_or_else(
                            || w.months_none.clone(),
                            |m| fill(&w.months, &[("months", num(m))]),
                        ),
                    ),
                    "food_cap" => row(
                        key,
                        cap,
                        &def.food_unit,
                        cap.map_or_else(
                            || w.no_limit.clone(),
                            |c| format!("{} {}", num(c), def.food_unit),
                        ),
                    ),
                    "food_wanted" => row(
                        key,
                        food_wanted,
                        &def.food_unit,
                        food_wanted
                            .map_or_else(String::new, |c| format!("{} {}", num(c), def.food_unit)),
                    ),
                    _ => {
                        let sink = sim_model::nature::forest_sink(p, f.stock);
                        row(
                            key,
                            Some(sink),
                            &def.tonnes_unit,
                            format!("{} {}", num(sink), def.tonnes_unit),
                        )
                    }
                });
            }
            let verdict = fill(
                if f.stock <= floor + 1e-9 && floor > 0.0 || f.stock <= 1e-9 {
                    &def.verdict_empty
                } else if over {
                    &def.verdict_over
                } else {
                    &def.verdict_under
                },
                &[
                    ("taken", num(f.taken)),
                    ("regrowth", num(f.regrowth)),
                    ("months", months.map_or_else(String::new, num)),
                ],
            );
            let change = f.regrowth - f.taken;
            let parts = vec![
                Part {
                    label: w.parts_regrowth.clone(),
                    value: f.regrowth,
                    unit: def.flow_unit.clone(),
                    note: None,
                },
                Part {
                    label: w.parts_taken.clone(),
                    value: -f.taken,
                    unit: def.flow_unit.clone(),
                    note: None,
                },
            ];
            let series = if commons {
                history_series(
                    history,
                    |h| {
                        Some(YieldPoint {
                            stock: h.commons_stock?,
                            regrowth: h.commons_regrowth?,
                            taken: h.commons_taken?,
                        })
                    },
                    now.then(|| point(f)),
                )
            } else {
                history_series(
                    history,
                    |h| {
                        Some(YieldPoint {
                            stock: h.forest_stock?,
                            regrowth: h.forest_regrowth?,
                            taken: h.forest_taken?,
                        })
                    },
                    now.then(|| point(f)),
                )
            };
            YieldView {
                id: def.id.clone(),
                title: def.title.clone(),
                blurb: def.blurb.clone(),
                formula: def.formula.clone(),
                headline: ratio_text,
                over_line: over,
                verdict,
                months_to_collapse: months,
                rows,
                reconciles: close(parts.iter().map(|x| x.value).sum::<f64>(), change)
                    && f.stock >= 0.0
                    && cap.is_none_or(|c| c >= 0.0 && c.is_finite()),
                parts,
                change,
                change_unit: def.flow_unit.clone(),
                change_label: w.change_label.clone(),
                series,
                series_labels: vec![
                    w.series_label_stock.clone(),
                    w.series_label_regrowth.clone(),
                    w.series_label_taken.clone(),
                ],
                how_id: def.how_id.clone(),
                access: commons
                    .then(|| tag(w, mechanisms, s.nature.commons_access))
                    .flatten(),
            }
        })
        .collect()
}
