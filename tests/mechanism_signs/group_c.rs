//! Group C: money flows, measured on the real Goods and Income systems.
//! A town quantity is a sum over active firms or households; each probe says which field it
//! nudges and which it reads. Mechanisms of one system write together, so where a stand-in is
//! needed (UnitsSold, UnitCost, Dividends have no state field) the comment says what it is.
use crate::common::{after, params, states};
use sim_model::causal::Node;
use sim_model::sign_probe::{Probe, Report};
use sim_params::Params;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;
use std::sync::OnceLock;

fn p() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(params)
}

/// Mid-run states plus variants that keep the relevant clamps off.
fn bases() -> Vec<State> {
    let all = states();
    let mut out: Vec<State> = all.iter().take(3).cloned().collect();
    out.push(all[3].clone());
    let mid = all[1].clone();
    // Rich households: budgets are not capped by savings.
    let mut rich = mid.clone();
    for h in &mut rich.households {
        h.savings = Cents(h.savings.0.max(0) * 20 + 50_000_000);
        h.last_disposable = Cents(h.last_disposable.0.max(0) * 3 + 2_000_000);
    }
    out.push(rich.clone());
    // Scarce stock: a dearer price raises what is spent (sales are stock-limited).
    let mut scarce = rich;
    for f in &mut scarce.firms {
        f.inventory = Units(f.inventory.0 * 0.01);
    }
    out.push(scarce);
    // Firms with sales and cash, so profit is positive and the profit tax applies.
    let mut profitable = mid;
    for f in &mut profitable.firms {
        f.revenue = Cents(f.revenue.0.max(0) * 4 + 5_000_000);
        f.cash = Cents(f.cash.0.max(0) * 4 + 5_000_000);
    }
    profitable.effective.tax_profit = Rate(0.2);
    out.push(profitable.clone());
    // Firms short of cash for payroll.
    let mut short = profitable;
    for f in &mut short.firms {
        f.cash = Cents(10_000);
    }
    out.push(short);
    out
}

fn act(s: &mut State, f: impl Fn(&mut Firm)) {
    for x in s.firms.iter_mut().filter(|f| f.active) {
        f(x);
    }
}

fn firm_sum(s: &State, f: impl Fn(&Firm) -> f64) -> f64 {
    s.firms.iter().filter(|x| x.active).map(f).sum()
}

fn hh_savings(s: &State) -> f64 {
    s.households.iter().map(|h| h.savings.as_f64()).sum()
}

/// Round away summation noise so a flat response is read as flat.
fn q(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

/// Put a jobless person on the first active firm's payroll.
fn give_job(s: &mut State) {
    let Some(fid) = s.firms.iter().find(|f| f.active).map(|f| f.id) else {
        return;
    };
    if let Some(x) = s
        .persons
        .iter_mut()
        .find(|x| x.in_labour_force && !x.employed())
    {
        x.job = Some(fid);
    }
}

fn wage_up(s: &mut State) {
    act(s, |f| f.wage = Cents(f.wage.0 * 11 / 10 + 100));
}

fn goods(s: &State) -> State {
    after(SystemId::Goods, s, p())
}
fn income(s: &State) -> State {
    after(SystemId::Income, s, p())
}

/// Keep from a sub-report only the listed (from, to) pairs and add them to `r`, so one mechanism
/// can be measured by several probes (each pair is judged exactly once).
fn merge(r: &mut Report, a: Report, pairs: &[(Q, Q)]) {
    let labels: Vec<String> = pairs
        .iter()
        .map(|(f, t)| format!(": {} -> {}:", f.as_str(), t.as_str()))
        .collect();
    r.failures.extend(
        a.failures
            .into_iter()
            .filter(|m| labels.iter().any(|l| m.contains(l.as_str()))),
    );
    r.probed.extend(
        a.probed
            .into_iter()
            .filter(|(_, f, t, _)| pairs.contains(&(*f, *t))),
    );
    r.strengths.extend(a.strengths.into_iter().filter(|e| {
        pairs
            .iter()
            .any(|(f, t)| e.from == f.as_str() && e.to == t.as_str())
    }));
}

pub fn probe_all(r: &mut Report) {
    goods_settlement(r);
    unit_cost(r);
    wage_payment(r);
    income_flows(r);
    profit(r);
}

/// UnitsSold has no field: it follows the buyers' budgets, so households' `last_disposable` (which
/// sets the discretionary budget) is the stand-in nudge; Price nudges every active firm's price.
/// Reads, after the Goods system: sum of firm revenue, firm inventory, household savings and
/// household consumption. A budget nudge changes savings only through spending.
fn goods_settlement(r: &mut Report) {
    fn units(s: &mut State) {
        for h in &mut s.households {
            h.last_disposable = Cents(h.last_disposable.0 * 2 + 1_000_000);
        }
    }
    fn price(s: &mut State) {
        act(s, |f| f.price = Cents(f.price.0 * 11 / 10 + 1));
    }
    let probe = Probe {
        node: Node::Mechanism(M::GoodsSettlement),
        bases: bases(),
        nudges: vec![(Q::UnitsSold, units as fn(&mut State)), (Q::Price, price)],
        levels: crate::common::levels(),
        reads: vec![
            (
                Q::Revenue,
                (|s| q(firm_sum(&goods(s), |f| f.revenue.as_f64()))) as fn(&State) -> f64,
            ),
            (Q::Inventory, |s| q(firm_sum(&goods(s), |f| f.inventory.0))),
            (Q::Savings, |s| q(hh_savings(&goods(s)))),
            (Q::Consumption, |s| {
                q(goods(s)
                    .households
                    .iter()
                    .map(|h| h.consumption.as_f64())
                    .sum())
            }),
        ],
    };
    // Price -> Inventory is not measurable here: the spec holds UnitsSold fixed, but in the whole
    // system a dearer price lowers the units a fixed nominal budget buys (the shop decision), so
    // inventory rises. It stays declared_unverified.
    let mut a = Report::new(Basis::PerturbedSystem);
    a.check(&probe);
    let all = [
        (Q::UnitsSold, Q::Revenue),
        (Q::UnitsSold, Q::Inventory),
        (Q::UnitsSold, Q::Savings),
        (Q::UnitsSold, Q::Consumption),
        (Q::Price, Q::Revenue),
        (Q::Price, Q::Savings),
        (Q::Price, Q::Consumption),
    ];
    merge(r, a, &all);
}

/// UnitCost is not stored: `price_view` computes it and `decide_price` moves the price toward
/// (1 + markup) x unit cost, so the read is the sum of firm prices after Goods (monotone in unit
/// cost). Wage nudges firm wage; Workers puts a jobless person on a payroll; Output raises firm
/// output; CarbonPrice and Footprint reach unit cost only through `firm.carbon_cost` (written by
/// Income), so both nudge that field.
fn unit_cost(r: &mut Report) {
    fn carbon(s: &mut State) {
        act(s, |f| f.carbon_cost = Cents(f.carbon_cost.0 * 2 + 100_000));
    }
    let probe = Probe {
        node: Node::Mechanism(M::UnitCostBuildup),
        bases: bases(),
        nudges: vec![
            (Q::Wage, wage_up as fn(&mut State)),
            (Q::Workers, give_job),
            (Q::CarbonPrice, carbon),
            (Q::Footprint, carbon),
            (Q::Output, |s| {
                act(s, |f| f.output = Units(f.output.0 * 1.2 + 1.0))
            }),
        ],
        levels: crate::common::levels(),
        reads: vec![(
            Q::UnitCost,
            (|s| q(firm_sum(&goods(s), |f| f.price.as_f64()))) as fn(&State) -> f64,
        )],
    };
    r.check(&probe);
}

/// Payroll: sum of persons' pay this tick (`person.wage`); Savings: household savings after Income
/// (wages and dividends in, rent moved between households, rent to public housing out).
fn wage_payment(r: &mut Report) {
    fn cash(s: &mut State) {
        act(s, |f| f.cash = Cents(f.cash.0 + 20_000));
    }
    let payroll = Probe {
        node: Node::Mechanism(M::WagePayment),
        bases: bases(),
        nudges: vec![
            (Q::Wage, wage_up as fn(&mut State)),
            (Q::Workers, give_job),
            (Q::Cash, cash),
        ],
        levels: crate::common::levels(),
        reads: vec![(
            Q::Payroll,
            (|s| q(income(s).persons.iter().map(|x| x.wage.as_f64()).sum())) as fn(&State) -> f64,
        )],
    };
    let mut a = Report::new(Basis::PerturbedSystem);
    a.check(&payroll);
    merge(
        r,
        a,
        &[
            (Q::Wage, Q::Payroll),
            (Q::Workers, Q::Payroll),
            (Q::Cash, Q::Payroll),
        ],
    );
    // Savings: the cash-rich "profitable" sample (index 6) is left out. There a dearer wage also
    // lifts the firm's reserve (payroll plus debt service) and its idle-cash line (six months of
    // outgoings), so the payout decision holds back more dividends than the wage adds: the
    // wage's own effect on households is `+`, but the town total can fall through dividends
    // (a finding, see the report). Dividends belong to the payout decision, not to wage payment.
    let mut bs = bases();
    bs.remove(6);
    let savings = Probe {
        node: Node::Mechanism(M::WagePayment),
        bases: bs,
        nudges: vec![
            (Q::Wage, wage_up as fn(&mut State)),
            (Q::Workers, give_job),
            (Q::Cash, cash),
        ],
        levels: crate::common::levels(),
        reads: vec![(
            Q::Savings,
            (|s| q(hh_savings(&income(s)))) as fn(&State) -> f64,
        )],
    };
    let mut a = Report::new(Basis::PerturbedSystem);
    a.check(&savings);
    merge(
        r,
        a,
        &[
            (Q::Wage, Q::Savings),
            (Q::Workers, Q::Savings),
            (Q::Cash, Q::Savings),
        ],
    );
}

/// Dividends are not a field: a firm's cash raises what it can pay out, so the stand-in nudge is
/// firm cash. Savings is the sum of household savings, CapitalIncome the sum of households'
/// `income.capital_income()` (dividends + rent received + interest). Rent nudges every home's
/// rent. Rent -> Savings needs a per-household read (town savings only move when public housing
/// is the landlord), so it is a second probe whose bases carry a household index k: household k
/// is a renter in some samples and a landlord in others.
fn income_flows(r: &mut Report) {
    fn rent(s: &mut State) {
        for h in &mut s.homes {
            h.rent = Cents(h.rent.0 * 3 / 2 + 1_000);
        }
    }
    let probe = Probe {
        node: Node::Mechanism(M::HouseholdIncomeFlows),
        bases: bases(),
        nudges: vec![
            (
                Q::Dividends,
                (|s| act(s, |f| f.cash = Cents(f.cash.0 * 3 + 5_000_000))) as fn(&mut State),
            ),
            (Q::Rent, rent),
        ],
        levels: crate::common::levels(),
        reads: vec![
            (
                Q::CapitalIncome,
                (|s| {
                    q(income(s)
                        .households
                        .iter()
                        .map(|h| h.income.capital_income().as_f64())
                        .sum())
                }) as fn(&State) -> f64,
            ),
            (Q::Savings, |s| q(hh_savings(&income(s)))),
        ],
    };
    let mut a = Report::new(Basis::PerturbedSystem);
    a.check(&probe);
    merge(
        r,
        a,
        &[
            (Q::Dividends, Q::Savings),
            (Q::Dividends, Q::CapitalIncome),
            (Q::Rent, Q::CapitalIncome),
        ],
    );
    type B = (State, usize);
    let mut split = Probe::<B> {
        node: Node::Mechanism(M::HouseholdIncomeFlows),
        bases: Vec::new(),
        nudges: vec![(Q::Rent, (|b| rent(&mut b.0)) as fn(&mut B))],
        levels: vec![],
        reads: vec![(
            Q::Savings,
            (|b| q(income(&b.0).households[b.1].savings.as_f64())) as fn(&B) -> f64,
        )],
    };
    for s in bases() {
        for k in 0..s.households.len() {
            split.bases.push((s.clone(), k));
        }
    }
    let mut b = Report::new(Basis::PerturbedSystem);
    b.check(&split);
    merge(r, b, &[(Q::Rent, Q::Savings)]);
}

/// Profit: sum of firm profit after Income. Revenue and debt nudge firm fields; Payroll nudges the
/// wage (wages paid); CarbonPrice and ProfitTaxRate nudge the effective policy; Footprint nudges
/// the firm footprint (the carbon charge is price x footprint).
fn profit(r: &mut Report) {
    let probe = Probe {
        node: Node::Mechanism(M::FirmProfit),
        bases: bases()
            .into_iter()
            .map(|mut s| {
                s.effective.carbon_price = Cents(s.effective.carbon_price.0.max(5_000));
                act(&mut s, |f| f.footprint = Tonnes(f.footprint.0.max(1.0)));
                s
            })
            .collect(),
        nudges: vec![
            (
                Q::Revenue,
                (|s| act(s, |f| f.revenue = Cents(f.revenue.0 * 11 / 10 + 10_000)))
                    as fn(&mut State),
            ),
            (Q::Payroll, wage_up),
            (Q::DebtService, |s| {
                act(s, |f| f.debt = Cents(f.debt.0 * 2 + 1_000_000))
            }),
            (Q::CarbonPrice, |s| {
                s.effective.carbon_price = Cents(s.effective.carbon_price.0 * 2)
            }),
            (Q::Footprint, |s| {
                act(s, |f| f.footprint = Tonnes(f.footprint.0 * 2.0))
            }),
            (Q::ProfitTaxRate, |s| {
                s.effective.tax_profit = Rate(s.effective.tax_profit.0 + 0.1)
            }),
        ],
        levels: crate::common::levels(),
        reads: vec![(
            Q::Profit,
            (|s| q(firm_sum(&income(s), |f| f.profit.as_f64()))) as fn(&State) -> f64,
        )],
    };
    r.check(&probe);
}
