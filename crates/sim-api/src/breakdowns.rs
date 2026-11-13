//! `snapshot.breakdowns` (contract v1.6): every headline number with its formula, live inputs and
//! parts. Each builder calls the same model or metrics function as the headline (or, where the
//! headline is one expression, takes its parts from the same state), so the numbers cannot
//! drift; tests check that parts sum to the result (tolerance in [`parts_tolerance`]) and that
//! each result equals the headline indicator.
use crate::emissions::{self, EmissionsBreakdown};
use crate::finance::Finance;
use crate::log::WrapperLog;
use crate::snapshot::firm_names;
use crate::sources::Sources;
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types::{LegitStep, QuantityId};

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownInput {
    pub label: String,
    pub value: f64,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    /// Contract v2.9: the graph quantity (`catalog().quantities[].id`) this input is, where the
    /// registry has one. The arithmetic stays here; the cause lists come from the graph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quantity_id: Option<String>,
}

impl BreakdownInput {
    /// Link this input to the graph quantity it stands for.
    fn quantity(mut self, q: QuantityId) -> Self {
        self.quantity_id = Some(q.as_str().to_string());
        self
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Value {
    pub value: f64,
    pub unit: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub label: String,
    pub value: f64,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakdown {
    pub id: String,
    pub title: String,
    pub formula: String,
    pub inputs: Vec<BreakdownInput>,
    pub result: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parts: Option<Vec<Part>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sums_to_result: Option<bool>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LorenzPoint {
    pub population_share: f64,
    pub wealth_share: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdWealth {
    pub household: u32,
    pub people: u32,
    pub net_worth: f64,
    /// Contract v1.9: what the net worth is made of (savings minus debt, homes, jet, bank
    /// equity, firm equity). The ids are `net_cash`, `homes`, `jet`, `bank`, `firms`; labels
    /// come from `content/ui.json` `text.estate.wealth_part_*`. Zero parts are omitted.
    #[serde(default)]
    pub parts: Vec<WealthPartView>,
    /// Contract v2.2: the statement, assets minus liabilities, line by line.
    pub statement: crate::networth::NetWorthStatement,
    /// Contract v2.2: the holder group (`households`, `firm_owners`, `bank`, `estate`).
    pub holder: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WealthPartView {
    pub id: String,
    pub value: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GiniBreakdown {
    #[serde(flatten)]
    pub base: Breakdown,
    pub lorenz: Vec<LorenzPoint>,
    pub by_household: Vec<HouseholdWealth>,
    /// Contract v2.2: the town's net worth by holder, reconciled to the headline.
    pub town: crate::networth::TownNetWorth,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegitTerm {
    pub id: String,
    pub label: String,
    pub input: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub threshold: Option<f64>,
    pub weight: f64,
    pub contribution: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegitimacyBreakdown {
    #[serde(flatten)]
    pub base: Breakdown,
    pub terms: Vec<LegitTerm>,
    /// Change over the latest month (after minus before).
    pub trend: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxRow {
    pub id: String,
    pub label: String,
    pub applies_to: String,
    /// The rate as set (a fraction; dollars per tonne for the carbon price, see `unit`).
    pub rate_nominal: Option<f64>,
    /// The rate in force after lobbying (equal to `rateNominal` for taxes lobbying cannot touch).
    pub rate_effective: Option<f64>,
    pub revenue_month: f64,
    pub revenue_quarter: f64,
    pub modelled: bool,
    /// Additive: `fraction` or `usd_per_tonne`; the base the tax falls on this month and what it
    /// is; the revenue over the base; a note.
    pub unit: String,
    pub base_month: Option<f64>,
    pub base_label: String,
    pub realised_rate: Option<f64>,
    pub note: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Breakdowns {
    pub gdp: Breakdown,
    pub gpi: Breakdown,
    #[serde(rename = "wageShare")]
    pub wage_share: Breakdown,
    #[serde(rename = "profitShare")]
    pub profit_share: Breakdown,
    #[serde(rename = "debtToGdp")]
    pub debt_to_gdp: Breakdown,
    pub gini: GiniBreakdown,
    pub legitimacy: LegitimacyBreakdown,
    #[serde(rename = "capitalRatio")]
    pub capital_ratio: Breakdown,
    pub emissions: EmissionsBreakdown,
    pub taxes: Vec<TaxRow>,
}

/// Tolerance for "parts sum to the result": 1e-9 of the largest magnitude involved, at least 1e-9.
pub fn parts_tolerance(result: f64, parts: &[Part]) -> f64 {
    let big = parts
        .iter()
        .map(|p| p.value.abs())
        .fold(result.abs(), f64::max);
    1e-9 * big.max(1.0)
}

fn input(label: &str, value: f64, unit: &str, source: Option<&str>) -> BreakdownInput {
    BreakdownInput {
        label: label.to_string(),
        value,
        unit: unit.to_string(),
        source: source.map(str::to_string),
        quantity_id: None,
    }
}

fn part(label: impl Into<String>, value: f64, unit: &str, note: Option<&str>) -> Part {
    Part {
        label: label.into(),
        value,
        unit: unit.to_string(),
        note: note.map(str::to_string),
    }
}

fn make(
    id: &str,
    title: &str,
    formula: &str,
    inputs: Vec<BreakdownInput>,
    result: f64,
    unit: &str,
    parts: Option<Vec<Part>>,
) -> Breakdown {
    let sums = parts.as_ref().map(|ps| {
        let sum: f64 = ps.iter().map(|p| p.value).sum();
        (sum - result).abs() <= parts_tolerance(result, ps)
    });
    Breakdown {
        id: id.to_string(),
        title: title.to_string(),
        formula: formula.to_string(),
        inputs,
        result: Value {
            value: result,
            unit: unit.to_string(),
        },
        parts,
        sums_to_result: sums,
    }
}

fn gdp(sim: &Sim) -> Breakdown {
    let s = &sim.state;
    let names = firm_names(&s.firms);
    let i = &s.indicators;
    let mut parts: Vec<Part> = s
        .firms
        .iter()
        .zip(&names)
        .filter(|(f, _)| f.active)
        .map(|(f, n)| {
            part(
                format!("{n} sales"),
                f.revenue.dollars() * 12.0,
                "$/year",
                Some("firm sales this month x 12"),
            )
        })
        .collect();
    parts.push(part(
        "Investment",
        s.gdp_parts.investment.dollars() * 12.0,
        "$/year",
        Some("firms' capital spending this month x 12"),
    ));
    parts.push(part(
        "Government services",
        s.gdp_parts.gov_services.dollars() * 12.0,
        "$/year",
        Some("public works, upkeep and job guarantee pay x 12"),
    ));
    let nominal: f64 = parts.iter().map(|p| p.value).sum();
    let real = i.gdp_real_annual;
    parts.push(part(
        "Price adjustment",
        real - nominal,
        "$/year",
        Some("converts to constant base-year prices"),
    ));
    make(
        "gdp",
        "GDP (real, per year)",
        "GDP = (firm sales + investment + government services) x 12, divided by the price index",
        vec![
            input(
                "Firm sales (month)",
                sim_metrics::firm_sales(s).dollars(),
                "$",
                Some("finance"),
            ),
            input(
                "Investment (month)",
                s.gdp_parts.investment.dollars(),
                "$",
                None,
            ),
            input(
                "Government services (month)",
                s.gdp_parts.gov_services.dollars(),
                "$",
                None,
            ),
            input("Price index", i.price_index, "index", Some("prices")),
        ],
        real,
        "$/year",
        Some(parts),
    )
}

fn gpi(sim: &Sim) -> Breakdown {
    let s = &sim.state;
    let i = &s.indicators;
    let m = &sim.params.metrics;
    let gdp_annual = i.gdp_annual.dollars();
    let gini = i.gini.0;
    let carbon = m.scc_usd * 12.0 * s.nature.gross_emissions.0;
    let depletion = m.depletion_usd_per_unit * 12.0 * s.nature.depletion.0;
    let result = i.gpi_annual;
    let parts = vec![
        part("GDP (nominal)", gdp_annual, "$/year", None),
        part(
            "Inequality discount",
            gdp_annual * (1.0 - gini.min(1.0)) - gdp_annual,
            "$/year",
            Some("GDP x (1 - Gini) takes the Gini share of GDP away"),
        ),
        part(
            "Social cost of carbon",
            -carbon,
            "$/year",
            Some("cost per tonne x 12 x gross emissions this month"),
        ),
        part(
            "Depletion of forest and commons",
            -depletion,
            "$/year",
            Some("cost per unit x 12 x natural capital lost this month"),
        ),
    ];
    make(
        "gpi",
        "GPI (per year)",
        "GPI = GDP x (1 - Gini) - social cost of carbon x 12 x gross emissions - depletion cost x 12 x depletion",
        vec![
            input("GDP (nominal, annual)", gdp_annual, "$/year", Some("gdp")),
            input("Gini", gini, "0..1", Some("gini")).quantity(QuantityId::Inequality),
            input("Gross emissions (month)", s.nature.gross_emissions.0, "t CO2", Some("emissions")),
            input("Social cost of carbon", m.scc_usd, "$/t CO2", None),
            input("Natural capital lost (month)", s.nature.depletion.0, "units", None),
            input("Depletion cost", m.depletion_usd_per_unit, "$/unit", None),
        ],
        result,
        "$/year",
        Some(parts),
    )
}

fn share_breakdown(sim: &Sim, wages: bool) -> Breakdown {
    let s = &sim.state;
    let gdp_f = s.indicators.gdp_month.as_f64();
    let (id, title, formula, unit_label) = if wages {
        (
            "wageShare",
            "Wage share",
            "Wage share = wages paid to households / GDP (monthly)",
            "wages",
        )
    } else {
        (
            "profitShare",
            "Profit share",
            "Profit share = profits of firms making a profit / GDP (monthly)",
            "profit",
        )
    };
    let mut parts = Vec::new();
    let total = if wages {
        for h in &s.households {
            parts.push(part(
                format!("{} wages", h.label),
                sim_types::safe_div(h.income.wages.as_f64(), gdp_f, 0.0),
                "share",
                None,
            ));
        }
        s.households
            .iter()
            .map(|h| h.income.wages.as_f64())
            .sum::<f64>()
    } else {
        let names = firm_names(&s.firms);
        for (f, n) in s.firms.iter().zip(&names).filter(|(f, _)| f.active) {
            parts.push(part(
                format!("{n} profit"),
                sim_types::safe_div(f.profit.as_f64().max(0.0), gdp_f, 0.0),
                "share",
                None,
            ));
        }
        s.firms
            .iter()
            .filter(|f| f.active)
            .map(|f| f.profit.as_f64().max(0.0))
            .sum::<f64>()
    };
    let result = if wages {
        s.indicators.wage_share.0
    } else {
        s.indicators.profit_share.0
    };
    make(
        id,
        title,
        formula,
        vec![
            input(
                &format!("Total {unit_label} (month)"),
                total / 100.0,
                "$",
                None,
            ),
            input(
                "GDP (month)",
                s.indicators.gdp_month.dollars(),
                "$",
                Some("gdp"),
            ),
        ],
        result,
        "share",
        Some(parts),
    )
}

fn debt(sim: &Sim) -> Breakdown {
    let s = &sim.state;
    make(
        "debtToGdp",
        "Public debt / GDP",
        "Debt / GDP = government debt / annual GDP (nominal); 0 if GDP is not positive",
        vec![
            input("Government debt", s.gov.debt.dollars(), "$", None)
                .quantity(QuantityId::PublicDebt),
            input(
                "GDP (annual, nominal)",
                s.indicators.gdp_annual.dollars(),
                "$/year",
                Some("gdp"),
            ),
        ],
        crate::history::debt_to_gdp(s),
        "share",
        None,
    )
}

fn gini(sim: &Sim, sources: &Sources) -> GiniBreakdown {
    let s = &sim.state;
    let wealth = sim_metrics::household_wealth(s, &sim.params);
    let person = sim_metrics::person_wealth(s, &wealth);
    let g = s.indicators.gini.0;
    let mut sorted = person.clone();
    sorted.sort_by(f64::total_cmp);
    let total: f64 = sorted.iter().sum();
    let n = sorted.len() as f64;
    let mut lorenz = Vec::new();
    if total > 0.0 && n > 0.0 {
        lorenz.push(LorenzPoint {
            population_share: 0.0,
            wealth_share: 0.0,
        });
        let mut acc = 0.0;
        for (k, w) in sorted.iter().enumerate() {
            acc += w;
            lorenz.push(LorenzPoint {
                population_share: (k + 1) as f64 / n,
                wealth_share: acc / total,
            });
        }
    }
    let parts = sim_metrics::household_wealth_parts(s, &sim.params);
    let lines = sim_metrics::household_wealth_lines(s, &sim.params);
    let statements: Vec<_> = s
        .households
        .iter()
        .zip(&lines)
        .zip(&wealth)
        .map(|((h, l), w)| {
            crate::networth::statement(&sources.net_worth, l, h.members.len() as u32, *w)
        })
        .collect();
    let holders: Vec<&'static str> = s
        .households
        .iter()
        .zip(&lines)
        .map(|(h, l)| crate::networth::holder_of(s, h.id, l.firms_count))
        .collect();
    let town = crate::networth::town(
        &sources.net_worth,
        s,
        &sim.params,
        &statements,
        &holders,
        total / 100.0,
    );
    let by_household = s
        .households
        .iter()
        .zip(&wealth)
        .zip(&parts)
        .zip(statements)
        .zip(&holders)
        .map(|((((h, w), p), statement), holder)| HouseholdWealth {
            household: h.id.0,
            people: h.members.len() as u32,
            net_worth: w / 100.0,
            statement,
            holder: (*holder).to_string(),
            parts: [
                ("net_cash", p.net_cash),
                ("homes", p.homes),
                ("jet", p.jet),
                ("bank", p.bank),
                ("firms", p.firms),
            ]
            .into_iter()
            .filter(|(_, v)| *v != 0.0)
            .map(|(id, v)| WealthPartView {
                id: id.to_string(),
                value: v / 100.0,
            })
            .collect(),
        })
        .collect();
    GiniBreakdown {
        base: make(
            "gini",
            "Inequality (Gini)",
            "Gini = sum over all pairs of people of |wealth_i - wealth_j| / (2 x people x total wealth); each person holds an equal share of their household's net worth; 0 = equal, 1 = one person has everything",
            vec![
                input("People", n, "people", None),
                input("Total household net worth", total / 100.0, "$", None),
                input("Richest household's share", s.indicators.top_wealth_share.0, "share", Some("top_wealth_share")),
            ],
            g,
            "0..1",
            None,
        ),
        lorenz,
        by_household,
        town,
    }
}

fn legitimacy(sim: &Sim, log: &WrapperLog) -> LegitimacyBreakdown {
    let s = &sim.state;
    let gp = &sim.params.government;
    let live = log.ticks.last().and_then(|t| t.legitimacy);
    let (st, played) = match live {
        Some(x) => (x, true),
        // No month played yet: the update the first month would make, for information.
        None => (
            sim_model::government::legitimacy_step(s, &sim.params, false),
            false,
        ),
    };
    let terms = legit_terms(&st, gp);
    let level = s.gov.legitimacy.0;
    let mut parts = vec![part(
        "Level before the month",
        if played { st.before } else { level },
        "0..1",
        None,
    )];
    if played {
        for t in &terms {
            parts.push(part(t.label.clone(), t.contribution, "0..1", None));
        }
        parts.push(part(
            "Switching economic system (resistance)",
            st.transition_term,
            "0..1",
            Some("only while a system change is under way"),
        ));
        parts.push(part(
            "Clamped to 0..1",
            st.after - st.raw,
            "0..1",
            Some("legitimacy cannot leave the range 0..1"),
        ));
    }
    let result = level;
    let base = make(
        "legitimacy",
        "Legitimacy",
        "Each month: legitimacy = previous + w1 x (wellbeing - 0.5) - w2 x max(0, unemployment - threshold) - w3 x max(0, Gini - threshold) - w4 x max(0, debt/GDP - 1) - bailout penalty - system-change resistance, kept within 0..1",
        vec![
            input("Average wellbeing", st.wellbeing_input, "0..1", Some("wellbeing")).quantity(QuantityId::Wellbeing),
            input("Unemployment (smoothed)", st.unemployment_input, "share", Some("unemployment")).quantity(QuantityId::Unemployment),
            input("Gini", st.inequality_input, "0..1", Some("gini")).quantity(QuantityId::Inequality),
            input("Public debt / GDP", st.debt_input, "ratio", Some("debt")).quantity(QuantityId::PublicDebtRatio),
            input("Bank bailout this month", if st.bailout_term != 0.0 { 1.0 } else { 0.0 }, "0 or 1", None).quantity(QuantityId::Bailout),
        ],
        result,
        "0..1",
        Some(parts),
    );
    LegitimacyBreakdown {
        base,
        terms,
        trend: if played { st.after - st.before } else { 0.0 },
    }
}

fn legit_terms(st: &LegitStep, gp: &sim_params::GovParams) -> Vec<LegitTerm> {
    vec![
        LegitTerm {
            id: "wellbeing".into(),
            label: "Wellbeing above or below 0.5".into(),
            input: st.wellbeing_input,
            threshold: Some(0.5),
            weight: gp.legit_wellbeing,
            contribution: st.wellbeing_term,
        },
        LegitTerm {
            id: "unemployment".into(),
            label: "Unemployment above its threshold".into(),
            input: st.unemployment_input,
            threshold: Some(gp.legit_unemployment_threshold),
            weight: gp.legit_unemployment,
            contribution: st.unemployment_term,
        },
        LegitTerm {
            id: "inequality".into(),
            label: "Inequality above its threshold".into(),
            input: st.inequality_input,
            threshold: Some(gp.legit_gini_threshold),
            weight: gp.legit_gini,
            contribution: st.inequality_term,
        },
        LegitTerm {
            id: "debt".into(),
            label: "Public debt above one year of GDP".into(),
            input: st.debt_input,
            threshold: Some(1.0),
            weight: gp.legit_debt,
            contribution: st.debt_term,
        },
        LegitTerm {
            id: "bailout".into(),
            label: "Bank bailout".into(),
            input: if st.bailout_term != 0.0 { 1.0 } else { 0.0 },
            threshold: None,
            weight: gp.legit_bailout,
            contribution: st.bailout_term,
        },
    ]
}

fn capital_ratio(sim: &Sim, f: &Finance) -> Breakdown {
    let b = &f.bank;
    let s = &sim.state;
    make(
        "capitalRatio",
        "Bank capital ratio",
        "Capital ratio = bank equity / loans outstanding (1 when there are no loans). The bank may lend while equity / limit >= loans; headroom = equity / limit - loans",
        vec![
            input("Bank equity", b.equity, "$", None).quantity(QuantityId::BankEquity),
            input("Loans outstanding", b.loans, "$", None).quantity(QuantityId::BankLoans),
            input("Capital ratio limit (policy)", b.capital_ratio_limit, "share", Some("bank_capital_ratio")).quantity(QuantityId::CapitalRatio),
            input("Lending limit (equity / limit)", sim_model::util::max_loans(s.bank.equity, b.capital_ratio_limit).dollars(), "$", None),
            input("Headroom (room to lend)", b.headroom, "$", None).quantity(QuantityId::CreditHeadroom),
        ],
        b.capital_ratio,
        "share",
        None,
    )
}

fn taxes(sim: &Sim, f: &Finance) -> Vec<TaxRow> {
    let s = &sim.state;
    let m = &f.government.revenue;
    let q = &f.quarter.gov_revenue;
    let wages: f64 = s.households.iter().map(|h| h.income.wages.dollars()).sum();
    let capital: f64 = s
        .households
        .iter()
        .map(|h| h.income.capital_income().dollars())
        .sum();
    let pre_tax: f64 = s
        .firms
        .iter()
        .filter(|f| f.active)
        .map(|x| (x.profit + x.tax_paid).dollars().max(0.0))
        .sum();
    let footprint: f64 = s
        .firms
        .iter()
        .filter(|x| x.active)
        .map(|x| x.footprint.0)
        .sum();
    let wealth_over: f64 = sim_metrics::household_wealth(s, &sim.params)
        .iter()
        .map(|w| (w / 100.0 - s.policy.wealth_threshold.dollars()).max(0.0))
        .sum();
    let ratio = |rev: f64, base: f64| if base > 0.0 { Some(rev / base) } else { None };
    let row = |id: &str,
               label: &str,
               applies: &str,
               nominal: f64,
               effective: f64,
               rm: f64,
               rq: f64,
               unit: &str,
               base: f64,
               base_label: &str,
               realised: Option<f64>,
               note: &str| TaxRow {
        id: id.into(),
        label: label.into(),
        applies_to: applies.into(),
        rate_nominal: Some(nominal),
        rate_effective: Some(effective),
        revenue_month: rm,
        revenue_quarter: rq,
        modelled: true,
        unit: unit.into(),
        base_month: Some(base),
        base_label: base_label.into(),
        realised_rate: realised,
        note: note.into(),
    };
    vec![
        row("income", "Income tax", "Wages and salaries households earn (job guarantee and public-works pay included)", s.policy.tax_wage.0, s.policy.tax_wage.0, m.wage_tax, q.wage_tax, "fraction", wages, "household wages this month ($)", ratio(m.wage_tax, wages), "Lever key tax_wage. Paid only from what the household has in savings."),
        row("profit", "Profit tax", "Firm profit before tax (nothing on a loss), before dividends", s.policy.tax_profit.0, s.effective.tax_profit.0, m.profit_tax, q.profit_tax, "fraction", pre_tax, "firm profit before tax this month ($)", ratio(m.profit_tax, pre_tax), "Lobbying can weaken it: the rate in force is the effective rate."),
        row("capital", "Capital tax", "Households' dividends, rent received and interest received", s.policy.tax_capital.0, s.policy.tax_capital.0, m.capital_tax, q.capital_tax, "fraction", capital, "household capital income this month ($)", ratio(m.capital_tax, capital), "Lever key tax_capital."),
        row("wealth", "Wealth tax", "Household net worth above the threshold (annual rate, collected in twelfths)", s.policy.wealth_tax.0, s.effective.wealth_tax.0, m.wealth_tax, q.wealth_tax, "fraction", wealth_over, "net worth above the threshold ($)", ratio(m.wealth_tax * 12.0, wealth_over), "Annual rate; the realised rate is revenue x 12 over the taxed wealth. Lobbying can weaken it."),
        row("carbon", "Carbon price", "Each firm's footprint: direct emissions plus the emissions of the electricity it uses", s.policy.carbon_price.dollars(), s.effective.carbon_price.dollars(), m.carbon, q.carbon, "usd_per_tonne", footprint, "firm footprints this month (t CO2)", ratio(m.carbon, footprint), "A price in dollars per tonne, not a fraction. Lobbying can weaken it."),
        TaxRow {
            id: "gst".into(), label: "GST / VAT".into(), applies_to: "Not modelled: the town has no sales or value-added tax".into(),
            rate_nominal: None, rate_effective: None, revenue_month: 0.0, revenue_quarter: 0.0, modelled: false, unit: "fraction".into(),
            base_month: None, base_label: String::new(), realised_rate: None,
            note: "Adding a sales tax would change prices, demand and the ledger; it is a possible future option.".into(),
        },
    ]
}

pub fn build(sim: &Sim, f: &Finance, log: &WrapperLog, sources: &Sources) -> Breakdowns {
    Breakdowns {
        gdp: gdp(sim),
        gpi: gpi(sim),
        wage_share: share_breakdown(sim, true),
        profit_share: share_breakdown(sim, false),
        debt_to_gdp: debt(sim),
        gini: gini(sim, sources),
        legitimacy: legitimacy(sim, log),
        capital_ratio: capital_ratio(sim, f),
        emissions: emissions::build(sim, &log.ticks, sources),
        taxes: taxes(sim, f),
    }
}
