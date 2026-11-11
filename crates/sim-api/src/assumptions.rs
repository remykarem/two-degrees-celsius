//! The model explorer (contract v1.5): `content/assumptions.json` is the authored catalog; this
//! module resolves it against the live `Params` and computes every worked example with the
//! model's own functions, through the registry below (one implementation of each formula).
//! Text is authored; numbers are never copied into text: they come from `params` and `examples`.
use crate::error::{code, ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use sim_params::Params;
use sim_types::Cents;
use std::collections::BTreeMap;

/// The catalog, embedded at build time.
pub const CATALOG: &str = include_str!("../../../content/assumptions.json");

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    EmissionsEnergy,
    PeopleHouseholds,
    ProductionPrices,
    NatureClimate,
    Weather,
    GovernmentPolicy,
    FinanceCredit,
    ClassPolitics,
    Scoring,
    LeavesOut,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Placeholder,
    Tuned,
    RoughRealWorld,
}

// ---- the authored file ----

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamRef {
    pub path: String,
    pub label: String,
    pub unit: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExampleDef {
    /// Name in the registry ([`registry`]).
    pub function: String,
    pub label: String,
    pub inputs: BTreeMap<String, f64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TopicDef {
    pub id: String,
    pub title: String,
    pub group: Group,
    pub summary: String,
    pub formula: String,
    pub params: Vec<ParamRef>,
    pub examples: Vec<ExampleDef>,
    pub status: Status,
    pub status_note: String,
    pub leaves_out: Vec<String>,
    pub code: Vec<String>,
    pub related: Vec<String>,
    pub used_by: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogFile {
    topics: Vec<TopicDef>,
    agents: AgentsDef,
}

// ---- the agents section (contract v1.7) ----

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Agent,
    NonAgent,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Distribution {
    Bernoulli,
    Uniform,
    UniformIndex,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct KindDef {
    pub id: String,
    pub title: String,
    pub actor: Actor,
    pub summary: String,
    pub decides: Vec<String>,
    pub observes: Vec<String>,
    pub rule: String,
    pub params: Vec<ParamRef>,
    #[serde(default)]
    pub variants: Vec<AgentVariant>,
    #[serde(default)]
    pub class_params: Vec<ParamRef>,
    pub code: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawDef {
    pub id: String,
    pub event: String,
    pub system: String,
    pub distribution: Distribution,
    pub probability: String,
    pub when: String,
    pub params: Vec<ParamRef>,
    pub code: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentsDef {
    pub kinds: Vec<KindDef>,
    pub randomness: Vec<DrawDef>,
    pub deterministic: Vec<String>,
}

/// A household class: the derived label, a one-line account of how such households decide, and
/// the rule that makes the label.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentVariant {
    /// The wire class key (`capitalist`, `rentier`, `manager`, `worker`, `dependent`).
    pub id: String,
    pub label: String,
    pub decision: String,
    pub rule: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentKindInfo {
    pub id: String,
    pub title: String,
    pub actor: Actor,
    pub summary: String,
    pub decides: Vec<String>,
    pub observes: Vec<String>,
    pub rule: String,
    pub params: Vec<AssumptionParam>,
    pub variants: Vec<AgentVariant>,
    pub class_params: Vec<AssumptionParam>,
    pub code: Vec<String>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RandomDraw {
    pub id: String,
    pub event: String,
    pub system: String,
    pub distribution: Distribution,
    pub probability: String,
    pub when: String,
    pub params: Vec<AssumptionParam>,
    pub code: String,
}

/// The agents catalog (contract v1.7): who decides what, how, and where chance enters.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentsCatalog {
    pub kinds: Vec<AgentKindInfo>,
    pub randomness: Vec<RandomDraw>,
    pub deterministic: Vec<String>,
}

fn load_file() -> ApiResult<CatalogFile> {
    serde_json::from_str::<CatalogFile>(CATALOG)
        .map_err(|e| ApiError::new(code::BAD_METADATA, format!("content/assumptions.json: {e}")))
}

fn resolve_refs(
    pj: &serde_json::Value,
    refs: &[ParamRef],
    energy_unit: &str,
    who: &str,
) -> ApiResult<Vec<AssumptionParam>> {
    refs.iter()
        .map(|r| {
            let value = param_value(pj, &r.path).ok_or_else(|| {
                ApiError::new(
                    code::BAD_METADATA,
                    format!("agents '{who}': no parameter {}", r.path),
                )
            })?;
            Ok(AssumptionParam {
                path: r.path.clone(),
                label: r.label.clone(),
                value,
                unit: r.unit.replace("{energy}", energy_unit),
            })
        })
        .collect()
}

/// Resolve the agents section against the live params.
pub fn resolve_agents(params: &Params, energy_unit: &str) -> ApiResult<AgentsCatalog> {
    let a = load_file()?.agents;
    let pj = serde_json::to_value(params)
        .map_err(|e| ApiError::new(code::BAD_METADATA, format!("params: {e}")))?;
    let kinds = a
        .kinds
        .iter()
        .map(|k| {
            Ok(AgentKindInfo {
                id: k.id.clone(),
                title: k.title.clone(),
                actor: k.actor,
                summary: k.summary.clone(),
                decides: k.decides.clone(),
                observes: k.observes.clone(),
                rule: k.rule.clone(),
                params: resolve_refs(&pj, &k.params, energy_unit, &k.id)?,
                variants: k.variants.clone(),
                class_params: resolve_refs(&pj, &k.class_params, energy_unit, &k.id)?,
                code: k.code.clone(),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    let randomness = a
        .randomness
        .iter()
        .map(|r| {
            Ok(RandomDraw {
                id: r.id.clone(),
                event: r.event.clone(),
                system: r.system.clone(),
                distribution: r.distribution,
                probability: r.probability.clone(),
                when: r.when.clone(),
                params: resolve_refs(&pj, &r.params, energy_unit, &r.id)?,
                code: r.code.clone(),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(AgentsCatalog {
        kinds,
        randomness,
        deterministic: a.deterministic,
    })
}

pub fn load() -> ApiResult<Vec<TopicDef>> {
    load_file().map(|f| f.topics)
}

// ---- the wire ----

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    Number(f64),
    Bool(bool),
    Text(String),
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssumptionParam {
    pub path: String,
    pub label: String,
    pub value: ParamValue,
    pub unit: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExampleResult {
    pub value: f64,
    pub unit: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssumptionExample {
    pub label: String,
    pub inputs: BTreeMap<String, f64>,
    pub result: ExampleResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    /// Additive: the registry name of the model function that computed the result.
    pub function: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssumptionTopic {
    pub id: String,
    pub title: String,
    pub group: Group,
    pub summary: String,
    pub formula: String,
    pub params: Vec<AssumptionParam>,
    pub examples: Vec<AssumptionExample>,
    pub status: Status,
    pub status_note: String,
    pub leaves_out: Vec<String>,
    pub code: Vec<String>,
    pub related: Vec<String>,
    pub used_by: Vec<String>,
}

/// The value at a dotted path of the serialised params.
pub fn param_value(params: &serde_json::Value, path: &str) -> Option<ParamValue> {
    let mut cur = params;
    for k in path.split('.') {
        cur = cur.get(k)?;
    }
    match cur {
        serde_json::Value::Number(n) => n.as_f64().map(ParamValue::Number),
        serde_json::Value::Bool(b) => Some(ParamValue::Bool(*b)),
        serde_json::Value::String(s) => Some(ParamValue::Text(s.clone())),
        _ => None,
    }
}

// ---- the example registry ----

/// One example function: the params, the inputs by name and the energy unit label give the
/// result and its unit.
pub type ExampleFn = fn(&Params, &Inputs, &str) -> Option<(f64, String)>;

pub struct Inputs<'a>(&'a BTreeMap<String, f64>);

impl Inputs<'_> {
    fn get(&self, k: &str) -> Option<f64> {
        self.0.get(k).copied()
    }
}

fn usd(c: Cents) -> Option<(f64, String)> {
    Some((c.dollars(), "$".to_string()))
}

/// Every example function, by name. Each calls the model or metrics function the simulation
/// itself uses.
pub fn registry() -> Vec<(&'static str, ExampleFn)> {
    use sim_model::{government, nature, production, util};
    vec![
        ("household_energy_use", |p, i, u| {
            let v = production::household_energy_use(
                p,
                i.get("heat")?,
                i.get("people")?,
                i.get("consumption_usd")?,
            );
            Some((v, format!("{u} per month")))
        }),
        ("firm_energy_use", |p, i, u| {
            let _ = p;
            let per = production::firm_energy_per_unit(
                i.get("energy_per_unit")?,
                i.get("green_tech")?,
                i.get("heat")?,
            );
            Some((i.get("output")? * per, format!("{u} per month")))
        }),
        ("firm_direct_emissions", |_, i, _| {
            let v = production::firm_direct_emissions(
                i.get("output")?,
                i.get("intensity_base")?,
                i.get("green_tech")?,
            );
            Some((v, "t CO2 per month".to_string()))
        }),
        ("plant_emissions_fossil", |p, i, _| {
            let v =
                production::plant_emissions(p, sim_types::PlantKind::Fossil, i.get("delivered")?);
            Some((v, "t CO2 per month".to_string()))
        }),
        ("grid_intensity", |p, i, u| {
            let v = production::grid_intensity_of(
                p,
                i.get("fossil_capacity")?,
                i.get("total_capacity")?,
            );
            Some((v, format!("t CO2 per {u}")))
        }),
        ("forest_sink", |p, i, _| {
            Some((
                nature::forest_sink(p, i.get("forest")?),
                "t CO2 per month".to_string(),
            ))
        }),
        ("cobb_douglas", |p, i, _| {
            let v = production::cobb_douglas(p, i.get("tfp")?, i.get("capital")?, i.get("labour")?);
            Some((v, "units per month".to_string()))
        }),
        ("temperature", |p, i, _| {
            let v = sim_metrics::temperature(sim_types::Tonnes(i.get("cumulative")?), p);
            Some((v, "C".to_string()))
        }),
        ("remaining_budget", |p, i, _| {
            let v = sim_metrics::remaining_budget(sim_types::Tonnes(i.get("cumulative")?), p);
            Some((v.0, "t CO2".to_string()))
        }),
        ("climate_stress", |p, i, _| {
            let v = sim_metrics::climate_stress(sim_types::Tonnes(i.get("cumulative")?), p);
            Some((v, "0 to 1".to_string()))
        }),
        ("gini_of_four", |_, i, _| {
            let v = sim_metrics::gini(&[i.get("a")?, i.get("b")?, i.get("c")?, i.get("d")?]);
            Some((v, "0 to 1".to_string()))
        }),
        ("gpi_annual", |p, i, _| {
            let v = sim_metrics::gpi_annual(
                p,
                i.get("gdp_annual")?,
                i.get("gini")?,
                i.get("gross_emissions_month")?,
                i.get("depletion_month")?,
            );
            Some((v, "$ per year".to_string()))
        }),
        ("debt_to_gdp", |_, i, _| {
            let v = sim_metrics::debt_to_gdp(i.get("debt")?, i.get("gdp_annual")?);
            Some((v, "ratio".to_string()))
        }),
        ("legitimacy_change", |p, i, _| {
            let (w, u, g, d) = government::legitimacy_terms(
                &p.government,
                i.get("wellbeing")?,
                i.get("unemployment")?,
                i.get("gini")?,
                i.get("debt_to_gdp")?,
            );
            Some((w - u - g - d, "legitimacy points per month".to_string()))
        }),
        ("income_tax", |_, i, _| {
            usd(util::tax_on(
                Cents::from_dollars(i.get("base")?),
                i.get("rate")?,
            ))
        }),
        ("profit_tax", |_, i, _| {
            usd(util::profit_tax_due(
                Cents::from_dollars(i.get("pre_tax_profit")?),
                i.get("rate")?,
            ))
        }),
        ("wealth_tax_monthly", |_, i, _| {
            usd(util::wealth_tax_monthly(
                i.get("net_worth")? * 100.0,
                Cents::from_dollars(i.get("threshold")?),
                i.get("annual_rate")?,
            ))
        }),
        ("carbon_cost", |_, i, _| {
            usd(util::carbon_cost(
                Cents::from_dollars(i.get("price")?),
                i.get("footprint")?,
            ))
        }),
        ("max_loans", |_, i, _| {
            usd(util::max_loans(
                Cents::from_dollars(i.get("equity")?),
                i.get("capital_ratio")?,
            ))
        }),
    ]
}

/// Resolve the catalog against the live params.
pub fn resolve(params: &Params, energy_unit: &str) -> ApiResult<Vec<AssumptionTopic>> {
    let defs = load()?;
    let reg = registry();
    let pj = serde_json::to_value(params)
        .map_err(|e| ApiError::new(code::BAD_METADATA, format!("params: {e}")))?;
    defs.into_iter()
        .map(|d| {
            let bad = |m: String| {
                ApiError::new(code::BAD_METADATA, format!("assumptions '{}': {m}", d.id))
            };
            let mut ps = Vec::new();
            for r in &d.params {
                let value = param_value(&pj, &r.path)
                    .ok_or_else(|| bad(format!("no parameter {}", r.path)))?;
                ps.push(AssumptionParam {
                    path: r.path.clone(),
                    label: r.label.clone(),
                    value,
                    unit: r.unit.replace("{energy}", energy_unit),
                });
            }
            let mut exs = Vec::new();
            for e in &d.examples {
                let f = reg
                    .iter()
                    .find(|(n, _)| *n == e.function)
                    .ok_or_else(|| bad(format!("no example function {}", e.function)))?
                    .1;
                let (value, unit) = f(params, &Inputs(&e.inputs), energy_unit)
                    .ok_or_else(|| bad(format!("example '{}' is missing an input", e.label)))?;
                exs.push(AssumptionExample {
                    label: e.label.clone(),
                    inputs: e.inputs.clone(),
                    result: ExampleResult { value, unit },
                    note: e.note.clone(),
                    function: e.function.clone(),
                });
            }
            Ok(AssumptionTopic {
                id: d.id.clone(),
                title: d.title.clone(),
                group: d.group,
                summary: d.summary.clone(),
                formula: d.formula.clone(),
                params: ps,
                examples: exs,
                status: d.status,
                status_note: d.status_note.clone(),
                leaves_out: d.leaves_out.clone(),
                code: d.code.clone(),
                related: d.related.clone(),
                used_by: d.used_by.clone(),
            })
        })
        .collect()
}
