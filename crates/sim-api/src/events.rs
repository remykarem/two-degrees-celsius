//! `SimEvent` and `TraceRecord` (contract section 9, item 3). Payloads only, no display text.
//! Money is in dollars. Importance (1 low, 3 high) ranks events on the results screen.
use crate::keys::{BlockReason, ClassKey, CollapseReason, LeverKey, SystemKey};
use serde::{Deserialize, Serialize};
use sim_types as st;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimEventKind {
    Commuted {
        household: u32,
        firm: u32,
        people: f64,
    },
    Hired {
        firm: u32,
        people: f64,
    },
    Fired {
        firm: u32,
        people: f64,
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
                household,
                firm,
                people,
            } => K::Commuted {
                household: household.0,
                firm: firm.0,
                people: people.0,
            },
            st::Event::Hired { firm, people } => K::Hired {
                firm: firm.0,
                people: people.0,
            },
            st::Event::Fired { firm, people } => K::Fired {
                firm: firm.0,
                people: people.0,
            },
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
        | K::SystemSwitchCompleted { .. } => 3,
        K::LoanDefaulted { .. }
        | K::FirmFounded { .. }
        | K::ForestShrank { .. }
        | K::PolicyBlocked { .. }
        | K::CommandActivated { .. }
        | K::RentShortfall { .. }
        | K::SpendingCut { .. }
        | K::ClassChanged { .. }
        | K::SystemSwitchStarted { .. } => 2,
        K::Commuted { .. } | K::Hired { .. } | K::Fired { .. } => 1,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmissionSourceKind {
    Firm,
    Households,
    Sink,
}

/// One emissions line, used by both `TraceRecord::Emissions` and `MonthFlows.emissions`.
/// The sink reports a negative tonnage (carbon absorbed), exactly as the sim does.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmissionLine {
    pub source: EmissionSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub tonnes: f64,
}

impl EmissionLine {
    pub fn from_sim(source: st::EmissionSource, tonnes: st::Tonnes) -> EmissionLine {
        let (source, id) = match source {
            st::EmissionSource::Firm(f) => (EmissionSourceKind::Firm, Some(f.0)),
            st::EmissionSource::Households => (EmissionSourceKind::Households, None),
            st::EmissionSource::Sink => (EmissionSourceKind::Sink, None),
        };
        EmissionLine {
            source,
            id,
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
    pub fn from_sim(t: &st::TraceRecord) -> TraceRecord {
        match *t {
            st::TraceRecord::Emissions { source, tonnes } => {
                let l = EmissionLine::from_sim(source, tonnes);
                TraceRecord::Emissions {
                    source: l.source,
                    id: l.id,
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
        }
    }
}
