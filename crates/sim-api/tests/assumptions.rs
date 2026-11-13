//! The model explorer (contract v1.5): the catalog resolves against the live params, every
//! example is recomputed by the model's own functions, and every metric, lever and UI anchor has
//! an entry.
use sim_api::assumptions::{self, Group, ParamValue};
use sim_api::keys::{LeverKey, MetricKey};
use sim_api::{content, Api};
use sim_model::production;
use sim_params::Params;
use std::collections::BTreeSet;

fn topics() -> Vec<assumptions::AssumptionTopic> {
    Api::init(1).unwrap().assumptions().unwrap()
}

#[test]
fn every_param_path_and_example_function_exists_and_resolves() {
    // `resolve` fails on a missing parameter, a missing function or a missing example input.
    let t = topics();
    assert!(t.len() >= 30, "{} topics", t.len());
    let ids: BTreeSet<&str> = t.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids.len(), t.len(), "topic ids are unique");
    for x in &t {
        for r in &x.related {
            assert!(ids.contains(r.as_str()), "{} relates to unknown {r}", x.id);
        }
        assert!(
            !x.summary.is_empty() && !x.formula.is_empty() && !x.code.is_empty(),
            "{}",
            x.id
        );
        for p in &x.params {
            assert!(
                !p.unit.contains("{energy}"),
                "{}: unit not resolved",
                p.path
            );
        }
        for e in &x.examples {
            assert!(e.result.value.is_finite(), "{}: {}", x.id, e.label);
        }
    }
    // The registry names every function the catalog uses and nothing else is unused.
    let reg: BTreeSet<&str> = assumptions::registry().iter().map(|(n, _)| *n).collect();
    let used: BTreeSet<String> = assumptions::load()
        .unwrap()
        .iter()
        .flat_map(|d| d.examples.iter().map(|e| e.function.clone()))
        .collect();
    for f in &used {
        assert!(reg.contains(f.as_str()), "unknown example function {f}");
    }
    for f in &reg {
        assert!(used.contains(*f), "registry function {f} has no example");
    }
}

#[test]
fn all_groups_have_an_entry() {
    let t = topics();
    for g in [
        Group::EmissionsEnergy,
        Group::PeopleHouseholds,
        Group::ProductionPrices,
        Group::NatureClimate,
        Group::Weather,
        Group::GovernmentPolicy,
        Group::FinanceCredit,
        Group::ClassPolitics,
        Group::Scoring,
        Group::LeavesOut,
    ] {
        assert!(t.iter().any(|x| x.group == g), "no topic in {g:?}");
    }
    // The twelve minimum topics of docs/town-v5.md section 10 plus the extras.
    for id in [
        "emissions",
        "energy_power",
        "accumulated_emissions",
        "production_output",
        "household_spending",
        "money_credit",
        "policy_lobbying",
        "class_politics",
        "nature_forest_commons",
        "weather_stress",
        "scoring_outcomes",
        "leaves_out",
        "gdp",
        "gpi",
        "wage_profit_share",
        "debt_gdp",
        "gini",
        "legitimacy",
        "capital_ratio",
        "tax_income",
        "tax_profit",
        "tax_capital",
        "tax_wealth",
        "tax_carbon",
        "tax_gst",
        "energy_units",
        "household_electricity",
    ] {
        assert!(t.iter().any(|x| x.id == id), "missing topic {id}");
    }
}

/// UI anchors outside the metric and lever key sets: panels and rows of the Finance, People,
/// Industry and Energy tabs (ids from `web/src/panels/finance.ts` and the Industry tab).
const ANCHORS: &[&str] = &[
    "panel:economy",
    "panel:ecology",
    "ecology:commons",
    "ecology:forest",
    "panel:society",
    "panel:finance",
    "panel:people",
    "panel:industry",
    "panel:policies",
    "panel:energy",
    "panel:markets",
    "markets:intro",
    "markets:prices",
    "markets:food",
    "markets:goods",
    "markets:services",
    "markets:labour",
    "markets:housing",
    "markets:credit",
    "markets:energy",
    "finance:gov-in",
    "finance:gov-out",
    "finance:gov-bal",
    "finance:gov-debt",
    "finance:gov-dgdp",
    "finance:gov-leg",
    "finance:bank-eq",
    "finance:bank-loans",
    "finance:bank-dep",
    "finance:bank-cr",
    "finance:bank-head",
    "finance:bank-def",
    "finance:bank-int",
    "finance:hh-tax",
    "finance:hh-rent",
    "finance:hh-loan",
    "finance:hh-spend",
    "networth:savings",
    "networth:homes",
    "networth:firm_capital",
    "networth:firm_cash",
    "networth:firm_inventory",
    "networth:bank_equity",
    "networth:jet",
    "networth:loans",
    "networth:firm_loans",
    "networth:holders",
    "finance:hh-save",
    "finance:hh-debt",
    "finance:hh-nw",
    "finance:f-rev",
    "finance:f-wage",
    "finance:f-profit",
    "finance:f-inv",
    "finance:f-debt",
    "finance:f-cash",
    "finance:m-supply",
    "finance:m-created",
    "finance:m-destroyed",
    "finance:p-index",
    "finance:p-wage",
    "finance:p-profit",
    "finance:p-gini",
    "finance:p-median",
    "finance:p-avg",
    "people:wellbeing",
    "people:footprint",
    "people:energy_use",
    "people:class",
    "people:wage",
    "people:commute",
    "industry:firms",
    "industry:ownership",
    "industry:infrastructure",
    "industry:public_goods",
    "energy:served",
    "energy:demand",
    "energy:supply",
    "energy:intensity",
    "energy:renewable_share",
    "energy:plant_emissions",
    "switch_system",
    "place:road",
    "place:public_housing",
    "place:fossil_plant",
    "place:renewable_plant",
    "place:fire_station",
];

const METRICS: &[&str] = &[
    "gdp",
    "gpi",
    "unemployment",
    "gini",
    "wage_share",
    "profit_share",
    "wellbeing",
    "emissions",
    "budget",
    "temperature",
    "legitimacy",
    "debt",
    "money",
    "prices",
    "housing",
    "forest",
    "commons",
    "avg_income",
    "median_income",
    "top_wealth_share",
    "forest_health",
    "commons_health",
    "farm_health",
    "climate_stress",
];

#[test]
fn every_metric_lever_and_anchor_is_covered_and_every_anchor_is_known() {
    let t = topics();
    let used: BTreeSet<&str> = t
        .iter()
        .flat_map(|x| x.used_by.iter().map(String::as_str))
        .collect();
    // The metric list matches the wire enum exactly.
    for m in METRICS {
        serde_json::from_value::<MetricKey>(serde_json::json!(m))
            .unwrap_or_else(|_| panic!("{m} is not a MetricKey"));
    }
    let mut uncovered: Vec<String> = Vec::new();
    for m in METRICS {
        if !used.contains(m) {
            uncovered.push(format!("metric {m}"));
        }
    }
    for l in LeverKey::ALL {
        if !used.contains(l.as_str()) {
            uncovered.push(format!("lever {}", l.as_str()));
        }
    }
    for a in ANCHORS {
        if !used.contains(a) {
            uncovered.push(format!("anchor {a}"));
        }
    }
    assert!(uncovered.is_empty(), "no catalog entry for: {uncovered:?}");
    // Every usedBy is a metric, a lever or a known anchor (no typos).
    let known: BTreeSet<&str> = METRICS
        .iter()
        .copied()
        .chain(LeverKey::ALL.iter().map(|l| l.as_str()))
        .chain(ANCHORS.iter().copied())
        .collect();
    for x in &t {
        for u in &x.used_by {
            assert!(known.contains(u.as_str()), "{}: unknown usedBy '{u}'", x.id);
        }
    }
}

#[test]
fn examples_are_recomputed_by_the_models_own_functions() {
    let params = Params::from_content(&content::default_content()).unwrap();
    let t = topics();
    let ex = |topic: &str, i: usize| {
        let x = t.iter().find(|x| x.id == topic).unwrap();
        x.examples[i].result.value
    };
    // Household electricity: 3 people spending $6,000 a month.
    let want = production::household_energy_use(&params, 1.0, 3.0, 6000.0);
    assert_eq!(ex("household_electricity", 0), want);
    // A drought raises it.
    assert!(ex("household_electricity", 1) > ex("household_electricity", 0));
    // Firm emissions with and without green technology.
    assert_eq!(
        ex("emissions", 0),
        production::firm_direct_emissions(1000.0, 0.012, 0.0)
    );
    assert!(ex("emissions", 1) < ex("emissions", 0));
    assert_eq!(ex("emissions", 2), 5.0 * params.energy.factor_fossil);
    assert_eq!(
        ex("accumulated_emissions", 1),
        params.nature.budget_b0 - 450.0
    );
    assert_eq!(ex("tax_income", 0), 8000.0 * 0.15);
    assert_eq!(ex("gini", 0), sim_metrics::gini(&[10.0, 20.0, 30.0, 40.0]));
}

#[test]
fn values_follow_the_live_params() {
    let mut v: serde_json::Value = serde_json::from_str(content::PARAMS).unwrap();
    v["energy"]["hh_energy_base_pc"] = 0.3.into();
    v["nature"]["budget_b0"] = 1000.0.into();
    let text = v.to_string();
    let c = sim_params::ContentJson {
        params: &text,
        ..content::default_content()
    };
    let api = Api::from_content(&c, content::LEVERS, 1).unwrap();
    let t = api.assumptions().unwrap();
    let base = |topics: &[assumptions::AssumptionTopic]| {
        let x = topics
            .iter()
            .find(|x| x.id == "household_electricity")
            .unwrap();
        (
            x.params
                .iter()
                .find(|p| p.path == "energy.hh_energy_base_pc")
                .unwrap()
                .value
                .clone(),
            x.examples[0].result.value,
        )
    };
    let (val, res) = base(&t);
    assert_eq!(val, ParamValue::Number(0.3));
    let (val0, res0) = base(&topics());
    assert_eq!(val0, ParamValue::Number(0.1));
    assert!(res > res0);
    let acc = t.iter().find(|x| x.id == "accumulated_emissions").unwrap();
    assert_eq!(acc.examples[1].result.value, 1000.0 - 450.0);
    // The energy unit label comes from content.
    let hh = t.iter().find(|x| x.id == "household_electricity").unwrap();
    assert_eq!(hh.examples[0].result.unit, "MWh per month");
}

#[test]
fn wire_shape_is_camel_case_with_snake_case_enums() {
    let t = topics();
    let j = serde_json::to_value(&t[0]).unwrap();
    for k in [
        "id",
        "title",
        "group",
        "summary",
        "formula",
        "params",
        "examples",
        "status",
        "statusNote",
        "leavesOut",
        "code",
        "related",
        "usedBy",
    ] {
        assert!(j.get(k).is_some(), "missing {k}");
    }
    assert!(matches!(
        j["status"].as_str(),
        Some("placeholder" | "tuned" | "rough_real_world")
    ));
    assert!(j["group"].as_str().unwrap().contains('_') || !j["group"].as_str().unwrap().is_empty());
    assert!(j["examples"][0]["result"]["unit"].is_string());
}

// ---- the agents catalog (contract v1.7) ----

fn agents() -> assumptions::AgentsCatalog {
    Api::init(1).unwrap().agents().unwrap()
}

#[test]
fn agents_catalog_resolves_and_covers_every_kind() {
    let a = agents();
    let ids: Vec<&str> = a.kinds.iter().map(|k| k.id.as_str()).collect();
    for want in [
        "households",
        "firms",
        "bank",
        "government",
        "lobbyists",
        "nature",
        "weather",
    ] {
        assert!(ids.contains(&want), "missing kind {want}");
    }
    for k in &a.kinds {
        assert!(
            (!k.decides.is_empty() || !k.notes.is_empty())
                && !k.observes.is_empty()
                && !k.rule.is_empty()
        );
    }
    let classes: Vec<&str> = a.kinds[0].variants.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(
        classes,
        ["capitalist", "rentier", "manager", "worker", "dependent"]
    );
    assert!(!a.deterministic.is_empty());
}

/// Every random draw in the model is in the table: the count of draw sites in the source equals
/// the draws the catalog documents (the drought size entry covers two uniform draws), and each
/// entry names a real file.
#[test]
fn randomness_table_matches_the_draw_sites_in_the_model() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sites = 0;
    for f in std::fs::read_dir(root.join("crates/sim-model/src")).unwrap() {
        let src = std::fs::read_to_string(f.unwrap().path()).unwrap();
        for line in src.lines().filter(|l| !l.trim_start().starts_with("//")) {
            for pat in ["r.chance(", "r.next_f64(", "r.below("] {
                sites += line.matches(pat).count();
            }
        }
    }
    let a = agents();
    let documented: usize = a
        .randomness
        .iter()
        .map(|d| if d.id == "drought_size" { 2 } else { 1 })
        .sum();
    assert_eq!(
        sites, documented,
        "a draw was added or removed: update the table"
    );
    // Town v7 added one draw site (the estate's monthly flight, `estate.rs`): 10 in all (9 before).
    assert_eq!(sites, 10, "draw sites in sim-model");
    // Weather draws use the weather's own stream (stored in the state), everything else the
    // shared one; the table says which, and the weather system must not touch the shared stream.
    for d in &a.randomness {
        let want = if d.system == "weather" {
            "weather"
        } else {
            "shared"
        };
        assert_eq!(d.stream, want, "{}: stream", d.id);
    }
    let wsrc = std::fs::read_to_string(root.join("crates/sim-model/src/weather.rs")).unwrap();
    assert!(
        wsrc.contains("_shared: &mut Rng"),
        "weather::run must ignore the shared stream"
    );
    assert!(
        a.randomness.iter().any(|d| d.id == "estate_flight"),
        "the estate flight draw is documented"
    );
    for d in &a.randomness {
        assert!(root.join(&d.code).exists(), "{}: {}", d.id, d.code);
        assert!(
            d.when.len() > 5 && d.probability.len() > 5,
            "{} is thin",
            d.id
        );
    }
}
