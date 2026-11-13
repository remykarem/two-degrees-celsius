//! The Markets view (contract v2.3, `docs/features/markets.md`, scope A: information only).
//!
//! Every market is one card with the same anatomy: what was **wanted** (demand), what was
//! **available** (supply), what was **traded**, and the two gaps: **unmet** (wanted but not
//! traded) and **unsold** (available but not traded). So `demand = traded + unmet` and
//! `supply = traded + unsold` for every market, whatever the unit (units of goods, people, homes,
//! dollars of credit, energy). Each card also carries the price (a shelf price, a wage, a rent, a
//! loan rate) with its direction since the last turn and the cause word the model really uses,
//! the people or firms that are short, and a `reconciles` flag.
//!
//! Figures come from two places and nothing here changes the simulation: the end state (prices,
//! people, homes, energy, via `sim-metrics`) and the output-only records the model emits while it
//! works (`TraceRecord::GoodsFirm`, `Vacancies`, `CreditRoom`, `CreditDecision`, `RentBurden`),
//! kept per tick in the wrapper log. `reconciles` cross-checks the two (for example the traced
//! sales against each firm's stored `sales_units`).
//!
//! The ids are a closed set the code knows how to total ([`MARKET_IDS`]); labels, units,
//! categories, verdict sentences, cause words and the coordination mechanisms live in
//! `content/ui.json` (`markets`), validated at load like the net worth lines. Adding a market is
//! a content entry plus one arm in [`raw_for`]; the web draws whatever list the snapshot has.
use crate::error::{code, ApiError, ApiResult};
use crate::history::HistoryPoint;
use crate::log::{GoodsFirmTrace, WrapperLog};
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types::{Firm, QuantityId, Sector, State};
use std::collections::BTreeMap;

/// Markets the code can total, in display order of the wire's own sums.
pub const MARKET_IDS: [&str; 7] = [
    "food", "goods", "services", "labour", "housing", "credit", "energy",
];

/// What the "who is short" chips of a market can be (content must give each a sentence).
pub fn chip_ids(market: &str) -> &'static [&'static str] {
    match market {
        "food" => &["sold_out", "unsold", "households_short", "commons_limit"],
        "goods" | "services" => &["sold_out", "unsold"],
        "labour" => &["jobless", "guarantee", "open_jobs"],
        "housing" => &["unhoused", "vacant"],
        "credit" => &["refused"],
        "energy" => &["unserved"],
        _ => &[],
    }
}

/// What the "drivers" of a market can be (content must give each a label).
pub fn driver_ids(market: &str) -> &'static [&'static str] {
    match market {
        "food" | "goods" | "services" => &["markup"],
        "labour" => &["unemployment", "u_ref"],
        "housing" => &["rent_burden", "rent_burden_cap"],
        "credit" => &["base_rate"],
        _ => &[],
    }
}

/// Contract v2.9: the graph quantity a market's price is, where the registry has one (the
/// housing rent and electricity are not in the decision registry yet).
pub fn price_quantity(market: &str) -> Option<QuantityId> {
    match market {
        "food" => Some(QuantityId::FoodPrice),
        "goods" | "services" => Some(QuantityId::Price),
        "labour" => Some(QuantityId::Wage),
        "credit" => Some(QuantityId::LoanRate),
        _ => None,
    }
}

/// Contract v2.9: the graph quantity a market driver is, where the registry has one.
pub fn driver_quantity(driver: &str) -> Option<QuantityId> {
    match driver {
        "markup" => Some(QuantityId::Markup),
        "unemployment" => Some(QuantityId::Unemployment),
        "base_rate" => Some(QuantityId::PolicyRate),
        _ => None,
    }
}

/// The columns of a market's trade list, in order (content gives each a label and help text).
pub fn trade_column_ids(market: &str) -> &'static [&'static str] {
    match market {
        "food" | "goods" | "services" => &[
            "buyer",
            "seller",
            "units",
            "unit_price",
            "total",
            "note",
            "month",
        ],
        "labour" => &["worker", "employer", "wage", "note", "month"],
        "housing" => &["home", "tenant", "owner", "rent", "note", "month"],
        "credit" => &[
            "borrower", "lender", "asked", "approved", "rate", "note", "month",
        ],
        _ => &[],
    }
}

/// The "what changed" notes a market's trade rows can carry (content gives each a sentence).
pub fn trade_note_ids(market: &str) -> &'static [&'static str] {
    match market {
        "food" | "goods" | "services" => &["self"],
        "labour" => &["hired", "fired", "open", "guarantee", "staff"],
        "housing" => &["vacant", "owner", "unhoused"],
        "credit" => &["full", "partial", "refused"],
        _ => &[],
    }
}

/// Names the trade lists use for parties that have no label of their own.
pub const TRADE_NAMES: [&str; 5] = ["government", "bank", "home", "open_job", "nobody"];

/// The direction ids a price can have (content gives each a word).
const DIRECTIONS: [&str; 5] = ["rising", "steady", "falling", "none", "unknown"];
/// The cause ids content may use.
const CAUSES: [&str; 5] = ["cost", "slack", "scarcity", "risk", "none"];

// ---- the authored content ----

/// How a quantity is written: a count, dollars, or the energy unit.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmountKind {
    Count,
    Money,
    Energy,
}

/// How a price is written: dollars, an annual rate, or no price.
/// Which gaps a market has this month.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    /// Wanted and available match.
    None,
    /// Something wanted went without.
    Short,
    /// Something available stayed unsold.
    Glut,
    /// Both at once (different sellers or buyers).
    Both,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceKind {
    Money,
    Rate,
    None,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verdict {
    pub balanced: String,
    pub short: String,
    pub glut: String,
    pub both: String,
}

/// A link from a card to another view (contract v2.6): a label and an anchor such as `panel:ecology`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketLink {
    pub label: String,
    pub target: String,
}

/// A chip sentence, singular and plural. Placeholders: `{count}`, `{firm}`, `{unit}`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortText {
    pub one: String,
    pub other: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MarketDef {
    pub id: String,
    pub label: String,
    /// One of `MarketsContent.categories`.
    pub category: String,
    /// Coordination mechanisms this market leans on (ids of `MarketsContent.mechanisms`).
    pub mechanisms: Vec<String>,
    /// A plain description of who is on each side.
    pub blurb: String,
    /// Plural unit word; `{energy}` becomes the energy unit label.
    pub unit: String,
    pub unit_one: String,
    pub amount: AmountKind,
    pub price_label: String,
    pub price_unit: String,
    pub price_kind: PriceKind,
    /// One of `MarketsContent.causes`.
    pub price_cause: String,
    /// An honest sentence on how this price is really set.
    pub price_note: String,
    pub demand_label: String,
    pub supply_label: String,
    pub traded_label: String,
    /// A gap at or below this (in the market's own unit) counts as no gap.
    pub gap_floor: f64,
    pub gap_unmet: String,
    pub gap_unsold: String,
    /// The explorer anchor behind this card's "how?" (a `usedBy` entry of an assumptions topic).
    pub how_id: String,
    /// Placeholders: `{demand}`, `{supply}`, `{traded}`, `{unmet}`, `{unsold}`, `{unit}`.
    pub verdict: Verdict,
    pub short: BTreeMap<String, ShortText>,
    pub drivers: BTreeMap<String, String>,
    /// Contract v2.4: the list of trades behind this market.
    pub trades: TradesDef,
    /// Contract v2.6: an optional link to another view (the food card links to the Ecology view).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub link: Option<MarketLink>,
}

/// One column of a market's trade list.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeColumnDef {
    pub id: String,
    pub label: String,
    /// Plain-English help: what the column means.
    pub help: String,
    pub numeric: bool,
}

/// The text of a market's trade list. `totals` placeholders: `{count}`, `{units}`, `{money}`, `{span}`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradesDef {
    pub intro: String,
    pub empty: String,
    pub columns: Vec<TradeColumnDef>,
    pub notes: BTreeMap<String, String>,
    /// Said under the table when some rows are listed but not counted; empty when all are counted.
    pub uncounted: String,
    pub totals: String,
}

/// Words shared by every market's trade list.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TradesCommon {
    /// Keys: [`TRADE_NAMES`]. `home` has `{n}`.
    pub names: BTreeMap<String, String>,
    pub span_month: String,
    /// `{months}`.
    pub span_months: String,
    /// `{month}`.
    pub month_word: String,
    /// `{months}`.
    pub quarter_word: String,
    /// `{turn}`, `{month}`.
    pub period_month: String,
    pub period_turn: String,
    pub period_rolled: String,
    /// `{rate}`: dollars a year for every $100 borrowed.
    pub rate: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketCategoryDef {
    pub id: String,
    pub label: String,
    pub blurb: String,
}

/// One way an economy can be coordinated (a market, rules and planning, a firm, community rules).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanismDef {
    pub id: String,
    pub label: String,
    pub blurb: String,
    pub example: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MarketsContent {
    /// A price that moved by less than this share since the last turn counts as steady.
    pub steady_band: f64,
    pub gap_none: String,
    /// A household whose food need was met at or above this share counts as fed.
    pub fully_met: f64,
    pub trades: TradesCommon,
    pub directions: BTreeMap<String, String>,
    pub causes: BTreeMap<String, String>,
    pub categories: Vec<MarketCategoryDef>,
    pub mechanisms: Vec<MechanismDef>,
    pub list: Vec<MarketDef>,
}

fn bad(msg: String) -> ApiError {
    ApiError::new(
        code::BAD_METADATA,
        format!("content/ui.json markets: {msg}"),
    )
}

fn same_keys(what: &str, got: Vec<&str>, want: &[&str]) -> ApiResult<()> {
    let mut g = got;
    g.sort_unstable();
    let mut w: Vec<&str> = want.to_vec();
    w.sort_unstable();
    if g == w {
        Ok(())
    } else {
        Err(bad(format!("{what} must be exactly {w:?}, got {g:?}")))
    }
}

impl MarketsContent {
    /// Check the content against the closed id sets: every market the code knows once, nothing
    /// unknown, every category, mechanism, cause, chip and driver defined.
    pub fn validated(self) -> ApiResult<Self> {
        same_keys(
            "markets.list ids",
            self.list.iter().map(|d| d.id.as_str()).collect(),
            &MARKET_IDS,
        )?;
        same_keys(
            "markets.directions",
            self.directions.keys().map(String::as_str).collect(),
            &DIRECTIONS,
        )?;
        same_keys(
            "markets.trades.names",
            self.trades.names.keys().map(String::as_str).collect(),
            &TRADE_NAMES,
        )?;
        same_keys(
            "markets.causes",
            self.causes.keys().map(String::as_str).collect(),
            &CAUSES,
        )?;
        for d in &self.list {
            let at = |m: &str| bad(format!("{}: {m}", d.id));
            if !self.categories.iter().any(|c| c.id == d.category) {
                return Err(at(&format!("unknown category {}", d.category)));
            }
            if d.mechanisms.is_empty() {
                return Err(at("needs at least one mechanism"));
            }
            for m in &d.mechanisms {
                if !self.mechanisms.iter().any(|x| &x.id == m) {
                    return Err(at(&format!("unknown mechanism {m}")));
                }
            }
            if !self.causes.contains_key(&d.price_cause) {
                return Err(at(&format!("unknown cause {}", d.price_cause)));
            }
            if (d.price_kind == PriceKind::None) != (d.price_cause == "none") {
                return Err(at("a market with no price has cause none, and only it"));
            }
            same_keys(
                &format!("{} short", d.id),
                d.short.keys().map(String::as_str).collect(),
                chip_ids(&d.id),
            )?;
            same_keys(
                &format!("{} drivers", d.id),
                d.drivers.keys().map(String::as_str).collect(),
                driver_ids(&d.id),
            )?;
            same_keys(
                &format!("{} trades columns", d.id),
                d.trades.columns.iter().map(|c| c.id.as_str()).collect(),
                trade_column_ids(&d.id),
            )?;
            // The columns keep the order the builder writes cells in.
            let order: Vec<&str> = d.trades.columns.iter().map(|c| c.id.as_str()).collect();
            if order != trade_column_ids(&d.id) {
                return Err(at(
                    "trades columns must be in the order the code writes them",
                ));
            }
            same_keys(
                &format!("{} trades notes", d.id),
                d.trades.notes.keys().map(String::as_str).collect(),
                trade_note_ids(&d.id),
            )?;
            if d.how_id.is_empty() || d.gap_floor < 0.0 {
                return Err(at("howId must be set and gapFloor must not be negative"));
            }
        }
        Ok(self)
    }
}

// ---- the wire ----

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketEntityKind {
    Firm,
    Household,
    Home,
    /// Contract v2.4 (trade lists).
    Person,
    Government,
    Bank,
}

/// Something on the map a chip points at (the web selects it in the inspector).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketEntity {
    pub kind: MarketEntityKind,
    pub id: u32,
}

/// Who or what is short (or left over) in a market.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketShort {
    pub id: String,
    /// The finished sentence from the content template.
    pub label: String,
    pub count: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub entity: Option<MarketEntity>,
}

/// A number that explains the price, with its label from the content.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDriver {
    pub id: String,
    pub label: String,
    /// Already written out, with its unit (for example `14% of income`).
    pub value: String,
    /// Contract v2.9: the graph quantity this driver is (`catalog().quantities[].id`), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quantity_id: Option<String>,
}

/// One market, this month (the latest tick). Quantities are in the market's own unit:
/// `demand = traded + unmet` and `supply = traded + unsold`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketView {
    pub id: String,
    pub label: String,
    pub category: String,
    pub mechanisms: Vec<String>,
    pub blurb: String,
    pub unit: String,
    pub amount: AmountKind,
    pub demand: f64,
    pub supply: f64,
    pub traded: f64,
    /// Wanted but not traded.
    pub unmet: f64,
    /// Available but not traded.
    pub unsold: f64,
    pub demand_label: String,
    pub supply_label: String,
    pub traded_label: String,
    /// The quantities written out, with money as dollars (units are in `unit`).
    pub demand_text: String,
    pub supply_text: String,
    pub traded_text: String,
    /// The gaps in words, for example `30 units went without`; one line saying there is none otherwise.
    pub gap_words: Vec<String>,
    pub gap_kind: GapKind,
    /// One plain-English sentence on this month, built from the figures.
    pub verdict: String,
    pub price_kind: PriceKind,
    pub price_label: String,
    pub price_unit: String,
    /// Dollars (a shelf price, a monthly wage, a monthly rent) or an annual rate as a fraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub price: Option<f64>,
    /// The price written out (`$23.09`); a loan rate is the dollars a year that $100 of borrowing
    /// costs, so the page never shows a bare percentage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub price_text: Option<String>,
    /// `rising`, `steady`, `falling`, `unknown` (no earlier turn yet) or `none` (no price).
    pub price_direction: String,
    pub price_direction_word: String,
    /// What the model really lets move this price: `cost`, `slack`, `scarcity`, `risk` or `none`.
    pub price_cause: String,
    pub price_cause_word: String,
    /// Contract v2.9: the graph quantity the price is (`catalog().quantities[].id`), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub price_quantity: Option<String>,
    pub price_note: String,
    /// The price at the end of each finished turn, oldest first, then this month if the turn is open.
    pub price_series: Vec<f64>,
    pub drivers: Vec<MarketDriver>,
    pub short: Vec<MarketShort>,
    pub how_id: String,
    /// Contract v2.6: a link to another view, from the card's content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub link: Option<MarketLink>,
    /// The figures agree with the end state and with each other (see the module comment).
    pub reconciles: bool,
}

// ---- building ----

struct Chip {
    id: &'static str,
    count: f64,
    firm: Option<String>,
    entity: Option<MarketEntity>,
}

struct Raw {
    demand: f64,
    supply: f64,
    traded: f64,
    unmet: f64,
    unsold: f64,
    price: Option<f64>,
    ok: bool,
    chips: Vec<Chip>,
    drivers: Vec<(&'static str, String)>,
}

pub(crate) fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
}

pub(crate) fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        out.insert(0, '-');
    }
    out
}

/// A quantity written for a sentence: whole numbers from 100 up, one decimal below, no `.0`.
fn amount(v: f64, kind: AmountKind) -> String {
    let v = if v.abs() < 1e-9 { 0.0 } else { v };
    if kind == AmountKind::Money {
        return format!("${}", thousands(v.round() as i64));
    }
    if v.abs() >= 100.0 || (v - v.round()).abs() < 0.05 {
        thousands(v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

/// A share as a count out of a hundred ("19 of every 100 people"): the page never shows a bare
/// percentage.
fn per_hundred(share: f64, tail: &str) -> String {
    format!("{:.0} of every 100 {tail}", share * 100.0)
}

/// A price written for the page: dollars (cents below $100), or an annual rate as the dollars a
/// year that $100 of borrowing costs.
fn price_text(kind: PriceKind, v: f64) -> Option<String> {
    match kind {
        PriceKind::None => None,
        PriceKind::Rate => Some(format!("${:.2}", v * 100.0)),
        PriceKind::Money if v.abs() < 100.0 => Some(format!("${v:.2}")),
        PriceKind::Money => Some(format!("${}", thousands(v.round() as i64))),
    }
}

pub(crate) fn fill(template: &str, vars: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

fn goods_raw(
    sim: &Sim,
    def: &MarketDef,
    sector: Sector,
    rows: &[GoodsFirmTrace],
    names: &BTreeMap<u32, String>,
    fully_met: f64,
) -> Raw {
    let s = &sim.state;
    let in_sector = |f: &Firm| f.sector == sector;
    let mine: Vec<&GoodsFirmTrace> = rows
        .iter()
        .filter(|r| s.firms.get(r.firm as usize).is_some_and(in_sector))
        .collect();
    let floor = def.gap_floor;
    let sold: f64 = mine.iter().map(|r| r.sold_units).sum();
    let stock: f64 = mine.iter().map(|r| r.stock_units).sum();
    let unmet: f64 = mine
        .iter()
        .map(|r| (r.demand_units - r.sold_units).max(0.0))
        .sum();
    let unsold: f64 = mine
        .iter()
        .map(|r| (r.stock_units - r.sold_units).max(0.0))
        .sum();
    // The traced sales are what each firm stored, and what is left is what the shelf holds.
    let stored: f64 = s
        .firms
        .iter()
        .filter(|f| in_sector(f))
        .map(|f| f.sales_units.0)
        .sum();
    let shelves_ok = mine.iter().all(|r| {
        let f = &s.firms[r.firm as usize];
        !f.active || close(f.inventory.0, (r.stock_units - r.sold_units).max(0.0))
    });
    let mut chips = Vec::new();
    for r in &mine {
        let left = r.stock_units - r.sold_units;
        let name = names.get(&r.firm).cloned();
        let entity = Some(MarketEntity {
            kind: MarketEntityKind::Firm,
            id: r.firm,
        });
        if r.demand_units - r.sold_units > floor && left <= floor {
            chips.push(Chip {
                id: "sold_out",
                count: 1.0,
                firm: name.clone(),
                entity,
            });
        }
        if left > floor {
            chips.push(Chip {
                id: "unsold",
                count: 1.0,
                firm: name,
                entity,
            });
        }
    }
    if sector == Sector::Food && unmet > floor {
        let hungry: Vec<&sim_types::Household> = s
            .households
            .iter()
            .filter(|h| h.food_met.0 < fully_met)
            .collect();
        if let Some(first) = hungry.first() {
            chips.push(Chip {
                id: "households_short",
                count: hungry.len() as f64,
                firm: None,
                entity: Some(MarketEntity {
                    kind: MarketEntityKind::Household,
                    id: first.id.0,
                }),
            });
        }
    }
    // Farm-from-nature: when the stocks left cap next month's food below what the farm made this
    // month, say so (the count is the most food the commons and forest can supply).
    if sector == Sector::Food {
        let made: f64 = s
            .firms
            .iter()
            .filter(|f| f.active && in_sector(f))
            .map(|f| f.output.0)
            .sum();
        if let Some(cap) = sim_metrics::food_cap(s, &sim.params) {
            if cap + floor < made {
                chips.push(Chip {
                    id: "commons_limit",
                    count: cap,
                    firm: None,
                    entity: None,
                });
            }
        }
    }
    let firms: Vec<&Firm> = s
        .firms
        .iter()
        .filter(|f| f.active && in_sector(f))
        .collect();
    let markup = if firms.is_empty() {
        0.0
    } else {
        firms.iter().map(|f| f.markup.0).sum::<f64>() / firms.len() as f64
    };
    Raw {
        demand: sold + unmet,
        supply: stock,
        traded: sold,
        unmet,
        unsold,
        price: crate::history::shelf_price(s, sector),
        ok: close(sold, stored) && shelves_ok,
        chips,
        drivers: vec![("markup", format!("${markup:.2} on top of every $1 of cost"))],
    }
}

fn labour_raw(sim: &Sim, tick: &crate::log::TickRecord, names: &BTreeMap<u32, String>) -> Raw {
    let s = &sim.state;
    let l = sim_metrics::labour_figures(s);
    let posted: u32 = tick.markets.vacancies.iter().map(|v| v.1).sum();
    let filled: u32 = tick.markets.vacancies.iter().map(|v| v.2).sum();
    let open = posted.saturating_sub(filled);
    let on_books: f64 = s.firms.iter().map(|f| f.workers.0).sum();
    let mut chips = Vec::new();
    if l.jobless > l.guarantee {
        chips.push(Chip {
            id: "jobless",
            count: f64::from(l.jobless - l.guarantee),
            firm: None,
            entity: None,
        });
    }
    if l.guarantee > 0 {
        chips.push(Chip {
            id: "guarantee",
            count: f64::from(l.guarantee),
            firm: None,
            entity: None,
        });
    }
    for &(firm, p, f) in &tick.markets.vacancies {
        if p > f {
            chips.push(Chip {
                id: "open_jobs",
                count: f64::from(p - f),
                firm: names.get(&firm).cloned(),
                entity: Some(MarketEntity {
                    kind: MarketEntityKind::Firm,
                    id: firm,
                }),
            });
        }
    }
    Raw {
        demand: f64::from(l.firm_jobs + open),
        supply: f64::from(l.labour_force),
        traded: f64::from(l.firm_jobs),
        unmet: f64::from(open),
        unsold: f64::from(l.jobless),
        price: sim_metrics::average_wage(s).map(|c| c / 100.0),
        ok: close(on_books, f64::from(l.firm_jobs))
            && posted >= filled
            && l.labour_force == l.firm_jobs + l.jobless,
        chips,
        drivers: vec![
            (
                "unemployment",
                per_hundred(
                    s.indicators.unemployment_smooth.0,
                    "people in the labour force",
                ),
            ),
            (
                "u_ref",
                per_hundred(sim.params.labour.u_ref, "people in the labour force"),
            ),
        ],
    }
}

fn housing_raw(sim: &Sim, tick: &crate::log::TickRecord) -> Raw {
    let s = &sim.state;
    let h = sim_metrics::housing_figures(s);
    let mut chips = Vec::new();
    if let Some(first) = s.households.iter().find(|x| x.home.is_none()) {
        chips.push(Chip {
            id: "unhoused",
            count: f64::from(h.unhoused),
            firm: None,
            entity: Some(MarketEntity {
                kind: MarketEntityKind::Household,
                id: first.id.0,
            }),
        });
    }
    if let Some(first) = s.homes.iter().find(|x| x.household.is_none()) {
        chips.push(Chip {
            id: "vacant",
            count: f64::from(h.vacant),
            firm: None,
            entity: Some(MarketEntity {
                kind: MarketEntityKind::Home,
                id: first.id.0,
            }),
        });
    }
    Raw {
        demand: f64::from(h.households),
        supply: f64::from(h.homes),
        traded: f64::from(h.housed),
        unmet: f64::from(h.unhoused),
        unsold: f64::from(h.vacant),
        price: Some(s.housing.rent_private.dollars()),
        ok: h.housed == h.occupied && h.unhoused == s.housing.unhoused_households,
        chips,
        drivers: vec![
            (
                "rent_burden",
                per_hundred(tick.markets.rent_burden, "dollars of their income"),
            ),
            (
                "rent_burden_cap",
                per_hundred(sim.params.goods.rent_burden_cap, "dollars of their income"),
            ),
        ],
    }
}

fn credit_raw(sim: &Sim, tick: &crate::log::TickRecord, names: &BTreeMap<u32, String>) -> Raw {
    let s = &sim.state;
    let m = &tick.markets;
    let dollars = |c: i64| c as f64 / 100.0;
    let requested: i64 = m.credit.iter().map(|c| c.1).sum();
    let approved: i64 = m.credit.iter().map(|c| c.2).sum();
    let mut chips = Vec::new();
    for &(firm, req, app) in &m.credit {
        if dollars(req - app) > 0.0 {
            chips.push(Chip {
                id: "refused",
                count: 1.0,
                firm: names.get(&firm).cloned(),
                entity: Some(MarketEntity {
                    kind: MarketEntityKind::Firm,
                    id: firm,
                }),
            });
        }
    }
    Raw {
        demand: dollars(requested),
        supply: dollars(m.credit_room),
        traded: dollars(approved),
        unmet: dollars(requested - approved),
        unsold: dollars((m.credit_room - approved).max(0)),
        price: sim_metrics::average_loan_rate(s),
        ok: approved == s.bank.lent_this_tick.0
            && approved <= m.credit_room.max(0)
            && approved <= requested,
        chips,
        drivers: vec![(
            "base_rate",
            format!(
                "${:.2} a year for every $100 borrowed",
                s.policy.base_rate.0 * 100.0
            ),
        )],
    }
}

fn energy_raw(sim: &Sim, floor: f64) -> Raw {
    let e = &sim.state.energy;
    let (demand, supply, delivered) = (e.demand.0, e.supply.0, e.delivered.0);
    let unmet = (demand - delivered).max(0.0);
    Raw {
        demand,
        supply,
        traded: delivered,
        unmet,
        unsold: (supply - delivered).max(0.0),
        price: None,
        ok: delivered <= supply + 1e-6 * supply.max(1.0)
            && delivered <= demand + 1e-6 * demand.max(1.0)
            && close(delivered, demand * e.served.0),
        chips: if unmet > floor {
            vec![Chip {
                id: "unserved",
                count: unmet,
                firm: None,
                entity: None,
            }]
        } else {
            Vec::new()
        },
        drivers: Vec::new(),
    }
}

fn raw_for(
    sim: &Sim,
    tick: &crate::log::TickRecord,
    def: &MarketDef,
    names: &BTreeMap<u32, String>,
    fully_met: f64,
) -> Option<Raw> {
    let rows = &tick.markets.goods;
    Some(match def.id.as_str() {
        "food" => goods_raw(sim, def, Sector::Food, rows, names, fully_met),
        "goods" => goods_raw(sim, def, Sector::Goods, rows, names, fully_met),
        "services" => goods_raw(sim, def, Sector::Services, rows, names, fully_met),
        "labour" => labour_raw(sim, tick, names),
        "housing" => housing_raw(sim, tick),
        "credit" => credit_raw(sim, tick, names),
        "energy" => energy_raw(sim, def.gap_floor),
        _ => return None,
    })
}

/// A market's price at the end of a finished turn.
fn history_price(id: &str, p: &HistoryPoint) -> Option<f64> {
    match id {
        "food" => p.price_food,
        "goods" => p.price_goods,
        "services" => p.price_services,
        "labour" => p.wage_avg,
        "housing" => p.rent_private,
        "credit" => p.loan_rate_avg,
        _ => None,
    }
}

/// The price direction since the previous point of the series: a move smaller than
/// `steady_band` (a share) is steady.
pub fn direction(series: &[f64], steady_band: f64) -> &'static str {
    let [.., prev, last] = series else {
        return "unknown";
    };
    if *prev <= 0.0 {
        return "unknown";
    }
    let change = (last - prev) / prev;
    if change > steady_band {
        "rising"
    } else if change < -steady_band {
        "falling"
    } else {
        "steady"
    }
}

fn view(
    def: &MarketDef,
    c: &MarketsContent,
    raw: Raw,
    history: &[HistoryPoint],
    live: bool,
    energy_unit: &str,
) -> MarketView {
    let unit = def.unit.replace("{energy}", energy_unit);
    let kind = def.amount;
    let floor = def.gap_floor;
    let (short, glut) = (raw.unmet > floor, raw.unsold > floor);
    let vars: Vec<(&str, String)> = vec![
        ("demand", amount(raw.demand, kind)),
        ("supply", amount(raw.supply, kind)),
        ("traded", amount(raw.traded, kind)),
        ("unmet", amount(raw.unmet, kind)),
        ("unsold", amount(raw.unsold, kind)),
        ("unit", unit.clone()),
    ];
    // `{demand_unit}` and friends: the unit word that agrees with that number ("1 home", "2 homes").
    let one = def.unit_one.replace("{energy}", energy_unit);
    let agree = |v: f64| {
        if amount(v, kind) == "1" {
            one.clone()
        } else {
            unit.clone()
        }
    };
    let mut vars = vars;
    for (k, v) in [
        ("demand_unit", raw.demand),
        ("supply_unit", raw.supply),
        ("traded_unit", raw.traded),
        ("unmet_unit", raw.unmet),
        ("unsold_unit", raw.unsold),
    ] {
        vars.push((k, agree(v)));
    }
    let verdict = fill(
        match (short, glut) {
            (true, true) => &def.verdict.both,
            (true, false) => &def.verdict.short,
            (false, true) => &def.verdict.glut,
            (false, false) => &def.verdict.balanced,
        },
        &vars,
    );
    let mut gap_words = Vec::new();
    if short {
        gap_words.push(fill(&def.gap_unmet, &vars));
    }
    if glut {
        gap_words.push(fill(&def.gap_unsold, &vars));
    }
    if gap_words.is_empty() {
        gap_words.push(c.gap_none.clone());
    }
    let mut series: Vec<f64> = history
        .iter()
        .filter_map(|p| history_price(&def.id, p))
        .collect();
    if let (true, Some(now)) = (live, raw.price) {
        series.push(now);
    } else if series.is_empty() {
        series.extend(raw.price);
    }
    let dir = if raw.price.is_none() {
        "none"
    } else {
        direction(&series, c.steady_band)
    };
    let word = |m: &BTreeMap<String, String>, k: &str| m.get(k).cloned().unwrap_or_default();
    let shorts = raw
        .chips
        .iter()
        .filter_map(|chip| {
            let t = def.short.get(chip.id)?;
            let template = if (chip.count - 1.0).abs() < 1e-9 {
                &t.one
            } else {
                &t.other
            };
            let label = fill(
                template,
                &[
                    ("count", amount(chip.count, kind)),
                    ("firm", chip.firm.clone().unwrap_or_default()),
                    ("unit", unit.clone()),
                ],
            );
            Some(MarketShort {
                id: chip.id.to_string(),
                label,
                count: chip.count,
                entity: chip.entity,
            })
        })
        .collect();
    let drivers = raw
        .drivers
        .iter()
        .filter_map(|(id, value)| {
            Some(MarketDriver {
                id: (*id).to_string(),
                label: def.drivers.get(*id)?.clone(),
                value: value.clone(),
                quantity_id: driver_quantity(id).map(|q| q.as_str().to_string()),
            })
        })
        .collect();
    MarketView {
        id: def.id.clone(),
        label: def.label.clone(),
        category: def.category.clone(),
        mechanisms: def.mechanisms.clone(),
        blurb: def.blurb.clone(),
        unit,
        amount: kind,
        demand: raw.demand,
        supply: raw.supply,
        traded: raw.traded,
        unmet: raw.unmet,
        unsold: raw.unsold,
        demand_label: def.demand_label.clone(),
        supply_label: def.supply_label.clone(),
        traded_label: def.traded_label.clone(),
        demand_text: amount(raw.demand, kind),
        supply_text: amount(raw.supply, kind),
        traded_text: amount(raw.traded, kind),
        gap_words,
        gap_kind: match (short, glut) {
            (true, true) => GapKind::Both,
            (true, false) => GapKind::Short,
            (false, true) => GapKind::Glut,
            (false, false) => GapKind::None,
        },
        verdict,
        price_kind: def.price_kind,
        price_label: def.price_label.clone(),
        price_unit: def.price_unit.clone(),
        price_text: raw.price.and_then(|v| price_text(def.price_kind, v)),
        price: raw.price,
        price_direction: dir.to_string(),
        price_direction_word: word(&c.directions, dir),
        price_cause: def.price_cause.clone(),
        price_cause_word: word(&c.causes, &def.price_cause),
        price_quantity: price_quantity(&def.id).map(|q| q.as_str().to_string()),
        price_note: def.price_note.clone(),
        price_series: series,
        drivers,
        short: shorts,
        how_id: def.how_id.clone(),
        link: def.link.clone(),
        reconciles: raw.ok
            && close(raw.demand, raw.traded + raw.unmet)
            && close(raw.supply, raw.traded + raw.unsold)
            && raw.unmet >= -1e-9
            && raw.unsold >= -1e-9,
    }
}

/// Every market, in content order, for the latest month. Empty until the first month has been
/// played (the figures come from that month's records).
pub fn build(
    sim: &Sim,
    history: &[HistoryPoint],
    log: &WrapperLog,
    content: &MarketsContent,
    energy_unit: &str,
) -> Vec<MarketView> {
    let Some(tick) = log.ticks.last() else {
        return Vec::new();
    };
    let s: &State = &sim.state;
    let names: BTreeMap<u32, String> = crate::snapshot::firm_names(&s.firms)
        .into_iter()
        .zip(&s.firms)
        .map(|(n, f)| (f.id.0, n))
        .collect();
    // The turn is open with a month played: the current month is not yet in the history.
    let live = sim.turn_phase.open && sim.turn_phase.months_done > 0;
    content
        .list
        .iter()
        .filter_map(|def| {
            let raw = raw_for(sim, tick, def, &names, content.fully_met)?;
            Some(view(def, content, raw, history, live, energy_unit))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_read_naturally() {
        assert_eq!(amount(310.0, AmountKind::Count), "310");
        assert_eq!(amount(12_345.6, AmountKind::Count), "12,346");
        assert_eq!(amount(30.04, AmountKind::Count), "30");
        assert_eq!(amount(2.5, AmountKind::Count), "2.5");
        assert_eq!(amount(0.0, AmountKind::Count), "0");
        assert_eq!(amount(1_250.4, AmountKind::Money), "$1,250");
    }

    #[test]
    fn direction_uses_the_steady_band() {
        assert_eq!(direction(&[], 0.01), "unknown");
        assert_eq!(direction(&[4.0], 0.01), "unknown");
        assert_eq!(direction(&[4.0, 4.02], 0.01), "steady");
        assert_eq!(direction(&[4.0, 4.2], 0.01), "rising");
        assert_eq!(direction(&[4.0, 3.5], 0.01), "falling");
    }

    #[test]
    fn templates_fill_every_placeholder() {
        assert_eq!(
            fill("{a} and {b}", &[("a", "1".into()), ("b", "2".into())]),
            "1 and 2"
        );
    }
}
