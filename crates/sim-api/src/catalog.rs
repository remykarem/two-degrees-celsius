//! The UI catalog (contract v1.8): every threshold, label and explanatory sentence the web used
//! to hard-code, authored in `content/ui.json` and served over the wire. The web renders it and
//! keeps only layout, formatting, animation and wording of wire ids (coach text, event wording).
//! System cards are resolved against the live `Params`, so a number in a card is the number the
//! model uses.
use crate::error::{code, ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use sim_params::Params;
use std::collections::BTreeMap;

/// The file, embedded at build time.
pub const UI: &str = include_str!("../../../content/ui.json");

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CrisisBands {
    pub legitimacy_below: f64,
    pub budget_below: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthBand {
    /// A value at or above `min` is in this band; bands run from the highest `min` down.
    pub min: f64,
    pub id: String,
    pub word: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordBand {
    pub below: f64,
    pub word: String,
}

/// A value below `bands[i].below` gets that band's word (first match); otherwise `top`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordBands {
    pub bands: Vec<WordBand>,
    pub top: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeterBands {
    pub low: f64,
    pub high: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Bands {
    pub crisis: CrisisBands,
    pub health: Vec<HealthBand>,
    pub stress_words: WordBands,
    pub severity_words: WordBands,
    /// The cap sent with `commons_access: community` (the wire gives no default).
    pub community_cap: f64,
    /// The bank counts as "tight" below `limit x` this factor.
    pub capital_tight_factor: f64,
    pub budget_meter: MeterBands,
    /// A building's table cost and the simulation's preview disagree when they differ by more
    /// than this share (and by more than `estimate_tolerance_usd`).
    pub estimate_tolerance_share: f64,
    pub estimate_tolerance_usd: f64,
    /// Money flows below this many dollars a year are left out of the flow diagram.
    pub flow_min_usd: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeverGroupInfo {
    pub label: String,
    pub blurb: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub label: String,
    pub value: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemParam {
    pub path: String,
    pub label: String,
    pub value: f64,
    /// `fraction`, `x` or `months`.
    pub unit: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCard {
    #[cfg_attr(feature = "ts", ts(type = "SystemKey"))]
    pub key: String,
    /// `params.systems.<key>.name`.
    pub name: String,
    pub tagline: String,
    /// Contract v2.3: which coordination mechanisms this system leans on, in a sentence.
    pub coordination: String,
    pub facts: Vec<Fact>,
    /// Numbers read from `params.systems.<key>`.
    pub params: Vec<SystemParam>,
    pub transition_months: f64,
    pub output_penalty: f64,
    pub resistance: String,
    pub resistance_placeholder: bool,
    pub tradeoffs: Vec<Fact>,
}

/// A wealth band (contract v1.9): a household whose net worth is at or above `min` (the first
/// match, from the highest `min` down) is in it; `min: null` catches everything below.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WealthBand {
    pub min: Option<f64>,
    pub id: String,
    pub label: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    /// Contract v1.9: short tags by key (`estate_owner`).
    pub tags: BTreeMap<String, String>,
    /// Contract v1.9: wealth bands, richest first.
    pub wealth_bands: Vec<WealthBand>,
    pub bands: Bands,
    pub lever_groups: Vec<LeverGroupInfo>,
    /// Authored text by group, then key (see `content/ui.json`). `{name}` placeholders are filled by the web.
    pub text: BTreeMap<String, BTreeMap<String, String>>,
    pub systems: Vec<SystemCard>,
    /// Contract v2.0: labels and categories of the emission sources (lines of `breakdowns.emissions`).
    pub emission_sources: Vec<crate::sources::EmissionSourceDef>,
    /// Contract v2.0: labels and categories of the income sources (household income lists).
    pub income_sources: Vec<crate::sources::IncomeSourceDef>,
    /// Contract v2.2: the lines of a net worth statement (labels, asset or liability, "how?" anchors).
    pub net_worth_lines: Vec<crate::networth::NetWorthLineDef>,
    /// Contract v2.2: the holder groups of the town's net worth.
    pub net_worth_holders: Vec<crate::networth::NetWorthHolderDef>,
    /// Contract v2.3: the Markets view's cards, categories, cause words and coordination mechanisms.
    pub markets: crate::markets::MarketsContent,
    /// Contract v2.1: the entity registry (`content/entities.json`).
    pub entities: Vec<crate::entities::EntityKind>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CardDef {
    key: String,
    tagline: String,
    coordination: String,
    facts: Vec<Fact>,
    params: Vec<String>,
    resistance: String,
    resistance_placeholder: bool,
    tradeoffs: Vec<Fact>,
}

#[derive(Deserialize)]
struct ParamLabel {
    label: String,
    unit: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UiFile {
    bands: Bands,
    lever_groups: Vec<LeverGroupInfo>,
    tags: BTreeMap<String, String>,
    wealth_bands: Vec<WealthBand>,
    text: BTreeMap<String, BTreeMap<String, String>>,
    system_cards: Vec<CardDef>,
    system_param_labels: BTreeMap<String, ParamLabel>,
    emission_sources: Vec<crate::sources::EmissionSourceDef>,
    income_sources: Vec<crate::sources::IncomeSourceDef>,
    net_worth_lines: Vec<crate::networth::NetWorthLineDef>,
    net_worth_holders: Vec<crate::networth::NetWorthHolderDef>,
    markets: crate::markets::MarketsContent,
}

fn bad(msg: String) -> ApiError {
    ApiError::new(code::BAD_METADATA, format!("content/ui.json: {msg}"))
}

/// The emission and income source definitions of `content/ui.json`, validated.
pub fn sources() -> ApiResult<crate::sources::Sources> {
    let f: UiFile = serde_json::from_str(UI).map_err(|e| bad(e.to_string()))?;
    let net_worth = crate::networth::NetWorthDefs::new(f.net_worth_lines, f.net_worth_holders)?;
    crate::sources::Sources::new(f.emission_sources, f.income_sources, net_worth, f.markets)
}

/// Parse `content/ui.json` and resolve the system cards against `params`.
pub fn resolve(params: &Params) -> ApiResult<Catalog> {
    let f: UiFile = serde_json::from_str(UI).map_err(|e| bad(e.to_string()))?;
    let pj = serde_json::to_value(&params.systems).map_err(|e| bad(e.to_string()))?;
    let num = |sys: &str, name: &str| -> ApiResult<f64> {
        pj.get(sys)
            .and_then(|s| s.get(name))
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| bad(format!("no numeric parameter systems.{sys}.{name}")))
    };
    let mut systems = Vec::new();
    for c in f.system_cards {
        let name = pj
            .get(&c.key)
            .and_then(|s| s.get("name"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| bad(format!("no system {}", c.key)))?
            .to_string();
        let mut ps = Vec::new();
        for path in &c.params {
            let (sys, field) = path
                .split_once('.')
                .ok_or_else(|| bad(format!("bad parameter path {path}")))?;
            let l = f
                .system_param_labels
                .get(field)
                .ok_or_else(|| bad(format!("no label for {field}")))?;
            ps.push(SystemParam {
                path: format!("systems.{path}"),
                label: l.label.clone(),
                value: num(sys, field)?,
                unit: l.unit.clone(),
            });
        }
        systems.push(SystemCard {
            transition_months: num(&c.key, "transition_months")?,
            output_penalty: num(&c.key, "transition_output_penalty")?,
            key: c.key,
            name,
            tagline: c.tagline,
            coordination: c.coordination,
            facts: c.facts,
            params: ps,
            resistance: c.resistance,
            resistance_placeholder: c.resistance_placeholder,
            tradeoffs: c.tradeoffs,
        });
    }
    let nw = crate::networth::NetWorthDefs::new(f.net_worth_lines, f.net_worth_holders)?;
    if !f.tags.contains_key("estate_owner") {
        return Err(bad("tags.estate_owner is missing".into()));
    }
    if f.wealth_bands.is_empty() || f.wealth_bands.last().is_some_and(|b| b.min.is_some()) {
        return Err(bad(
            "wealth_bands must end with a band whose min is null".into()
        ));
    }
    Ok(Catalog {
        tags: f.tags,
        wealth_bands: f.wealth_bands,
        bands: f.bands,
        lever_groups: f.lever_groups,
        text: f.text,
        systems,
        emission_sources: f.emission_sources,
        income_sources: f.income_sources,
        net_worth_lines: nw.lines,
        net_worth_holders: nw.holders,
        markets: f.markets.validated()?,
        entities: crate::entities::load()?,
    })
}
