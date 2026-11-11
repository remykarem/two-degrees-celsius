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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CrisisBands {
    pub legitimacy_below: f64,
    pub budget_below: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthBand {
    /// A value at or above `min` is in this band; bands run from the highest `min` down.
    pub min: f64,
    pub id: String,
    pub word: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordBand {
    pub below: f64,
    pub word: String,
}

/// A value below `bands[i].below` gets that band's word (first match); otherwise `top`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordBands {
    pub bands: Vec<WordBand>,
    pub top: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeterBands {
    pub low: f64,
    pub high: f64,
}

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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeverGroupInfo {
    pub label: String,
    pub blurb: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemParam {
    pub path: String,
    pub label: String,
    pub value: f64,
    /// `fraction`, `x` or `months`.
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCard {
    pub key: String,
    /// `params.systems.<key>.name`.
    pub name: String,
    pub tagline: String,
    pub facts: Vec<Fact>,
    /// Numbers read from `params.systems.<key>`.
    pub params: Vec<SystemParam>,
    pub transition_months: f64,
    pub output_penalty: f64,
    pub resistance: String,
    pub resistance_placeholder: bool,
    pub tradeoffs: Vec<Fact>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub bands: Bands,
    pub lever_groups: Vec<LeverGroupInfo>,
    /// Authored text by group, then key (see `content/ui.json`). `{name}` placeholders are filled by the web.
    pub text: BTreeMap<String, BTreeMap<String, String>>,
    pub systems: Vec<SystemCard>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CardDef {
    key: String,
    tagline: String,
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
    text: BTreeMap<String, BTreeMap<String, String>>,
    system_cards: Vec<CardDef>,
    system_param_labels: BTreeMap<String, ParamLabel>,
}

fn bad(msg: String) -> ApiError {
    ApiError::new(code::BAD_METADATA, format!("content/ui.json: {msg}"))
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
            facts: c.facts,
            params: ps,
            resistance: c.resistance,
            resistance_placeholder: c.resistance_placeholder,
            tradeoffs: c.tradeoffs,
        });
    }
    Ok(Catalog {
        bands: f.bands,
        lever_groups: f.lever_groups,
        text: f.text,
        systems,
    })
}
