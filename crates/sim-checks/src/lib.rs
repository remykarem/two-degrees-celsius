//! Invariants run after every system: finite numbers, non-negative stocks,
//! money conservation. A violation names the tick and the system that caused it.
#![forbid(unsafe_code)]

use sim_types::*;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Violation {
    pub tick: u32,
    pub system: SystemId,
    pub what: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invariant violated at tick {} after {}: {}",
            self.tick,
            self.system.name(),
            self.what
        )
    }
}

impl std::error::Error for Violation {}

/// Money created and destroyed by one system's operations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoneyLedger {
    pub created: Cents,
    pub destroyed: Cents,
}

/// Which systems may create or destroy money: credit (S5) and government borrowing (S8).
pub fn may_change_money(system: SystemId) -> bool {
    matches!(system, SystemId::Credit | SystemId::Government)
}

/// Every floating-point value in the state, with a name.
pub fn all_floats(s: &State) -> Vec<(String, f64)> {
    let mut v: Vec<(String, f64)> = Vec::new();
    let mut add = |name: String, x: f64| v.push((name, x));
    for h in &s.households {
        let i = h.id.0;
        add(format!("household {i} weight"), h.weight.0);
        add(format!("household {i} labour_share"), h.labour_share.0);
        add(format!("household {i} skill"), h.skill.0);
        add(format!("household {i} wellbeing"), h.wellbeing.0);
        add(
            format!("household {i} political_power"),
            h.political_power.0,
        );
        add(format!("household {i} food_met"), h.food_met.0);
        add(format!("household {i} housed_share"), h.housed_share.0);
        add(format!("household {i} footprint"), h.footprint.0);
        for (f, e) in h.employment.iter().enumerate() {
            add(format!("household {i} employment[{f}]"), e.0);
        }
    }
    for f in &s.firms {
        let i = f.id.0;
        add(format!("firm {i} capital"), f.capital.0);
        add(format!("firm {i} workers"), f.workers.0);
        add(format!("firm {i} inventory"), f.inventory.0);
        add(format!("firm {i} output"), f.output.0);
        add(format!("firm {i} emissions"), f.emissions.0);
        add(format!("firm {i} green_tech"), f.green_tech.0);
        add(format!("firm {i} tfp"), f.tfp.0);
        add(format!("firm {i} intensity_base"), f.intensity_base.0);
        add(format!("firm {i} loan_rate"), f.loan_rate.0);
        add(format!("firm {i} excess_demand"), f.excess_demand.0);
        for (k, x) in f.sales_hist.iter().enumerate() {
            add(format!("firm {i} sales_hist[{k}]"), *x);
        }
    }
    let n = &s.nature;
    add("nature forest".into(), n.forest.0);
    add("nature commons".into(), n.commons.0);
    add("nature cumulative".into(), n.cumulative.0);
    add("nature net_emissions".into(), n.net_emissions.0);
    add("nature gross_emissions".into(), n.gross_emissions.0);
    add("nature sink".into(), n.sink.0);
    add("nature atmosphere".into(), n.atmosphere.0);
    add("nature depletion".into(), n.depletion.0);
    add("housing private_cap".into(), s.housing.private_cap.0);
    add("housing public_cap".into(), s.housing.public_cap.0);
    add("housing occupancy".into(), s.housing.occupancy.0);
    add("energy fossil".into(), s.energy.fossil_cap.0);
    add("energy renewable".into(), s.energy.renewable_cap.0);
    add("gov legitimacy".into(), s.gov.legitimacy.0);
    add("bank default_rate".into(), s.bank.default_rate.0);
    add("union_density".into(), s.union_density.0);
    add("net_lobby".into(), s.net_lobby.0);
    add("output_penalty".into(), s.output_penalty.0);
    add(
        "indicators unemployment".into(),
        s.indicators.unemployment.0,
    );
    add("indicators gini".into(), s.indicators.gini.0);
    add("indicators wellbeing".into(), s.indicators.avg_wellbeing.0);
    add("indicators temperature".into(), s.indicators.temperature);
    add("indicators gpi".into(), s.indicators.gpi_annual);
    add("indicators real gdp".into(), s.indicators.gdp_real_annual);
    add("indicators price_index".into(), s.indicators.price_index);
    v
}

fn fail(tick: u32, system: SystemId, what: String) -> Result<(), Violation> {
    Err(Violation { tick, system, what })
}

/// State invariants: finite, non-negative stocks, bank balance sheet, constant population.
pub fn check_state(
    s: &State,
    tick: u32,
    system: SystemId,
    expected_population: f64,
) -> Result<(), Violation> {
    for (name, x) in all_floats(s) {
        if !x.is_finite() {
            return fail(tick, system, format!("{name} is not finite ({x})"));
        }
    }
    let stocks: [(&str, f64); 6] = [
        ("forest", s.nature.forest.0),
        ("commons", s.nature.commons.0),
        ("cumulative emissions", s.nature.cumulative.0),
        ("private housing", s.housing.private_cap.0),
        ("public housing", s.housing.public_cap.0),
        ("legitimacy", s.gov.legitimacy.0),
    ];
    for (name, x) in stocks {
        if x < 0.0 {
            return fail(tick, system, format!("{name} is negative ({x})"));
        }
    }
    for h in &s.households {
        if h.savings.is_negative() || h.debt.is_negative() {
            return fail(
                tick,
                system,
                format!("household {} has negative savings or debt", h.id.0),
            );
        }
        if h.weight.0 <= 0.0 {
            return fail(
                tick,
                system,
                format!("household {} weight not positive", h.id.0),
            );
        }
        if h.employment.iter().any(|e| e.0 < -1e-9) {
            return fail(
                tick,
                system,
                format!("household {} negative employment", h.id.0),
            );
        }
        if h.employed().0 > h.labour_force().0 + 1e-6 {
            return fail(
                tick,
                system,
                format!("household {} employed exceeds labour force", h.id.0),
            );
        }
    }
    for f in &s.firms {
        if f.cash.is_negative() || f.debt.is_negative() {
            return fail(
                tick,
                system,
                format!("firm {} has negative cash or debt", f.id.0),
            );
        }
        if f.capital.0 < 0.0 || f.inventory.0 < -1e-9 || f.workers.0 < -1e-9 {
            return fail(
                tick,
                system,
                format!("firm {} has a negative stock", f.id.0),
            );
        }
        if !f.active && (f.cash != Cents::ZERO || f.debt != Cents::ZERO) {
            return fail(
                tick,
                system,
                format!("dormant firm {} holds money or debt", f.id.0),
            );
        }
    }
    if s.gov.cash.is_negative() || s.gov.debt.is_negative() {
        return fail(tick, system, "government cash or debt negative".into());
    }
    let pop: f64 = s.households.iter().map(|h| h.weight.0).sum();
    if (pop - expected_population).abs() > 1e-6 * expected_population.max(1.0) {
        return fail(
            tick,
            system,
            format!("population {pop} != {expected_population}"),
        );
    }
    // Bank balance sheet: loans = sum of debts, deposits = sum of balances,
    // equity = loans - deposits (the bank holds only loans).
    if s.bank.loans != s.total_debts() {
        return fail(
            tick,
            system,
            format!(
                "bank loans {} != sum of debts {}",
                s.bank.loans.0,
                s.total_debts().0
            ),
        );
    }
    if s.bank.deposits != s.total_deposits() {
        return fail(
            tick,
            system,
            format!(
                "bank deposits {} != sum of balances {}",
                s.bank.deposits.0,
                s.total_deposits().0
            ),
        );
    }
    if s.bank.equity != s.bank.loans - s.bank.deposits {
        return fail(
            tick,
            system,
            format!(
                "bank equity {} != loans {} - deposits {}",
                s.bank.equity.0, s.bank.loans.0, s.bank.deposits.0
            ),
        );
    }
    // The credit system resolves a negative equity (haircut or bailout), so the stage
    // that follows it is the first place where it must be non-negative.
    let resolved_by_now = !matches!(
        system,
        SystemId::Commands
            | SystemId::Policy
            | SystemId::Labour
            | SystemId::Production
            | SystemId::Nature
            | SystemId::Goods
            | SystemId::Income
    );
    if resolved_by_now && s.bank.equity.is_negative() {
        return fail(tick, system, "bank equity negative and unresolved".into());
    }
    Ok(())
}

/// Money conservation: the change in total deposits equals created minus destroyed,
/// and only credit systems may create or destroy money.
pub fn check_money(
    before: Cents,
    after: Cents,
    ledger: MoneyLedger,
    tick: u32,
    system: SystemId,
) -> Result<(), Violation> {
    let expected = before + ledger.created - ledger.destroyed;
    if after != expected {
        return fail(
            tick,
            system,
            format!(
                "money not conserved: before {} created {} destroyed {} expected {} got {}",
                before.0, ledger.created.0, ledger.destroyed.0, expected.0, after.0
            ),
        );
    }
    if !may_change_money(system)
        && (ledger.created != Cents::ZERO || ledger.destroyed != Cents::ZERO)
    {
        return fail(
            tick,
            system,
            "system created or destroyed money outside credit".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_conserved_ok() {
        let l = MoneyLedger::default();
        assert!(check_money(Cents(100), Cents(100), l, 3, SystemId::Goods).is_ok());
    }

    #[test]
    fn money_leak_names_tick_and_system() {
        let e = check_money(
            Cents(100),
            Cents(90),
            MoneyLedger::default(),
            7,
            SystemId::Income,
        )
        .unwrap_err();
        assert_eq!(e.tick, 7);
        assert_eq!(e.system, SystemId::Income);
        assert!(e.to_string().contains("S4 income distribution"));
    }

    #[test]
    fn only_credit_may_create_money() {
        let l = MoneyLedger {
            created: Cents(5),
            destroyed: Cents(0),
        };
        assert!(check_money(Cents(100), Cents(105), l, 1, SystemId::Goods).is_err());
        assert!(check_money(Cents(100), Cents(105), l, 1, SystemId::Credit).is_ok());
    }
}
