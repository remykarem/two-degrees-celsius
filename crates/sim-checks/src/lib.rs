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
    for x in &s.persons {
        let i = x.id.0;
        add(format!("person {i} skill"), x.skill.0);
        add(format!("person {i} effective_hours"), x.effective_hours.0);
        add(format!("person {i} wellbeing"), x.wellbeing.0);
    }
    for h in &s.households {
        let i = h.id.0;
        add(format!("household {i} wellbeing"), h.wellbeing.0);
        add(
            format!("household {i} political_power"),
            h.political_power.0,
        );
        add(format!("household {i} food_met"), h.food_met.0);
        add(format!("household {i} footprint"), h.footprint.0);
        add(format!("household {i} energy_use"), h.energy_use.0);
    }
    for f in &s.firms {
        let i = f.id.0;
        add(format!("firm {i} capital"), f.capital.0);
        add(format!("firm {i} workers"), f.workers.0);
        add(format!("firm {i} inventory"), f.inventory.0);
        add(format!("firm {i} output"), f.output.0);
        add(format!("firm {i} emissions"), f.emissions.0);
        add(format!("firm {i} footprint"), f.footprint.0);
        add(format!("firm {i} energy_use"), f.energy_use.0);
        add(format!("firm {i} green_tech"), f.green_tech.0);
        add(format!("firm {i} tfp"), f.tfp.0);
        add(format!("firm {i} intensity_base"), f.intensity_base.0);
        add(format!("firm {i} energy_per_unit"), f.energy_per_unit.0);
        add(format!("firm {i} loan_rate"), f.loan_rate.0);
        add(format!("firm {i} excess_demand"), f.excess_demand.0);
        add(format!("firm {i} expected_sales"), f.expected_sales.0);
        add(format!("firm {i} markup"), f.markup.0);
        for (k, x) in f.sales_hist.iter().enumerate() {
            add(format!("firm {i} sales_hist[{k}]"), *x);
        }
    }
    for p in &s.plants {
        let i = p.id.0;
        add(format!("plant {i} capacity"), p.capacity.0);
        add(format!("plant {i} output"), p.output.0);
        add(format!("plant {i} emissions"), p.emissions.0);
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
    add("housing occupancy".into(), s.housing.occupancy.0);
    add("energy demand".into(), s.energy.demand.0);
    add("energy supply".into(), s.energy.supply.0);
    add("energy delivered".into(), s.energy.delivered.0);
    add("energy served".into(), s.energy.served.0);
    add("energy grid_intensity".into(), s.energy.grid_intensity.0);
    add("gov legitimacy".into(), s.gov.legitimacy.0);
    add("bank default_rate".into(), s.bank.default_rate.0);
    add("union_density".into(), s.union_density.0);
    add("net_lobby".into(), s.net_lobby.0);
    add("political_capital".into(), s.political_capital);
    add("output_penalty".into(), s.output_penalty.0);
    add(
        "indicators unemployment".into(),
        s.indicators.unemployment.0,
    );
    add("indicators gini".into(), s.indicators.gini.0);
    add(
        "indicators top_wealth_share".into(),
        s.indicators.top_wealth_share.0,
    );
    add("indicators wellbeing".into(), s.indicators.avg_wellbeing.0);
    add("indicators temperature".into(), s.indicators.temperature);
    add("indicators gpi".into(), s.indicators.gpi_annual);
    add("indicators real gdp".into(), s.indicators.gdp_real_annual);
    add("indicators price_index".into(), s.indicators.price_index);
    v
}

/// Net financial position of each sector, computed from balances alone (never from the
/// money ledger): deposits minus debts for households, firms and government, plus bank equity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SectorBalances {
    pub households: Cents,
    pub firms: Cents,
    pub government: Cents,
    pub bank: Cents,
}

impl SectorBalances {
    pub fn of(s: &State) -> SectorBalances {
        SectorBalances {
            households: Cents::sum(s.households.iter().map(|h| h.savings - h.debt)),
            firms: Cents::sum(s.firms.iter().map(|f| f.cash - f.debt)),
            government: s.gov.cash - s.gov.debt,
            bank: s.bank.equity,
        }
    }
    pub fn total(&self) -> Cents {
        self.households + self.firms + self.government + self.bank
    }
}

/// Sector-balance identity (model doc section 8): every financial asset is someone's
/// liability, so the sector net positions sum to zero at all times, and therefore household
/// saving = firm financing need (investment) + government deficit + change in bank equity.
/// Computed from state before and after a system, independent of the apply ledger.
pub fn check_sector_balance(
    before: &SectorBalances,
    after: &SectorBalances,
    tick: u32,
    system: SystemId,
) -> Result<(), Violation> {
    if after.total() != Cents::ZERO {
        return fail(
            tick,
            system,
            format!(
                "sector net positions sum to {} (households {}, firms {}, government {}, bank {})",
                after.total().0,
                after.households.0,
                after.firms.0,
                after.government.0,
                after.bank.0
            ),
        );
    }
    let saving = after.households - before.households;
    let firm_need = before.firms - after.firms;
    let deficit = before.government - after.government;
    let bank_gain = after.bank - before.bank;
    if saving != firm_need + deficit - bank_gain {
        return fail(
            tick,
            system,
            format!(
                "household saving {} != firm need {} + government deficit {} - bank gain {}",
                saving.0, firm_need.0, deficit.0, bank_gain.0
            ),
        );
    }
    Ok(())
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
    let stocks: [(&str, f64); 4] = [
        ("forest", s.nature.forest.0),
        ("commons", s.nature.commons.0),
        ("cumulative emissions", s.nature.cumulative.0),
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
    }
    check_people_homes_tiles(s, tick, system)?;
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
    let pop = s.persons.len() as f64;
    if (pop - expected_population).abs() > 1e-6 {
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

/// Structural invariants of Town v2: every person is in exactly one household, every home holds
/// at most one household (and the two sides agree), jobs refer to active firms and match the
/// firms' headcounts, and the map holds at most one thing per tile with every thing on its tile.
pub fn check_people_homes_tiles(s: &State, tick: u32, system: SystemId) -> Result<(), Violation> {
    // People and households.
    let mut count = vec![0u32; s.persons.len()];
    for h in &s.households {
        for m in &h.members {
            match count.get_mut(m.idx()) {
                Some(c) => *c += 1,
                None => {
                    return fail(
                        tick,
                        system,
                        format!("household {} lists unknown person {}", h.id.0, m.0),
                    )
                }
            }
            if s.persons[m.idx()].household != h.id {
                return fail(
                    tick,
                    system,
                    format!(
                        "person {} is listed by household {} but belongs to {}",
                        m.0,
                        h.id.0,
                        s.persons[m.idx()].household.0
                    ),
                );
            }
        }
    }
    if let Some(i) = count.iter().position(|c| *c != 1) {
        return fail(
            tick,
            system,
            format!("person {i} is in {} households, not exactly one", count[i]),
        );
    }
    // Jobs.
    for x in &s.persons {
        if let Some(f) = x.job {
            if !s.firms.get(f.idx()).is_some_and(|f| f.active) {
                return fail(
                    tick,
                    system,
                    format!(
                        "person {} works in inactive or missing firm {}",
                        x.id.0, f.0
                    ),
                );
            }
            if x.jg {
                return fail(
                    tick,
                    system,
                    format!("person {} has a firm job and a guarantee job", x.id.0),
                );
            }
        }
        if (x.job.is_some() || x.jg) && !x.in_labour_force {
            return fail(
                tick,
                system,
                format!("person {} works but is outside the labour force", x.id.0),
            );
        }
    }
    for f in &s.firms {
        let n = s.persons.iter().filter(|x| x.job == Some(f.id)).count() as f64;
        if (n - f.workers.0).abs() > 1e-9 {
            return fail(
                tick,
                system,
                format!(
                    "firm {} headcount {} != {} workers by job",
                    f.id.0, f.workers.0, n
                ),
            );
        }
    }
    // Homes.
    for (i, home) in s.homes.iter().enumerate() {
        if s.homes[..i].iter().any(|o| o.id == home.id) {
            return fail(tick, system, format!("duplicate home id {}", home.id.0));
        }
        if let Some(hh) = home.household {
            let ok = s
                .households
                .get(hh.idx())
                .is_some_and(|h| h.home == Some(home.id));
            if !ok {
                return fail(
                    tick,
                    system,
                    format!(
                        "home {} holds household {} which lives elsewhere",
                        home.id.0, hh.0
                    ),
                );
            }
        }
    }
    for h in &s.households {
        if let Some(hid) = h.home {
            match s.home(hid) {
                Some(home) if home.household == Some(h.id) => {}
                _ => {
                    return fail(
                        tick,
                        system,
                        format!(
                            "household {} claims home {} that does not hold it",
                            h.id.0, hid.0
                        ),
                    )
                }
            }
        }
    }
    // Map: one thing per tile, and every thing sits on a tile that says so.
    let m = &s.map;
    if m.cells.len() != m.width as usize * m.height as usize {
        return fail(tick, system, "map size does not match its cells".into());
    }
    let mut owner_of_tile: Vec<Option<String>> = vec![None; m.cells.len()];
    let mut claim = |t: Tile, kind_ok: &[TileKind], id: Option<u32>, what: String| {
        let Some(c) = m.get(t) else {
            return Err(format!(
                "{what} stands outside the map at ({}, {})",
                t.x, t.y
            ));
        };
        let slot = &mut owner_of_tile[m.idx(t)];
        if let Some(other) = slot {
            return Err(format!(
                "tile ({}, {}) holds both {other} and {what}",
                t.x, t.y
            ));
        }
        *slot = Some(what.clone());
        if !kind_ok.contains(&c.kind) || c.id != id {
            return Err(format!(
                "{what} at ({}, {}) but the map shows {:?} {:?}",
                t.x, t.y, c.kind, c.id
            ));
        }
        Ok(())
    };
    for home in &s.homes {
        let kind = match home.owner {
            HomeOwner::Public => TileKind::PublicHousing,
            HomeOwner::Household(_) => TileKind::Home,
        };
        claim(
            home.tile,
            &[kind],
            Some(home.id.0),
            format!("home {}", home.id.0),
        )
        .or_else(|e| fail(tick, system, e))?;
    }
    for f in &s.firms {
        if f.active {
            claim(
                f.tile,
                &[TileKind::Firm],
                Some(f.id.0),
                format!("firm {}", f.id.0),
            )
            .or_else(|e| fail(tick, system, e))?;
        }
    }
    for pl in &s.plants {
        let kind = match pl.kind {
            PlantKind::Fossil => TileKind::FossilPlant,
            PlantKind::Renewable => TileKind::RenewablePlant,
        };
        claim(
            pl.tile,
            &[kind],
            Some(pl.id.0),
            format!("plant {}", pl.id.0),
        )
        .or_else(|e| fail(tick, system, e))?;
    }
    // Everything else on the map is scenery or a road and carries no id.
    for (i, c) in m.cells.iter().enumerate() {
        let known = owner_of_tile[i].is_some();
        let expects_entity = matches!(
            c.kind,
            TileKind::Home
                | TileKind::PublicHousing
                | TileKind::FossilPlant
                | TileKind::RenewablePlant
        );
        if expects_entity && !known {
            return fail(
                tick,
                system,
                format!("map shows {:?} at cell {i} with nothing behind it", c.kind),
            );
        }
        if c.kind == TileKind::Firm && !known {
            // A closed firm keeps its building; it must still be a firm slot's own tile.
            let t = Tile {
                x: (i % m.width as usize) as u8,
                y: (i / m.width as usize) as u8,
            };
            if !s.firms.iter().any(|f| f.tile == t) {
                return fail(
                    tick,
                    system,
                    format!("map shows a firm at cell {i} with no firm"),
                );
            }
        }
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
    fn sector_identity_detects_a_leak() {
        let a = SectorBalances {
            households: Cents(100),
            firms: Cents(-60),
            government: Cents(-50),
            bank: Cents(10),
        };
        assert!(check_sector_balance(&a, &a, 1, SystemId::Goods).is_ok());
        let leak = SectorBalances {
            households: Cents(150),
            ..a
        };
        let err = check_sector_balance(&a, &leak, 1, SystemId::Goods).unwrap_err();
        assert!(err.what.contains("sum to"));
    }

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
