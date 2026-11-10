//! `SimEvent` and `TraceRecord` (contract section 9, item 3). Payloads only, no display text.
//! Money is in dollars. Importance (1 low, 3 high) ranks events on the results screen.
use crate::command::Tile;
use crate::keys::{BlockReason, ClassKey, CollapseReason, InfraKind, LeverKey, SystemKey};
use serde::{Deserialize, Serialize};
use sim_types as st;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimEventKind {
    Commuted {
        person: u32,
        firm: u32,
        tiles: u32,
        minutes: u32,
    },
    PersonHired {
        person: u32,
        firm: u32,
    },
    PersonFired {
        person: u32,
        firm: u32,
    },
    Purchase {
        household: u32,
        firm: u32,
        amount: f64,
    },
    /// `to` is the id of the home the household moved into.
    HouseholdMoved {
        household: u32,
        to: u32,
    },
    HouseholdEvicted {
        household: u32,
    },
    Built {
        kind: InfraKind,
        tile: Tile,
    },
    Removed {
        tile: Tile,
    },
    ElectricityRationed {
        served: f64,
    },
    LoanDefaulted {
        firm: u32,
    },
    FirmFounded {
        firm: u32,
    },
    FirmClosed {
        firm: u32,
    },
    ForestShrank {
        stock: f64,
    },
    CommonsCollapsed,
    CommonsEnclosed {
        owner: u32,
    },
    PolicyBlocked {
        reason: BlockReason,
    },
    CommandActivated {
        id: u32,
    },
    BankFailed,
    RentShortfall {
        household: u32,
        amount: f64,
    },
    SpendingCut {
        shortfall: f64,
    },
    BankBailout {
        amount: f64,
    },
    ClassChanged {
        household: u32,
        to: ClassKey,
    },
    SystemSwitchStarted {
        to: SystemKey,
    },
    SystemSwitchCompleted {
        to: SystemKey,
    },
    BudgetExhausted,
    Collapse {
        reason: CollapseReason,
    },
    /// Contract v1.2 (weather).
    FireStarted {
        tile: Tile,
    },
    FireSpread {
        from: Tile,
        to: Tile,
    },
    FireOut {
        tile: Tile,
    },
    ForestBurned {
        tile: Tile,
    },
    DroughtStarted {
        severity: f64,
        months: u32,
    },
    DroughtEnded,
    HomeDamaged {
        home: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimEvent {
    pub importance: u8,
    #[serde(flatten)]
    pub kind: SimEventKind,
}

impl SimEvent {
    pub fn new(kind: SimEventKind) -> SimEvent {
        SimEvent {
            importance: importance(&kind),
            kind,
        }
    }

    pub fn from_sim(e: &st::Event) -> SimEvent {
        use SimEventKind as K;
        let kind = match *e {
            st::Event::Commuted {
                person,
                firm,
                tiles,
                minutes,
            } => K::Commuted {
                person: person.0,
                firm: firm.0,
                tiles,
                minutes,
            },
            st::Event::PersonHired { person, firm } => K::PersonHired {
                person: person.0,
                firm: firm.0,
            },
            st::Event::PersonFired { person, firm } => K::PersonFired {
                person: person.0,
                firm: firm.0,
            },
            st::Event::Purchase {
                household,
                firm,
                amount,
            } => K::Purchase {
                household: household.0,
                firm: firm.0,
                amount: amount.dollars(),
            },
            st::Event::HouseholdMoved { household, home } => K::HouseholdMoved {
                household: household.0,
                to: home.0,
            },
            st::Event::HouseholdEvicted { household } => K::HouseholdEvicted {
                household: household.0,
            },
            st::Event::Built { kind, tile } => K::Built {
                kind: InfraKind::from_sim(kind),
                tile: Tile::from_sim(tile),
            },
            st::Event::Removed { tile } => K::Removed {
                tile: Tile::from_sim(tile),
            },
            st::Event::ElectricityRationed { served } => {
                K::ElectricityRationed { served: served.0 }
            }
            st::Event::LoanDefaulted { firm } => K::LoanDefaulted { firm: firm.0 },
            st::Event::FirmFounded { firm } => K::FirmFounded { firm: firm.0 },
            st::Event::FirmClosed { firm } => K::FirmClosed { firm: firm.0 },
            st::Event::ForestShrank { stock } => K::ForestShrank { stock: stock.0 },
            st::Event::CommonsCollapsed => K::CommonsCollapsed,
            st::Event::CommonsEnclosed { owner } => K::CommonsEnclosed { owner: owner.0 },
            st::Event::PolicyBlocked { reason } => K::PolicyBlocked {
                reason: BlockReason::from_sim(reason),
            },
            st::Event::CommandActivated { id } => K::CommandActivated { id },
            st::Event::BankFailed => K::BankFailed,
            st::Event::RentShortfall { household, amount } => K::RentShortfall {
                household: household.0,
                amount: amount.dollars(),
            },
            st::Event::SpendingCut { shortfall } => K::SpendingCut {
                shortfall: shortfall.dollars(),
            },
            st::Event::BankBailout { amount } => K::BankBailout {
                amount: amount.dollars(),
            },
            st::Event::ClassChanged { household, to } => K::ClassChanged {
                household: household.0,
                to: ClassKey::from_sim(to),
            },
            st::Event::SystemSwitchStarted { to } => K::SystemSwitchStarted {
                to: SystemKey::from_sim(to),
            },
            st::Event::SystemSwitchCompleted { to } => K::SystemSwitchCompleted {
                to: SystemKey::from_sim(to),
            },
            st::Event::BudgetExhausted => K::BudgetExhausted,
            st::Event::Collapse { reason } => K::Collapse {
                reason: CollapseReason::from_sim(reason),
            },
            st::Event::FireStarted { tile } => K::FireStarted {
                tile: Tile::from_sim(tile),
            },
            st::Event::FireSpread { from, to } => K::FireSpread {
                from: Tile::from_sim(from),
                to: Tile::from_sim(to),
            },
            st::Event::FireOut { tile } => K::FireOut {
                tile: Tile::from_sim(tile),
            },
            st::Event::ForestBurned { tile } => K::ForestBurned {
                tile: Tile::from_sim(tile),
            },
            st::Event::DroughtStarted { severity, months } => K::DroughtStarted {
                severity: severity.0,
                months,
            },
            st::Event::DroughtEnded => K::DroughtEnded,
            st::Event::HomeDamaged { home } => K::HomeDamaged { home: home.0 },
        };
        SimEvent::new(kind)
    }
}

/// Ranking used by the results screen: 3 = game-shaping, 2 = notable, 1 = routine churn.
pub fn importance(k: &SimEventKind) -> u8 {
    use SimEventKind as K;
    match k {
        K::BankFailed
        | K::Collapse { .. }
        | K::BudgetExhausted
        | K::CommonsCollapsed
        | K::CommonsEnclosed { .. }
        | K::BankBailout { .. }
        | K::FirmClosed { .. }
        | K::HouseholdEvicted { .. }
        | K::DroughtStarted { .. }
        | K::ForestBurned { .. }
        | K::SystemSwitchCompleted { .. } => 3,
        K::LoanDefaulted { .. }
        | K::FirmFounded { .. }
        | K::ForestShrank { .. }
        | K::PolicyBlocked { .. }
        | K::CommandActivated { .. }
        | K::RentShortfall { .. }
        | K::SpendingCut { .. }
        | K::ClassChanged { .. }
        | K::HouseholdMoved { .. }
        | K::PersonFired { .. }
        | K::Built { .. }
        | K::Removed { .. }
        | K::ElectricityRationed { .. }
        | K::FireStarted { .. }
        | K::HomeDamaged { .. }
        | K::SystemSwitchStarted { .. } => 2,
        K::Commuted { .. }
        | K::PersonHired { .. }
        | K::Purchase { .. }
        | K::FireSpread { .. }
        | K::FireOut { .. }
        | K::DroughtEnded => 1,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmissionSourceKind {
    Firm,
    Households,
    Plant,
    Sink,
    /// Sink capacity lost to fire; informational, not a term of net emissions.
    FireSinkLoss,
}

/// One emissions line, used by both `TraceRecord::Emissions` and `MonthFlows.emissions`.
/// The sink reports a negative tonnage (carbon absorbed), exactly as the sim does.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmissionLine {
    pub source: EmissionSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// Set for `plant` sources: the tile the plant stands on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile: Option<Tile>,
    pub tonnes: f64,
}

impl EmissionLine {
    /// `plants` maps plant ids to tiles (the sim's trace carries only the plant id).
    pub fn from_sim(
        source: st::EmissionSource,
        tonnes: st::Tonnes,
        plants: &[st::Plant],
    ) -> EmissionLine {
        let (source, id, tile) = match source {
            st::EmissionSource::Firm(f) => (EmissionSourceKind::Firm, Some(f.0), None),
            st::EmissionSource::Households => (EmissionSourceKind::Households, None, None),
            st::EmissionSource::Plant(p) => (
                EmissionSourceKind::Plant,
                Some(p.0),
                plants
                    .iter()
                    .find(|x| x.id == p)
                    .map(|x| Tile::from_sim(x.tile)),
            ),
            st::EmissionSource::Sink => (EmissionSourceKind::Sink, None, None),
            st::EmissionSource::FireSinkLoss => (EmissionSourceKind::FireSinkLoss, None, None),
        };
        EmissionLine {
            source,
            id,
            tile,
            tonnes: tonnes.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncomeKindKey {
    Wages,
    Dividends,
    Rent,
    Interest,
    Benefits,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceRecord {
    Emissions {
        source: EmissionSourceKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tile: Option<Tile>,
        tonnes: f64,
    },
    Income {
        household: u32,
        kind: IncomeKindKey,
        amount: f64,
    },
    PolicyGap {
        lever: LeverKey,
        nominal: f64,
        effective: f64,
        #[serde(rename = "netLobby")]
        net_lobby: f64,
    },
}

impl TraceRecord {
    pub fn from_sim(t: &st::TraceRecord, plants: &[st::Plant]) -> Option<TraceRecord> {
        Some(match *t {
            st::TraceRecord::Emissions { source, tonnes } => {
                let l = EmissionLine::from_sim(source, tonnes, plants);
                TraceRecord::Emissions {
                    source: l.source,
                    id: l.id,
                    tile: l.tile,
                    tonnes: l.tonnes,
                }
            }
            st::TraceRecord::Income {
                household,
                kind,
                amount,
            } => TraceRecord::Income {
                household: household.0,
                kind: match kind {
                    st::IncomeKind::Wages => IncomeKindKey::Wages,
                    st::IncomeKind::Dividends => IncomeKindKey::Dividends,
                    st::IncomeKind::Rent => IncomeKindKey::Rent,
                    st::IncomeKind::Interest => IncomeKindKey::Interest,
                    st::IncomeKind::Benefits => IncomeKindKey::Benefits,
                },
                amount: amount.dollars(),
            },
            st::TraceRecord::PolicyGap {
                lever,
                nominal,
                effective,
                net_lobby,
            } => TraceRecord::PolicyGap {
                lever: LeverKey::from_lever(lever),
                nominal,
                effective,
                net_lobby: net_lobby.0,
            },
            // Internal bookkeeping for the breakdowns; not part of the wire trace.
            st::TraceRecord::Legitimacy(_) => return None,
        })
    }
}
