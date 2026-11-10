//! Transaction ledgers (contract section 14). The engine returns every money movement of a tick
//! as a `Transaction`; this module keeps them in the API wrapper (never in the model `State`):
//! full detail for the last `detail_turns` turns, older periods rolled up to monthly summaries
//! per (account, counterparty, kind) with exact totals, opening balances at turn 0 and the
//! closing balances of every turn. Everything is saved with the game.
//!
//! Balances: see `sim_types::ledger` (balance = savings, cash or bank equity; debt = debt or
//! bank loans). A page's `reconciles` is `opening + roll-ups + entries == the state's balance`,
//! computed from the stored data against the live state, for both quantities.
use crate::error::{code, ApiError, ApiResult};
use crate::events::{SimEvent, SimEventKind};
use serde::{Deserialize, Serialize};
use sim_engine::StepOutput;
use sim_types::{
    effects_of, Balances, Cents, External, LedgerAccount, LedgerKind, Position, Posting, State,
};

// ---------------------------------------------------------------- wire shapes

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AccountRef {
    Household { id: u32 },
    Firm { id: u32 },
    Government,
    Bank,
    External { label: String },
}

impl AccountRef {
    pub fn from_sim(a: LedgerAccount) -> AccountRef {
        match a {
            LedgerAccount::Household(h) => AccountRef::Household { id: h.0 },
            LedgerAccount::Firm(f) => AccountRef::Firm { id: f.0 },
            LedgerAccount::Government => AccountRef::Government,
            LedgerAccount::Bank => AccountRef::Bank,
            LedgerAccount::External(e) => AccountRef::External {
                label: e.label().to_string(),
            },
        }
    }

    /// The account in the model, if it exists in `s` and can be queried (not external).
    pub fn to_sim(&self, s: &State) -> ApiResult<LedgerAccount> {
        let bad = |m: String| ApiError::new(code::BAD_REQUEST, m);
        match self {
            AccountRef::Household { id } if (*id as usize) < s.households.len() => {
                Ok(LedgerAccount::Household(sim_types::HouseholdId(*id)))
            }
            AccountRef::Firm { id } if (*id as usize) < s.firms.len() => {
                Ok(LedgerAccount::Firm(sim_types::FirmId(*id)))
            }
            AccountRef::Government => Ok(LedgerAccount::Government),
            AccountRef::Bank => Ok(LedgerAccount::Bank),
            AccountRef::External { .. } => Err(bad(
                "external accounts have no ledger of their own".to_string(),
            )),
            AccountRef::Household { id } => Err(bad(format!("no household {id}"))),
            AccountRef::Firm { id } => Err(bad(format!("no firm {id}"))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerKindKey {
    Wage,
    Dividend,
    Rent,
    Benefit,
    Tax,
    Purchase,
    Lobby,
    Interest,
    LoanNew,
    LoanRepay,
    LoanWriteoff,
    Build,
    Upkeep,
    Subsidy,
    Bailout,
    WealthConsumption,
    Neutral,
}

impl LedgerKindKey {
    pub fn from_sim(k: LedgerKind) -> LedgerKindKey {
        match k {
            LedgerKind::Wage => LedgerKindKey::Wage,
            LedgerKind::Dividend => LedgerKindKey::Dividend,
            LedgerKind::Rent => LedgerKindKey::Rent,
            LedgerKind::Benefit => LedgerKindKey::Benefit,
            LedgerKind::Tax => LedgerKindKey::Tax,
            LedgerKind::Purchase => LedgerKindKey::Purchase,
            LedgerKind::Lobby => LedgerKindKey::Lobby,
            LedgerKind::Interest => LedgerKindKey::Interest,
            LedgerKind::LoanNew => LedgerKindKey::LoanNew,
            LedgerKind::LoanRepay => LedgerKindKey::LoanRepay,
            LedgerKind::LoanWriteoff => LedgerKindKey::LoanWriteoff,
            LedgerKind::Build => LedgerKindKey::Build,
            LedgerKind::Upkeep => LedgerKindKey::Upkeep,
            LedgerKind::Subsidy => LedgerKindKey::Subsidy,
            LedgerKind::Bailout => LedgerKindKey::Bailout,
            LedgerKind::WealthConsumption => LedgerKindKey::WealthConsumption,
            LedgerKind::Neutral => LedgerKindKey::Neutral,
        }
    }
    pub fn to_sim(self) -> LedgerKind {
        match self {
            LedgerKindKey::Wage => LedgerKind::Wage,
            LedgerKindKey::Dividend => LedgerKind::Dividend,
            LedgerKindKey::Rent => LedgerKind::Rent,
            LedgerKindKey::Benefit => LedgerKind::Benefit,
            LedgerKindKey::Tax => LedgerKind::Tax,
            LedgerKindKey::Purchase => LedgerKind::Purchase,
            LedgerKindKey::Lobby => LedgerKind::Lobby,
            LedgerKindKey::Interest => LedgerKind::Interest,
            LedgerKindKey::LoanNew => LedgerKind::LoanNew,
            LedgerKindKey::LoanRepay => LedgerKind::LoanRepay,
            LedgerKindKey::LoanWriteoff => LedgerKind::LoanWriteoff,
            LedgerKindKey::Build => LedgerKind::Build,
            LedgerKindKey::Upkeep => LedgerKind::Upkeep,
            LedgerKindKey::Subsidy => LedgerKind::Subsidy,
            LedgerKindKey::Bailout => LedgerKind::Bailout,
            LedgerKindKey::WealthConsumption => LedgerKind::WealthConsumption,
            LedgerKindKey::Neutral => LedgerKind::Neutral,
        }
    }
}

/// `query` of `ledger(account, query)`: all fields optional (turns inclusive).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerQuery {
    #[serde(default)]
    pub from_turn: Option<u32>,
    #[serde(default)]
    pub to_turn: Option<u32>,
    #[serde(default)]
    pub kind: Option<LedgerKindKey>,
}

impl LedgerQuery {
    fn turn_ok(&self, turn: u32) -> bool {
        self.from_turn.is_none_or(|f| turn >= f) && self.to_turn.is_none_or(|t| turn <= t)
    }
    fn kind_ok(&self, k: LedgerKind) -> bool {
        self.kind.is_none_or(|q| q.to_sim() == k)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub turn: u32,
    pub month: u8,
    pub tick: u32,
    pub from: AccountRef,
    pub to: AccountRef,
    pub amount: f64,
    pub kind: LedgerKindKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub memo: String,
    /// The queried account's balance (savings, cash, bank equity) after this entry.
    pub balance_after: f64,
    /// Contract v1.4 addition: its debt (bank: loans) after this entry.
    pub debt_after: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerSummaryRow {
    pub turn: u32,
    pub month: u8,
    pub counterparty: AccountRef,
    pub kind: LedgerKindKey,
    pub inflow: f64,
    pub outflow: f64,
    pub count: u32,
    /// Contract v1.4 addition: the net effect of the row on the account's balance and debt.
    pub net: f64,
    pub debt_net: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerPage {
    pub account: AccountRef,
    /// At turn 0 (the start of the game).
    pub opening_balance: f64,
    pub opening_debt: f64,
    pub entries: Vec<LedgerEntry>,
    pub summary: Vec<LedgerSummaryRow>,
    pub closing_balance: f64,
    pub closing_debt: f64,
    pub reconciles: bool,
    pub detail_from_turn: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonLedger {
    pub person: u32,
    pub household: u32,
    /// Wages earned (credited to the household), detail window only.
    pub earnings: Vec<LedgerEntry>,
    /// Contract v1.4 addition: older earnings, one row per month and firm.
    pub summary: Vec<LedgerSummaryRow>,
    /// Hired, fired and commute changes, oldest first.
    pub events: Vec<SimEvent>,
    /// Contract v1.4 addition: the turn of each event.
    pub event_turns: Vec<u32>,
}

// ---------------------------------------------------------------- stored shapes

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    tick: u32,
    from: LedgerAccount,
    to: LedgerAccount,
    amount: i64,
    kind: LedgerKind,
    detail: String,
    posting: Posting,
}

impl Entry {
    fn turn(&self) -> u32 {
        turn_of(self.tick)
    }
    fn effects(&self) -> Vec<(LedgerAccount, Position)> {
        effects_of(self.posting, self.from, self.to, Cents(self.amount))
    }
    fn effect_on(&self, a: LedgerAccount) -> Option<Position> {
        self.effects()
            .into_iter()
            .find(|(x, _)| *x == a)
            .map(|x| x.1)
    }
}

/// One roll-up row: all entries of one month with the same sender, receiver, kind and posting,
/// summed exactly. A row is stored once (not per account); each account's view of it is derived
/// from its posting, which is linear in the amount, so account totals stay exact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct RollRow {
    turn: u32,
    month: u8,
    from: LedgerAccount,
    to: LedgerAccount,
    kind: LedgerKind,
    posting: Posting,
    amount: i64,
    count: u32,
}

impl RollRow {
    fn key(&self) -> (u32, u8, LedgerAccount, LedgerAccount, LedgerKind, Posting) {
        (
            self.turn,
            self.month,
            self.from,
            self.to,
            self.kind,
            self.posting,
        )
    }
    fn effects(&self) -> Vec<(LedgerAccount, Position)> {
        effects_of(self.posting, self.from, self.to, Cents(self.amount))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Earn {
    tick: u32,
    person: u32,
    household: u32,
    firm: Option<u32>,
    amount: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct EarnRoll {
    person: u32,
    turn: u32,
    month: u8,
    firm: Option<u32>,
    amount: i64,
    count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct PersonEvent {
    tick: u32,
    kind: SimEventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TurnClose {
    turn: u32,
    balances: Balances,
}

/// The game's first turn (`State::turn` starts at 1; tick 0 is its first month).
pub const FIRST_TURN: u32 = 1;

/// The game turn a tick belongs to.
pub fn turn_of(tick: u32) -> u32 {
    tick / 3 + FIRST_TURN
}

/// The most person events kept (oldest dropped first), so memory stays bounded.
const MAX_PERSON_EVENTS: usize = 2000;

/// Sizes, for tests and the notes (not part of the wire).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedgerSize {
    pub entries: usize,
    pub rollups: usize,
    pub earnings: usize,
    pub earning_rollups: usize,
    pub person_events: usize,
    pub closings: usize,
}

/// The ledger of a game. Lives in `Api`, is saved in the export blob, and is never read by the
/// model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerStore {
    detail_turns: u32,
    /// Positions at turn 0.
    opening: Balances,
    /// Positions at the start of the detail window: `opening` plus every rolled-up entry.
    window_base: Balances,
    /// Positions after the last recorded tick (`window_base` plus the window's entries).
    current: Balances,
    /// Turns up to and including this one are rolled up (`None`: nothing rolled yet).
    rolled_through: Option<u32>,
    last_tick: Option<u32>,
    entries: Vec<Entry>,
    rollups: Vec<RollRow>,
    earnings: Vec<Earn>,
    earning_rollups: Vec<EarnRoll>,
    person_events: Vec<PersonEvent>,
    /// `(person, firm, tiles, minutes)` of the last commute event stored per person.
    last_commute: Vec<(u32, u32, u32, u32)>,
    closings: Vec<TurnClose>,
}

impl LedgerStore {
    /// A new store for a game whose state is `s` (turn 0, before any tick).
    pub fn new(s: &State, detail_turns: u32) -> LedgerStore {
        let b = Balances::of(s);
        LedgerStore {
            detail_turns: detail_turns.max(2),
            opening: b.clone(),
            window_base: b.clone(),
            current: b,
            rolled_through: None,
            last_tick: None,
            entries: Vec::new(),
            rollups: Vec::new(),
            earnings: Vec::new(),
            earning_rollups: Vec::new(),
            person_events: Vec::new(),
            last_commute: Vec::new(),
            closings: Vec::new(),
        }
    }

    pub fn size(&self) -> LedgerSize {
        LedgerSize {
            entries: self.entries.len(),
            rollups: self.rollups.len(),
            earnings: self.earnings.len(),
            earning_rollups: self.earning_rollups.len(),
            person_events: self.person_events.len(),
            closings: self.closings.len(),
        }
    }

    pub fn detail_turns(&self) -> u32 {
        self.detail_turns
    }

    /// First turn whose entries are kept in full.
    pub fn detail_from_turn(&self) -> u32 {
        self.rolled_through.map_or(FIRST_TURN, |t| t + 1)
    }

    pub fn last_tick(&self) -> Option<u32> {
        self.last_tick
    }

    /// Record the tick that has just been applied (`s` is the state after it). Fails, leaving
    /// the store unusable (the caller discards it by restoring its backup), if the recorded
    /// transactions do not account for the state's balances.
    pub fn record_tick(&mut self, out: &StepOutput, s: &State) -> Result<(), String> {
        let tick = s.tick.saturating_sub(1);
        for t in &out.transactions {
            self.entries.push(Entry {
                tick: t.tick,
                from: t.from,
                to: t.to,
                amount: t.amount.0,
                kind: t.kind,
                detail: t.detail.to_string(),
                posting: t.posting,
            });
            self.current.post(t);
        }
        let now = Balances::of(s);
        if self.current != now {
            let what = self
                .current
                .accounts()
                .into_iter()
                .zip(now.accounts())
                .find(|(a, b)| a.1 != b.1)
                .map(|(a, b)| {
                    format!(
                        "{}: ledger says {}/{} cents, state has {}/{}",
                        a.0.label(),
                        a.1.balance.0,
                        a.1.debt.0,
                        b.1.balance.0,
                        b.1.debt.0
                    )
                })
                .unwrap_or_default();
            return Err(format!(
                "ledger does not match the state at tick {tick}: {what}"
            ));
        }
        for e in &out.earnings {
            self.earnings.push(Earn {
                tick: e.tick,
                person: e.person.0,
                household: e.household.0,
                firm: e.firm.map(|f| f.0),
                amount: e.amount.0,
            });
        }
        for ev in &out.events {
            self.note_person_event(tick, ev);
        }
        if tick % 3 == 2 {
            self.closings.push(TurnClose {
                turn: turn_of(tick),
                balances: now,
            });
        }
        self.last_tick = Some(tick);
        self.roll_to(turn_of(tick));
        Ok(())
    }

    fn note_person_event(&mut self, tick: u32, e: &sim_types::Event) {
        let kind = match SimEvent::from_sim(e).kind {
            k @ (SimEventKind::PersonHired { .. } | SimEventKind::PersonFired { .. }) => k,
            k @ SimEventKind::Commuted {
                person,
                firm,
                tiles,
                minutes,
            } => {
                let same = self.last_commute.contains(&(person, firm, tiles, minutes));
                if same {
                    return;
                }
                self.last_commute.retain(|c| c.0 != person);
                self.last_commute.push((person, firm, tiles, minutes));
                k
            }
            _ => return,
        };
        self.person_events.push(PersonEvent { tick, kind });
        if self.person_events.len() > MAX_PERSON_EVENTS {
            let drop = self.person_events.len() - MAX_PERSON_EVENTS;
            self.person_events.drain(..drop);
        }
    }

    /// Keep full detail only for the last `detail_turns` turns up to `current_turn`.
    pub fn roll_to(&mut self, current_turn: u32) {
        let through = match current_turn.checked_sub(self.detail_turns) {
            Some(t) if t >= FIRST_TURN => t,
            _ => return,
        };
        if self.rolled_through.is_some_and(|t| t >= through) {
            return;
        }
        let split = self.entries.partition_point(|e| e.turn() <= through);
        let old: Vec<Entry> = self.entries.drain(..split).collect();
        for e in &old {
            self.roll_entry(e);
        }
        let split = self
            .earnings
            .partition_point(|e| turn_of(e.tick) <= through);
        let old: Vec<Earn> = self.earnings.drain(..split).collect();
        for e in old {
            let (turn, month) = (turn_of(e.tick), (e.tick % 3) as u8);
            match self.earning_rollups.iter_mut().find(|r| {
                r.person == e.person && r.turn == turn && r.month == month && r.firm == e.firm
            }) {
                Some(r) => {
                    r.amount += e.amount;
                    r.count += 1;
                }
                None => self.earning_rollups.push(EarnRoll {
                    person: e.person,
                    turn,
                    month,
                    firm: e.firm,
                    amount: e.amount,
                    count: 1,
                }),
            }
        }
        self.rolled_through = Some(through);
    }

    fn roll_entry(&mut self, e: &Entry) {
        for (acct, eff) in e.effects() {
            self.window_base.add(acct, eff);
        }
        let row = RollRow {
            turn: e.turn(),
            month: (e.tick % 3) as u8,
            from: e.from,
            to: e.to,
            kind: e.kind,
            posting: e.posting,
            amount: e.amount,
            count: 1,
        };
        match self.rollups.binary_search_by(|r| r.key().cmp(&row.key())) {
            Ok(i) => {
                self.rollups[i].amount += row.amount;
                self.rollups[i].count += 1;
            }
            Err(i) => self.rollups.insert(i, row),
        }
    }

    /// Check that the stored data is internally consistent with `s`: opening plus roll-ups plus
    /// the window's entries is the state's position, for every account (used on import).
    pub fn verify(&self, s: &State) -> Result<(), String> {
        let mut b = self.opening.clone();
        for r in &self.rollups {
            for (a, eff) in r.effects() {
                b.add(a, eff);
            }
        }
        if b != self.window_base {
            return Err("roll-ups do not add up to the window's opening positions".into());
        }
        for e in &self.entries {
            for (a, eff) in e.effects() {
                b.add(a, eff);
            }
        }
        if b != self.current {
            return Err(
                "opening, roll-ups and entries do not add up to the recorded closing positions"
                    .into(),
            );
        }
        if b != Balances::of(s) {
            return Err("the ledger does not match the saved state's balances".into());
        }
        Ok(())
    }

    /// Change the detail window (config) and roll up what no longer fits.
    pub fn set_detail_turns(&mut self, n: u32, current_turn: u32) {
        self.detail_turns = n.max(2);
        self.roll_to(current_turn);
    }

    // ------------------------------------------------------------ pages

    pub fn page(&self, acct: LedgerAccount, q: &LedgerQuery, s: &State) -> LedgerPage {
        let opening = self.opening.get(acct).unwrap_or(Position::ZERO);
        let closing = Balances::of(s).get(acct).unwrap_or(Position::ZERO);
        let mut run = self.window_base.get(acct).unwrap_or(Position::ZERO);
        let mut entries = Vec::new();
        for e in &self.entries {
            let Some(eff) = e.effect_on(acct) else {
                continue;
            };
            run += eff;
            if q.turn_ok(e.turn()) && q.kind_ok(e.kind) {
                entries.push(LedgerEntry {
                    turn: e.turn(),
                    month: (e.tick % 3) as u8,
                    tick: e.tick,
                    from: AccountRef::from_sim(e.from),
                    to: AccountRef::from_sim(e.to),
                    amount: Cents(e.amount).dollars(),
                    kind: LedgerKindKey::from_sim(e.kind),
                    detail: Some(e.detail.clone()),
                    memo: format!("{}: {} -> {}", e.detail, e.from.label(), e.to.label()),
                    balance_after: run.balance.dollars(),
                    debt_after: run.debt.dollars(),
                });
            }
        }
        let mut summary: Vec<LedgerSummaryRow> = self
            .rollups
            .iter()
            .filter(|r| q.turn_ok(r.turn) && q.kind_ok(r.kind))
            .filter_map(|r| {
                let eff = r.effects().into_iter().find(|(a, _)| *a == acct)?.1;
                // The other end of the row; for the bank as a bystander of a loan, the borrower.
                let counterparty = if acct == r.from {
                    r.to
                } else if acct == r.to {
                    r.from
                } else if matches!(r.from, LedgerAccount::External(_)) {
                    r.to
                } else {
                    r.from
                };
                Some(LedgerSummaryRow {
                    turn: r.turn,
                    month: r.month,
                    counterparty: AccountRef::from_sim(counterparty),
                    kind: LedgerKindKey::from_sim(r.kind),
                    inflow: if acct == r.to {
                        Cents(r.amount).dollars()
                    } else {
                        0.0
                    },
                    outflow: if acct == r.from {
                        Cents(r.amount).dollars()
                    } else {
                        0.0
                    },
                    count: r.count,
                    net: eff.balance.dollars(),
                    debt_net: eff.debt.dollars(),
                })
            })
            .collect();
        summary.sort_by(|a, b| {
            (a.turn, a.month, &a.counterparty, a.kind).cmp(&(
                b.turn,
                b.month,
                &b.counterparty,
                b.kind,
            ))
        });
        LedgerPage {
            account: AccountRef::from_sim(acct),
            opening_balance: opening.balance.dollars(),
            opening_debt: opening.debt.dollars(),
            entries,
            summary,
            closing_balance: closing.balance.dollars(),
            closing_debt: closing.debt.dollars(),
            // Opening, plus every roll-up (folded into the window base), plus every entry.
            reconciles: run == closing && self.rolled_base_ok(acct),
            detail_from_turn: self.detail_from_turn(),
        }
    }

    /// `window_base` for `acct` equals `opening` plus the account's roll-up rows.
    fn rolled_base_ok(&self, acct: LedgerAccount) -> bool {
        let mut p = self.opening.get(acct).unwrap_or(Position::ZERO);
        for r in &self.rollups {
            if let Some((_, eff)) = r.effects().into_iter().find(|(a, _)| *a == acct) {
                p += eff;
            }
        }
        p == self.window_base.get(acct).unwrap_or(Position::ZERO)
    }

    pub fn person_ledger(&self, person: u32, household: u32, s: &State) -> PersonLedger {
        let hh = LedgerAccount::Household(sim_types::HouseholdId(household));
        // The household's balance at the end of each tick of the window, for `balanceAfter`.
        let mut by_tick: Vec<(u32, Position)> = Vec::new();
        let mut run = self.window_base.get(hh).unwrap_or(Position::ZERO);
        for e in &self.entries {
            if let Some(eff) = e.effect_on(hh) {
                run += eff;
                match by_tick.last_mut() {
                    Some(l) if l.0 == e.tick => l.1 = run,
                    _ => by_tick.push((e.tick, run)),
                }
            }
        }
        let after = |tick: u32| {
            by_tick
                .iter()
                .rev()
                .find(|(t, _)| *t <= tick)
                .map_or(self.window_base.get(hh).unwrap_or(Position::ZERO), |x| x.1)
        };
        let label = s
            .persons
            .get(person as usize)
            .map_or_else(|| format!("person {person}"), |x| x.label.clone());
        let firm_acct = |f: Option<u32>| match f {
            Some(f) => LedgerAccount::Firm(sim_types::FirmId(f)),
            None => LedgerAccount::Government,
        };
        let earnings = self
            .earnings
            .iter()
            .filter(|e| e.person == person)
            .map(|e| {
                let from = firm_acct(e.firm);
                let p = after(e.tick);
                LedgerEntry {
                    turn: turn_of(e.tick),
                    month: (e.tick % 3) as u8,
                    tick: e.tick,
                    from: AccountRef::from_sim(from),
                    to: AccountRef::from_sim(hh),
                    amount: Cents(e.amount).dollars(),
                    kind: LedgerKindKey::Wage,
                    detail: Some("wages earned".to_string()),
                    memo: format!(
                        "wages earned by {label}: {} -> {}",
                        from.label(),
                        hh.label()
                    ),
                    balance_after: p.balance.dollars(),
                    debt_after: p.debt.dollars(),
                }
            })
            .collect();
        let summary = self
            .earning_rollups
            .iter()
            .filter(|r| r.person == person)
            .map(|r| LedgerSummaryRow {
                turn: r.turn,
                month: r.month,
                counterparty: AccountRef::from_sim(firm_acct(r.firm)),
                kind: LedgerKindKey::Wage,
                inflow: Cents(r.amount).dollars(),
                outflow: 0.0,
                count: r.count,
                net: Cents(r.amount).dollars(),
                debt_net: 0.0,
            })
            .collect();
        let mine: Vec<&PersonEvent> = self
            .person_events
            .iter()
            .filter(|e| match e.kind {
                SimEventKind::PersonHired { person: p, .. }
                | SimEventKind::PersonFired { person: p, .. }
                | SimEventKind::Commuted { person: p, .. } => p == person,
                _ => false,
            })
            .collect();
        PersonLedger {
            person,
            household,
            earnings,
            summary,
            events: mine.iter().map(|e| SimEvent::new(e.kind)).collect(),
            event_turns: mine.iter().map(|e| turn_of(e.tick)).collect(),
        }
    }

    // ------------------------------------------------------------ for finance

    /// Window entries of ticks `from..=to`, as plain tuples for the finance aggregation.
    pub(crate) fn window(
        &self,
        from_tick: u32,
        to_tick: u32,
    ) -> impl Iterator<Item = FinEntry<'_>> {
        self.entries
            .iter()
            .filter(move |e| e.tick >= from_tick && e.tick <= to_tick)
            .map(|e| FinEntry {
                from: e.from,
                to: e.to,
                amount: Cents(e.amount),
                kind: e.kind,
                detail: e.detail.as_str(),
                posting: e.posting,
            })
    }
}

/// A stored entry, for aggregation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FinEntry<'a> {
    pub from: LedgerAccount,
    pub to: LedgerAccount,
    pub amount: Cents,
    pub kind: LedgerKind,
    pub detail: &'a str,
    pub posting: Posting,
}

/// Is this the account name the wire uses for credit creation (used in tests)?
pub fn credit_creation() -> AccountRef {
    AccountRef::from_sim(LedgerAccount::External(External::CreditCreation))
}
