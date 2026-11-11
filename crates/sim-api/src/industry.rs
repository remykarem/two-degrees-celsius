//! `snapshot.industry` (contract v1.6): private firms with their ownership split, public
//! infrastructure and public goods and services.
use crate::finance::Finance;
use crate::keys::SectorKey;
use crate::snapshot::firm_names;
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types as st;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    Household,
    Workers,
    State,
    Bank,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnerShare {
    pub owner: OwnerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub household: Option<u32>,
    pub share: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryFirm {
    pub firm: u32,
    pub name: String,
    pub sector: SectorKey,
    pub output: f64,
    pub price: f64,
    pub workers: f64,
    pub wage: f64,
    pub revenue: f64,
    pub profit: f64,
    pub emissions: f64,
    pub ownership: Vec<OwnerShare>,
    pub state_share: f64,
    /// Additive: false for the dormant slot.
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfraKindKey {
    Road,
    PublicHousing,
    FossilPlant,
    RenewablePlant,
    FireStation,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Infrastructure {
    pub kind: InfraKindKey,
    pub count: u32,
    /// Dollars to build one.
    pub build_cost: f64,
    /// Dollars a month for all of them.
    pub upkeep_monthly: f64,
    /// Additive: dollars a month for one.
    pub upkeep_each_monthly: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicGood {
    pub id: String,
    pub label: String,
    pub spending_monthly: f64,
    pub buys: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Industry {
    pub firms: Vec<IndustryFirm>,
    pub infrastructure: Vec<Infrastructure>,
    #[serde(rename = "publicGoods")]
    pub public_goods: Vec<PublicGood>,
    pub note: String,
}

fn ownership(sim: &Sim, f: &st::Firm) -> Vec<OwnerShare> {
    let s = &sim.state;
    match f.owner {
        st::Owner::Household(h) => vec![OwnerShare {
            owner: OwnerKind::Household,
            household: Some(h.0),
            share: 1.0,
        }],
        st::Owner::Bank => vec![OwnerShare {
            owner: OwnerKind::Bank,
            household: Some(s.bank.owner.0),
            share: 1.0,
        }],
        st::Owner::State => vec![OwnerShare {
            owner: OwnerKind::State,
            household: None,
            share: 1.0,
        }],
        st::Owner::Workers => {
            let workers: Vec<&st::Person> =
                s.persons.iter().filter(|x| x.job == Some(f.id)).collect();
            let total = workers.len() as f64;
            let mut by: Vec<(u32, f64)> = Vec::new();
            for w in workers {
                match by.iter_mut().find(|(h, _)| *h == w.household.0) {
                    Some(e) => e.1 += 1.0,
                    None => by.push((w.household.0, 1.0)),
                }
            }
            by.sort_by_key(|(h, _)| *h);
            if total > 0.0 {
                by.into_iter()
                    .map(|(h, n)| OwnerShare {
                        owner: OwnerKind::Workers,
                        household: Some(h),
                        share: n / total,
                    })
                    .collect()
            } else {
                vec![OwnerShare {
                    owner: OwnerKind::Workers,
                    household: None,
                    share: 1.0,
                }]
            }
        }
    }
}

pub fn build(sim: &Sim, f: &Finance) -> Industry {
    let s = &sim.state;
    let names = firm_names(&s.firms);
    let firms = s
        .firms
        .iter()
        .zip(names)
        .map(|(x, name)| {
            let own = ownership(sim, x);
            let state_share = own
                .iter()
                .filter(|o| o.owner == OwnerKind::State)
                .map(|o| o.share)
                .sum();
            IndustryFirm {
                firm: x.id.0,
                name,
                sector: SectorKey::from_sim(x.sector),
                output: x.output.0,
                price: x.price.dollars(),
                workers: x.workers.0,
                wage: x.wage.dollars(),
                revenue: x.revenue.dollars(),
                profit: x.profit.dollars(),
                emissions: x.emissions.0,
                ownership: own,
                state_share,
                active: x.active,
            }
        })
        .collect();
    let g = &sim.params.government;
    let w = &sim.params.weather;
    let count = |k: st::TileKind| s.map.cells.iter().filter(|c| c.kind == k).count() as u32;
    let public_homes = s
        .homes
        .iter()
        .filter(|h| h.owner == st::HomeOwner::Public)
        .count() as u32;
    let infra = |kind, n: u32, cost: f64, each: f64| Infrastructure {
        kind,
        count: n,
        build_cost: cost,
        upkeep_monthly: each * f64::from(n),
        upkeep_each_monthly: each,
    };
    let infrastructure = vec![
        infra(
            InfraKindKey::Road,
            count(st::TileKind::Road),
            g.road_cost_usd,
            g.road_upkeep_usd,
        ),
        infra(
            InfraKindKey::PublicHousing,
            public_homes,
            g.public_housing_cost_usd,
            g.housing_upkeep_usd,
        ),
        infra(
            InfraKindKey::FossilPlant,
            count(st::TileKind::FossilPlant),
            g.plant_cost_usd,
            g.plant_upkeep_usd,
        ),
        infra(
            InfraKindKey::RenewablePlant,
            count(st::TileKind::RenewablePlant),
            g.plant_cost_usd,
            g.plant_upkeep_usd,
        ),
        infra(
            InfraKindKey::FireStation,
            count(st::TileKind::FireStation),
            w.fire_station_cost_usd,
            w.fire_station_upkeep_usd,
        ),
    ];
    let sp = &f.government.spending;
    let services = sp.public_spending * sim.params.goods.gov_services_share;
    let pg = |id: &str, label: &str, v: f64, buys: &str| PublicGood {
        id: id.into(),
        label: label.into(),
        spending_monthly: v,
        buys: buys.into(),
    };
    let public_goods = vec![
        pg("goods_purchases", "Government purchases of goods", sp.public_spending - services, "Goods from the Factory, bought with the public spending lever. Split with services by a fixed share; this is derived, the ledger records one purchase line."),
        pg("services", "Government purchases of services", services, "Services from the Shop, the other part of the public spending lever."),
        pg("public_works", "Public works (upkeep and construction)", sp.upkeep + sp.builds, "Upkeep of roads, public housing, power plants and fire stations, resilience spending and new builds; paid out as wages to households."),
        pg("job_guarantee", "Job guarantee", sp.job_guarantee, "Government jobs at the minimum wage for unemployed people who want one."),
        pg("benefits", "Benefits", sp.benefits, "Payments to unemployed adults and retired people."),
        pg("ubi", "Universal basic income", sp.ubi, "A flat payment to every person."),
        pg("subsidy", "Green investment subsidy", sp.subsidy, "Part of firms' green investment costs."),
    ];
    Industry {
        firms,
        infrastructure,
        public_goods,
        note: "No state shareholding in the current economic systems (Market, Regulated, Co-op): the state's ownership share is 0% for every firm. The state-owned variant exists in the model but no system uses it yet.".into(),
    }
}
