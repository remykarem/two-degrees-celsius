//! The net worth statement (Town v8, contract v2.2).
//!
//! Every household's net worth is shown as a list of lines, assets minus liabilities, computed in
//! Rust by `sim_metrics::household_wealth_lines` (the same rules as `household_wealth`, the figure
//! behind the Gini and the Lorenz curve). Each statement carries a `reconciles` flag: the lines
//! add up to the headline net worth of that household. The town view splits the town's net worth
//! by holder and reconciles to the sum of the household figures.
//!
//! Ids are a closed set the model knows how to total (`LINE_IDS`, `HOLDER_IDS`); labels,
//! categories and "how?" anchors live in `content/ui.json` (`net_worth_lines`,
//! `net_worth_holders`) and are checked at load, like the emission and income sources.
use crate::error::{code, ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use sim_metrics::WealthLines;

/// Line ids of a household statement, in display order of the wire's own sums.
pub const LINE_IDS: [&str; 9] = [
    "savings",
    "homes",
    "firm_capital",
    "firm_cash",
    "firm_inventory",
    "bank_equity",
    "jet",
    "loans",
    "firm_loans",
];
/// Holders of the town view. `government` is a memo row: it is outside the household headline.
pub const HOLDER_IDS: [&str; 5] = ["households", "firm_owners", "bank", "estate", "government"];

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetWorthCategory {
    Asset,
    Liability,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NetWorthLineDef {
    pub id: String,
    pub label: String,
    pub category: NetWorthCategory,
    /// The explorer anchor behind this line's "how?" (a `usedBy` entry of an assumptions topic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub how_id: Option<String>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NetWorthHolderDef {
    pub id: String,
    pub label: String,
    /// One sentence on who is in the group.
    pub note: String,
    /// False for a memo row that is not part of household net worth (the government).
    pub in_headline: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub how_id: Option<String>,
}

/// One line of a statement, in dollars. `perPerson` is the line divided by the household's people
/// (the Gini gives each person an equal share).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthLine {
    pub id: String,
    pub label: String,
    pub category: NetWorthCategory,
    pub value: f64,
    pub per_person: f64,
    /// How many homes or firms the line stands for, where that applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub how_id: Option<String>,
}

/// A net worth statement: assets minus liabilities, every line listed. Zero lines are left out.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthStatement {
    pub lines: Vec<NetWorthLine>,
    pub assets: f64,
    pub liabilities: f64,
    pub net_worth: f64,
    pub people: u32,
    /// The household figure the Gini uses (`household_wealth`); the lines add up to it.
    pub headline: f64,
    pub reconciles: bool,
}

/// One holder group of the town view.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthHolder {
    pub id: String,
    pub label: String,
    pub note: String,
    pub in_headline: bool,
    pub households: u32,
    pub people: u32,
    pub assets: f64,
    pub liabilities: f64,
    pub net_worth: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub how_id: Option<String>,
}

/// The town's net worth by holder. The holders that are in the headline add up to `headline`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TownNetWorth {
    pub holders: Vec<NetWorthHolder>,
    /// Sum of the households' net worth (the denominator of the Gini).
    pub headline: f64,
    /// Sum of the holders marked `inHeadline`.
    pub holders_total: f64,
    pub reconciles: bool,
}

/// The validated definitions from `content/ui.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct NetWorthDefs {
    pub lines: Vec<NetWorthLineDef>,
    pub holders: Vec<NetWorthHolderDef>,
}

fn bad(msg: String) -> ApiError {
    ApiError::new(code::BAD_METADATA, format!("content/ui.json: {msg}"))
}

impl NetWorthDefs {
    pub fn new(lines: Vec<NetWorthLineDef>, holders: Vec<NetWorthHolderDef>) -> ApiResult<Self> {
        for id in LINE_IDS {
            if lines.iter().filter(|d| d.id == id).count() != 1 {
                return Err(bad(format!("net_worth_lines must list {id} exactly once")));
            }
        }
        if let Some(d) = lines.iter().find(|d| !LINE_IDS.contains(&d.id.as_str())) {
            return Err(bad(format!("unknown net worth line {}", d.id)));
        }
        for id in HOLDER_IDS {
            if holders.iter().filter(|d| d.id == id).count() != 1 {
                return Err(bad(format!(
                    "net_worth_holders must list {id} exactly once"
                )));
            }
        }
        if let Some(d) = holders
            .iter()
            .find(|d| !HOLDER_IDS.contains(&d.id.as_str()))
        {
            return Err(bad(format!("unknown net worth holder {}", d.id)));
        }
        Ok(Self { lines, holders })
    }
}

/// Reconciliation tolerance in dollars: float rounding only (1e-9 of the largest magnitude).
fn tolerance(values: &[f64]) -> f64 {
    1e-9 * values.iter().fold(1.0_f64, |a, v| a.max(v.abs()))
}

fn value_of(l: &WealthLines, id: &str) -> f64 {
    match id {
        "savings" => l.savings,
        "homes" => l.homes,
        "firm_capital" => l.firm_capital,
        "firm_cash" => l.firm_cash,
        "firm_inventory" => l.firm_inventory,
        "bank_equity" => l.bank_equity,
        "jet" => l.jet,
        "loans" => l.loans,
        "firm_loans" => l.firm_loans,
        _ => 0.0,
    }
}

fn count_of(l: &WealthLines, id: &str) -> Option<u32> {
    match id {
        "homes" => Some(l.homes_count),
        "firm_capital" | "firm_cash" | "firm_inventory" | "firm_loans" => Some(l.firms_count),
        _ => None,
    }
}

/// The statement of one household. `headline` is its `household_wealth`, in cents.
pub fn statement(
    defs: &NetWorthDefs,
    l: &WealthLines,
    people: u32,
    headline: f64,
) -> NetWorthStatement {
    let n = f64::from(people.max(1));
    let lines: Vec<NetWorthLine> = defs
        .lines
        .iter()
        .filter_map(|d| {
            let v = value_of(l, &d.id) / 100.0;
            (v != 0.0).then(|| NetWorthLine {
                id: d.id.clone(),
                label: d.label.clone(),
                category: d.category,
                value: v,
                per_person: v / n,
                count: count_of(l, &d.id).filter(|c| *c > 0),
                how_id: d.how_id.clone(),
            })
        })
        .collect();
    let sum = |c: NetWorthCategory| -> f64 {
        lines
            .iter()
            .filter(|x| x.category == c)
            .map(|x| x.value)
            .sum()
    };
    let assets = sum(NetWorthCategory::Asset);
    let liabilities = sum(NetWorthCategory::Liability);
    let net_worth = assets - liabilities;
    let head = headline / 100.0;
    NetWorthStatement {
        reconciles: (net_worth - head).abs() <= tolerance(&[net_worth, head, assets, liabilities]),
        lines,
        assets,
        liabilities,
        net_worth,
        people,
        headline: head,
    }
}

/// Which holder group a household is in: the estate, the bank's owner, an owner of a firm, or the rest.
pub fn holder_of(
    s: &sim_types::State,
    h: sim_types::HouseholdId,
    firms_count: u32,
) -> &'static str {
    if s.estate.as_ref().is_some_and(|e| e.household == h) {
        "estate"
    } else if s.bank.owner == h {
        "bank"
    } else if firms_count > 0 {
        "firm_owners"
    } else {
        "households"
    }
}

/// The town view. `holder[i]` names the group of household `i`; the memo rows come from the state.
pub fn town(
    defs: &NetWorthDefs,
    s: &sim_types::State,
    p: &sim_params::Params,
    statements: &[NetWorthStatement],
    holder: &[&'static str],
    headline_total: f64,
) -> TownNetWorth {
    let state_equity: f64 = s
        .firms
        .iter()
        .filter(|f| f.active && f.owner == sim_types::Owner::State)
        .map(|f| sim_metrics::firm_books(f, p).equity())
        .sum::<f64>()
        / 100.0;
    let holders: Vec<NetWorthHolder> = defs
        .holders
        .iter()
        .map(|d| {
            let (mut households, mut people, mut assets, mut liabilities) = (0, 0, 0.0, 0.0);
            if d.id == "government" {
                assets = s.gov.cash.dollars() + state_equity.max(0.0);
                liabilities = s.gov.debt.dollars() + (-state_equity).max(0.0);
            } else {
                for (st, h) in statements.iter().zip(holder) {
                    if *h == d.id {
                        households += 1;
                        people += st.people;
                        assets += st.assets;
                        liabilities += st.liabilities;
                    }
                }
            }
            NetWorthHolder {
                id: d.id.clone(),
                label: d.label.clone(),
                note: d.note.clone(),
                in_headline: d.in_headline,
                households,
                people,
                assets,
                liabilities,
                net_worth: assets - liabilities,
                how_id: d.how_id.clone(),
            }
        })
        .collect();
    let holders_total: f64 = holders
        .iter()
        .filter(|h| h.in_headline)
        .map(|h| h.net_worth)
        .sum();
    TownNetWorth {
        reconciles: (holders_total - headline_total).abs()
            <= tolerance(&[holders_total, headline_total]),
        holders,
        headline: headline_total,
        holders_total,
    }
}
