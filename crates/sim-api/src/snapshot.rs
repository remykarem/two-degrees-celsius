//! The `Snapshot` view of the sim state (contract sections 4, 9 and 10). A stable wire type,
//! decoupled from the internal `State`. One function per area.
use crate::history::{budget_share, debt_to_gdp, HistoryPoint};
use crate::keys::{ClassKey, GameOutcome, LeverKey, PlainOutcome, SectorKey, SystemKey};
use crate::meta::{GameCalendar, Meta};
use crate::turn::PendingItem;
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types as st;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOver {
    pub turn: u32,
    pub outcome: GameOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PcView {
    pub current: f64,
    pub cap: f64,
    pub regen_next_turn: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionView {
    pub to: SystemKey,
    pub months_left: u32,
    pub total: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SystemView {
    pub current: SystemKey,
    pub transition: Option<TransitionView>,
}

/// A lever value: number, boolean or a mode name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PolicyScalar {
    Bool(bool),
    Number(f64),
    Mode(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyEntry {
    pub nominal: PolicyScalar,
    /// Present only for the four lobbied levers; `null` otherwise.
    pub effective: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Buildings {
    pub public_housing: u32,
    pub fossil_plant: u32,
    pub renewable_plant: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Indicators {
    pub gdp_annual: f64,
    pub gdp_real_annual: f64,
    pub unemployment: f64,
    pub employed: f64,
    pub labour_force: f64,
    pub gini: f64,
    pub wage_share: f64,
    pub profit_share: f64,
    pub avg_wellbeing: f64,
    pub remaining_budget: f64,
    pub budget_share: f64,
    pub temperature: f64,
    pub gpi_annual: f64,
    pub money_in_circulation: f64,
    pub price_index: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdIncome {
    pub wages: f64,
    pub dividends: f64,
    pub rent: f64,
    pub interest: f64,
    pub benefits: f64,
    pub tax_paid: f64,
    pub rent_paid: f64,
    pub loan_payments: f64,
    pub spending: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdView {
    pub id: u32,
    pub label: String,
    pub class: ClassKey,
    pub people: f64,
    pub employed: f64,
    pub labour_force: f64,
    pub savings: f64,
    pub debt: f64,
    pub wealth_per_capita: f64,
    pub income: HouseholdIncome,
    pub wellbeing: f64,
    pub political_power: f64,
    pub footprint_tonnes: f64,
    pub food_met: f64,
    pub housed_share: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FirmOwner {
    Household { id: u32 },
    Workers,
    State,
    Bank,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmView {
    pub id: u32,
    pub active: bool,
    pub sector: SectorKey,
    pub name: String,
    pub owner: FirmOwner,
    pub workers: f64,
    pub wage: f64,
    pub price: f64,
    pub output: f64,
    pub emissions: f64,
    pub revenue: f64,
    pub profit: f64,
    pub capital: f64,
    pub debt: f64,
    pub green_tech: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankView {
    pub equity: f64,
    pub loans: f64,
    pub deposits: f64,
    pub default_rate: f64,
    pub failed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernmentView {
    pub cash: f64,
    pub debt: f64,
    pub debt_to_gdp: f64,
    pub legitimacy: f64,
    pub receipts: f64,
    pub spending: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommonsAccessKey {
    Open,
    Community,
    Enclosed,
}

/// A stock as a share of its maximum, clamped to 0..=1 (0 if the maximum is not positive).
fn share(stock: f64, max: f64) -> f64 {
    if max > 0.0 {
        (stock / max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NatureView {
    /// Forest stock as a share of `nature.forest_max`, 0..=1 (`stock / forest_max`, clamped).
    pub forest: f64,
    /// Commons stock as a share of `nature.commons_max`, 0..=1 (`stock / commons_max`, clamped).
    pub commons: f64,
    /// Raw forest stock (the model's unit).
    pub forest_stock: f64,
    /// Raw commons stock (the model's unit).
    pub commons_stock: f64,
    pub commons_access: CommonsAccessKey,
    pub cumulative_tonnes: f64,
    pub net_emissions_monthly: f64,
    pub gross_emissions_monthly: f64,
    pub sink_monthly: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HousingView {
    pub private_cap: f64,
    pub public_cap: f64,
    pub rent_private: f64,
    pub rent_public: f64,
    pub occupancy: f64,
    pub housed_share: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnergyView {
    pub fossil_cap: f64,
    pub renewable_cap: f64,
    pub fossil_share: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: u32,
    pub game: GameCalendar,
    pub turn: u32,
    pub tick: u32,
    pub game_over: Option<GameOver>,
    pub pc: PcView,
    pub system: SystemView,
    pub buildings: Buildings,
    pub pending: Vec<PendingItem>,
    pub policy: BTreeMap<LeverKey, PolicyEntry>,
    pub net_lobby: f64,
    pub union_density: f64,
    pub indicators: Indicators,
    pub households: Vec<HouseholdView>,
    pub firms: Vec<FirmView>,
    pub bank: BankView,
    pub government: GovernmentView,
    pub nature: NatureView,
    pub housing: HousingView,
    pub energy: EnergyView,
    pub history: Vec<HistoryPoint>,
}

/// Housing added by one `public_housing` build. The model adds exactly `count` units of
/// public capacity per build (`sim-model/src/government.rs`), so this is 1.
pub const PUBLIC_HOUSING_PER_BUILDING: f64 = 1.0;

fn buildings_from(
    public_cap: f64,
    fossil_cap: f64,
    renewable_cap: f64,
    plant_cap: f64,
) -> Buildings {
    let count = |cap: f64, per: f64| -> u32 {
        if per > 0.0 && cap.is_finite() && cap > 0.0 {
            (cap / per).round().min(f64::from(u32::MAX)) as u32
        } else {
            0
        }
    };
    Buildings {
        public_housing: count(public_cap, PUBLIC_HOUSING_PER_BUILDING),
        fossil_plant: count(fossil_cap, plant_cap),
        renewable_plant: count(renewable_cap, plant_cap),
    }
}

pub fn buildings(sim: &Sim) -> Buildings {
    buildings_from(
        sim.state.housing.public_cap.0,
        sim.state.energy.fossil_cap.0,
        sim.state.energy.renewable_cap.0,
        sim.params.government.plant_capacity,
    )
}

/// `Some` once the game has ended: the first recorded outcome, or `survived` after the last turn.
pub fn game_over(sim: &Sim) -> Option<GameOver> {
    let tpt = sim.params.run.ticks_per_turn.max(1);
    match sim.state.outcome {
        Some((tick, o)) => Some(GameOver {
            turn: tick / tpt + 1,
            outcome: GameOutcome::from_sim(o),
        }),
        None if sim.state.turn > sim.params.run.turns => Some(GameOver {
            turn: sim.state.turn,
            outcome: GameOutcome::Plain(PlainOutcome::Survived),
        }),
        None => None,
    }
}

fn indicators(sim: &Sim) -> Indicators {
    let i = &sim.state.indicators;
    Indicators {
        gdp_annual: i.gdp_annual.dollars(),
        gdp_real_annual: i.gdp_real_annual,
        unemployment: i.unemployment.0,
        employed: i.employed.0,
        labour_force: i.labour_force.0,
        gini: i.gini.0,
        wage_share: i.wage_share.0,
        profit_share: i.profit_share.0,
        avg_wellbeing: i.avg_wellbeing.0,
        remaining_budget: i.remaining_budget.0,
        budget_share: budget_share(&sim.state, &sim.params),
        temperature: i.temperature,
        gpi_annual: i.gpi_annual,
        money_in_circulation: i.money.dollars(),
        price_index: i.price_index,
    }
}

fn policy(sim: &Sim) -> BTreeMap<LeverKey, PolicyEntry> {
    let p = &sim.state.policy;
    let e = &sim.state.effective;
    let num = |v: f64| PolicyScalar::Number(v);
    let plain = |v: f64| PolicyEntry {
        nominal: num(v),
        effective: None,
    };
    let lobbied = |n: f64, eff: f64| PolicyEntry {
        nominal: num(n),
        effective: Some(eff),
    };
    let mode = match sim.state.nature.commons_access {
        st::CommonsAccess::Open => "open",
        st::CommonsAccess::Community { .. } => "community",
        st::CommonsAccess::Enclosed(_) => "enclosed",
    };
    let mut m = BTreeMap::new();
    m.insert(
        LeverKey::CarbonPrice,
        lobbied(p.carbon_price.dollars(), e.carbon_price.dollars()),
    );
    m.insert(LeverKey::TaxWage, plain(p.tax_wage.0));
    m.insert(LeverKey::TaxProfit, lobbied(p.tax_profit.0, e.tax_profit.0));
    m.insert(LeverKey::TaxCapital, plain(p.tax_capital.0));
    m.insert(LeverKey::WealthTax, lobbied(p.wealth_tax.0, e.wealth_tax.0));
    m.insert(
        LeverKey::WealthThreshold,
        plain(p.wealth_threshold.dollars()),
    );
    m.insert(LeverKey::BaseRate, plain(p.base_rate.0));
    m.insert(LeverKey::MinWage, plain(p.min_wage.dollars()));
    m.insert(LeverKey::Benefit, plain(p.benefit.dollars()));
    m.insert(LeverKey::Ubi, plain(p.ubi.dollars()));
    m.insert(LeverKey::GreenSubsidy, plain(p.green_subsidy.0));
    m.insert(LeverKey::CampaignCap, plain(p.campaign_cap.0));
    m.insert(
        LeverKey::EmissionsCap,
        lobbied(p.emissions_cap.0, e.emissions_cap.0),
    );
    m.insert(LeverKey::CommonsHarvest, plain(p.commons_harvest.0));
    m.insert(
        LeverKey::CommonsAccess,
        PolicyEntry {
            nominal: PolicyScalar::Mode(mode.to_string()),
            effective: None,
        },
    );
    m.insert(LeverKey::BankCapitalRatio, plain(p.bank_capital_ratio.0));
    m.insert(
        LeverKey::Bailout,
        PolicyEntry {
            nominal: PolicyScalar::Bool(p.bailout),
            effective: None,
        },
    );
    m.insert(LeverKey::PublicSpending, plain(p.public_spending.dollars()));
    m.insert(LeverKey::JobGuarantee, plain(p.job_guarantee.0));
    m
}

fn households(sim: &Sim) -> Vec<HouseholdView> {
    sim.state
        .households
        .iter()
        .map(|h| HouseholdView {
            id: h.id.0,
            label: h.label.clone(),
            class: ClassKey::from_sim(h.class),
            people: h.weight.0,
            employed: h.employed().0,
            labour_force: h.labour_force().0,
            savings: h.savings.dollars(),
            debt: h.debt.dollars(),
            wealth_per_capita: h.wealth_pc.dollars(),
            income: HouseholdIncome {
                wages: h.income.wages.dollars(),
                dividends: h.income.dividends.dollars(),
                rent: h.income.rent_received.dollars(),
                interest: h.income.interest_received.dollars(),
                benefits: h.income.benefits.dollars(),
                tax_paid: h.income.tax_paid.dollars(),
                rent_paid: h.income.rent_paid.dollars(),
                loan_payments: h.income.loan_payments.dollars(),
                spending: h.income.spending.dollars(),
            },
            wellbeing: h.wellbeing.0,
            political_power: h.political_power.0,
            footprint_tonnes: h.footprint.0,
            food_met: h.food_met.0,
            housed_share: h.housed_share.0,
        })
        .collect()
}

/// Firms have no label in the sim (gap G8): name them from sector and slot order.
fn firm_names(firms: &[st::Firm]) -> Vec<String> {
    let mut goods = 0u32;
    let mut food = 0u32;
    firms
        .iter()
        .map(|f| {
            let (base, n) = match f.sector {
                st::Sector::Goods => ("Factory", &mut goods),
                st::Sector::Food => ("Farm", &mut food),
            };
            *n += 1;
            if *n == 1 {
                base.to_string()
            } else {
                format!("{base} {n}")
            }
        })
        .collect()
}

fn firms(sim: &Sim) -> Vec<FirmView> {
    let names = firm_names(&sim.state.firms);
    sim.state
        .firms
        .iter()
        .zip(names)
        .map(|(f, name)| FirmView {
            id: f.id.0,
            active: f.active,
            sector: SectorKey::from_sim(f.sector),
            name,
            owner: match f.owner {
                st::Owner::Household(h) => FirmOwner::Household { id: h.0 },
                st::Owner::Workers => FirmOwner::Workers,
                st::Owner::State => FirmOwner::State,
                st::Owner::Bank => FirmOwner::Bank,
            },
            workers: f.workers.0,
            wage: f.wage.dollars(),
            price: f.price.dollars(),
            output: f.output.0,
            emissions: f.emissions.0,
            revenue: f.revenue.dollars(),
            profit: f.profit.dollars(),
            capital: f.capital.0,
            debt: f.debt.dollars(),
            green_tech: f.green_tech.0,
        })
        .collect()
}

/// Build the full snapshot from the sim, the saved history and the metadata.
pub fn build(sim: &Sim, history: &[HistoryPoint], meta: &Meta) -> Snapshot {
    let s = &sim.state;
    let tp = &sim.params.turn;
    Snapshot {
        schema_version: crate::SCHEMA_VERSION,
        game: meta.game,
        turn: s.turn,
        tick: s.tick,
        game_over: game_over(sim),
        pc: PcView {
            current: s.political_capital,
            cap: tp.pc_cap,
            regen_next_turn: tp.pc_base * s.gov.legitimacy.0,
        },
        system: SystemView {
            current: SystemKey::from_sim(s.system),
            transition: s.transition.map(|t| TransitionView {
                to: SystemKey::from_sim(t.to),
                months_left: t.months_left,
                total: t.total,
            }),
        },
        buildings: buildings(sim),
        pending: s.pending.iter().map(PendingItem::from_sim).collect(),
        policy: policy(sim),
        net_lobby: s.net_lobby.0,
        union_density: s.union_density.0,
        indicators: indicators(sim),
        households: households(sim),
        firms: firms(sim),
        bank: BankView {
            equity: s.bank.equity.dollars(),
            loans: s.bank.loans.dollars(),
            deposits: s.bank.deposits.dollars(),
            default_rate: s.bank.default_rate.0,
            failed: s.bank.failed,
        },
        government: GovernmentView {
            cash: s.gov.cash.dollars(),
            debt: s.gov.debt.dollars(),
            debt_to_gdp: debt_to_gdp(s),
            legitimacy: s.gov.legitimacy.0,
            receipts: s.gov.receipts.dollars(),
            spending: s.gov.spending.dollars(),
        },
        nature: NatureView {
            forest: share(s.nature.forest.0, sim.params.nature.forest_max),
            commons: share(s.nature.commons.0, sim.params.nature.commons_max),
            forest_stock: s.nature.forest.0,
            commons_stock: s.nature.commons.0,
            commons_access: match s.nature.commons_access {
                st::CommonsAccess::Open => CommonsAccessKey::Open,
                st::CommonsAccess::Community { .. } => CommonsAccessKey::Community,
                st::CommonsAccess::Enclosed(_) => CommonsAccessKey::Enclosed,
            },
            cumulative_tonnes: s.nature.cumulative.0,
            net_emissions_monthly: s.nature.net_emissions.0,
            gross_emissions_monthly: s.nature.gross_emissions.0,
            sink_monthly: s.nature.sink.0,
        },
        housing: HousingView {
            private_cap: s.housing.private_cap.0,
            public_cap: s.housing.public_cap.0,
            rent_private: s.housing.rent_private.dollars(),
            rent_public: s.housing.rent_public.dollars(),
            occupancy: s.housing.occupancy.0,
            housed_share: s.housing.housed_share.0,
        },
        energy: EnergyView {
            fossil_cap: s.energy.fossil_cap.0,
            renewable_cap: s.energy.renewable_cap.0,
            fossil_share: s.energy.grid_factor(),
        },
        history: history.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stocks_become_shares_of_their_maximum() {
        assert_eq!(share(50_000.0, 100_000.0), 0.5);
        assert_eq!(share(150_000.0, 100_000.0), 1.0);
        assert_eq!(share(-1.0, 100_000.0), 0.0);
        assert_eq!(share(1.0, 0.0), 0.0);
    }

    #[test]
    fn building_counts_round_from_capacity() {
        let b = buildings_from(0.0, 100.0, 0.0, 20.0);
        assert_eq!(
            (b.public_housing, b.fossil_plant, b.renewable_plant),
            (0, 5, 0)
        );
        let b = buildings_from(3.0, 99.9, 40.0, 20.0);
        assert_eq!(
            (b.public_housing, b.fossil_plant, b.renewable_plant),
            (3, 5, 2)
        );
        assert_eq!(buildings_from(-1.0, f64::NAN, 5.0, 0.0).fossil_plant, 0);
    }
}
