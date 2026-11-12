//! Market trades (contract v2.4, docs/features/market-trades.md): the list behind each market.
//! The rows are output only. Shops' purchases come from `TraceRecord::Trade` (the model pushes
//! it, nothing reads it); jobs, homes and loans are read from the state after each tick plus the
//! tick's hire and fire events and loan decisions. They live in the ledger store (same clone on
//! a turn, same save blob, same `detail_turns`): full rows for the last `detail_turns` turns,
//! older turns summed per (market, parties, note, turn) with exact unit and money totals.
use crate::error::{code, ApiError, ApiResult};
use crate::markets::{
    close, fill, thousands, MarketDef, MarketEntity, MarketEntityKind, MarketView, MarketsContent,
};
use serde::{Deserialize, Serialize};
use sim_engine::StepOutput;
use sim_types::{Event, Sector, State, TraceRecord, TradeBuyer};
use std::collections::BTreeMap;

// ---------------------------------------------------------------- stored shapes

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Party {
    Nobody,
    Household(u32),
    Firm(u32),
    Person(u32),
    Home(u32),
    Government,
    Bank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Tag {
    Food,
    Goods,
    Services,
    Labour,
    Housing,
    Credit,
}

impl Tag {
    fn of(id: &str) -> Option<Tag> {
        Some(match id {
            "food" => Tag::Food,
            "goods" => Tag::Goods,
            "services" => Tag::Services,
            "labour" => Tag::Labour,
            "housing" => Tag::Housing,
            "credit" => Tag::Credit,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Note {
    None,
    Hired,
    Fired,
    Open,
    Guarantee,
    Staff,
    Vacant,
    Owner,
    Unhoused,
    Full,
    Partial,
    Refused,
    SelfBuy,
}

impl Note {
    fn id(self) -> Option<&'static str> {
        Some(match self {
            Note::None => return None,
            Note::Hired => "hired",
            Note::Fired => "fired",
            Note::Open => "open",
            Note::Guarantee => "guarantee",
            Note::Staff => "staff",
            Note::Vacant => "vacant",
            Note::Owner => "owner",
            Note::Unhoused => "unhoused",
            Note::Full => "full",
            Note::Partial => "partial",
            Note::Refused => "refused",
            Note::SelfBuy => "self",
        })
    }
}

/// One trade of one month. `a` is the buyer, tenant, worker or borrower, `b` the seller, home,
/// employer or lender, `c` the home's owner. `units`: goods units, 1 for a job or a home that is
/// counted, approved dollars of a loan. `amount`: cents (price paid, wage, rent, approved loan).
/// `extra`: cents asked for. `rate`: the loan's yearly rate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Trade {
    tick: u32,
    tag: Tag,
    a: Party,
    b: Party,
    c: Party,
    note: Note,
    counted: bool,
    units: f64,
    amount: i64,
    extra: i64,
    rate: f64,
    price: f64,
}

/// A trade seen over `count` months (the same parties and note), summed exactly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Roll {
    turn: u32,
    tag: Tag,
    a: Party,
    b: Party,
    c: Party,
    note: Note,
    counted: bool,
    units: f64,
    amount: i64,
    extra: i64,
    rate_sum: f64,
    price_sum: f64,
    count: u32,
    /// The month of a single-month row (`count == 1`); unused otherwise.
    month: u8,
}

type RollKey = (u32, Tag, Party, Party, Party, Note, bool);

impl Roll {
    fn key(&self) -> RollKey {
        (
            self.turn,
            self.tag,
            self.a,
            self.b,
            self.c,
            self.note,
            self.counted,
        )
    }
    fn one(t: &Trade) -> Roll {
        Roll {
            turn: crate::ledger::turn_of(t.tick),
            tag: t.tag,
            a: t.a,
            b: t.b,
            c: t.c,
            note: t.note,
            counted: t.counted,
            units: t.units,
            amount: t.amount,
            extra: t.extra,
            rate_sum: t.rate,
            price_sum: t.price,
            count: 1,
            month: (t.tick % 3) as u8,
        }
    }
    fn add(&mut self, o: &Roll) {
        self.units += o.units;
        self.amount += o.amount;
        self.extra += o.extra;
        self.rate_sum += o.rate_sum;
        self.price_sum += o.price_sum;
        self.count += o.count;
    }
}

fn merge(rows: impl IntoIterator<Item = Roll>) -> Vec<Roll> {
    let mut by: BTreeMap<RollKey, Roll> = BTreeMap::new();
    for r in rows {
        match by.get_mut(&r.key()) {
            Some(x) => x.add(&r),
            None => {
                by.insert(r.key(), r);
            }
        }
    }
    by.into_values().collect()
}

/// Sizes, for tests and the notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradeSize {
    pub rows: usize,
    pub rolls: usize,
}

/// The trades of a game (a field of the ledger store).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TradeStore {
    rows: Vec<Trade>,
    rolls: Vec<Roll>,
    /// Turns up to and including this one are rolled up.
    rolled_through: Option<u32>,
    last_tick: Option<u32>,
}

fn party_of(b: TradeBuyer) -> Party {
    match b {
        TradeBuyer::Household(h) => Party::Household(h.0),
        TradeBuyer::Firm(f) => Party::Firm(f.0),
        TradeBuyer::Government => Party::Government,
    }
}

impl TradeStore {
    pub fn size(&self) -> TradeSize {
        TradeSize {
            rows: self.rows.len(),
            rolls: self.rolls.len(),
        }
    }

    /// Record the tick that has just been applied (`s` is the state after it).
    pub fn record_tick(&mut self, out: &StepOutput, s: &State) {
        let tick = s.tick.saturating_sub(1);
        let mut push = |t: Trade| self.rows.push(t);
        let base = |tag: Tag, a: Party, b: Party| Trade {
            tick,
            tag,
            a,
            b,
            c: Party::Nobody,
            note: Note::None,
            counted: true,
            units: 0.0,
            amount: 0,
            extra: 0,
            rate: 0.0,
            price: 0.0,
        };
        // Goods, food and services: what the model traced.
        for t in &out.trace {
            if let TraceRecord::Trade {
                sector,
                buyer,
                firm,
                units,
                unit_price,
                amount,
            } = *t
            {
                let tag = match sector {
                    Sector::Food => Tag::Food,
                    Sector::Goods => Tag::Goods,
                    Sector::Services => Tag::Services,
                };
                push(Trade {
                    units,
                    amount: amount.0,
                    price: unit_price,
                    note: if buyer == TradeBuyer::Firm(firm) {
                        Note::SelfBuy
                    } else {
                        Note::None
                    },
                    ..base(tag, party_of(buyer), Party::Firm(firm.0))
                });
            }
        }
        // Labour: every job in the labour force on a firm's books, then what is not counted.
        let hired: Vec<u32> = out
            .events
            .iter()
            .filter_map(|e| match e {
                Event::PersonHired { person, .. } => Some(person.0),
                _ => None,
            })
            .collect();
        for p in &s.persons {
            if let (true, None, Some(f)) = (p.in_labour_force, p.staff_of, p.job) {
                push(Trade {
                    units: 1.0,
                    amount: p.wage.0,
                    note: if hired.contains(&p.id.0) {
                        Note::Hired
                    } else {
                        Note::None
                    },
                    ..base(Tag::Labour, Party::Person(p.id.0), Party::Firm(f.0))
                });
            } else if p.jg && p.in_labour_force {
                push(Trade {
                    counted: false,
                    amount: p.wage.0,
                    note: Note::Guarantee,
                    ..base(Tag::Labour, Party::Person(p.id.0), Party::Government)
                });
            } else if let Some(h) = p.staff_of {
                push(Trade {
                    counted: false,
                    amount: p.wage.0,
                    note: Note::Staff,
                    ..base(Tag::Labour, Party::Person(p.id.0), Party::Household(h.0))
                });
            }
        }
        for e in &out.events {
            if let Event::PersonFired { person, firm } = e {
                let still = s
                    .persons
                    .get(person.idx())
                    .is_some_and(|p| p.job == Some(*firm));
                if !still {
                    push(Trade {
                        counted: false,
                        note: Note::Fired,
                        ..base(Tag::Labour, Party::Person(person.0), Party::Firm(firm.0))
                    });
                }
            }
        }
        for t in &out.trace {
            if let TraceRecord::Vacancies {
                firm,
                posted,
                filled,
            } = *t
            {
                for _ in 0..posted.saturating_sub(filled) {
                    push(Trade {
                        counted: false,
                        note: Note::Open,
                        ..base(Tag::Labour, Party::Nobody, Party::Firm(firm.0))
                    });
                }
            }
        }
        // Housing: every home, then the households that have none.
        for h in &s.homes {
            let owner = match h.owner {
                sim_types::HomeOwner::Household(o) => Party::Household(o.0),
                sim_types::HomeOwner::Public => Party::Government,
            };
            let own = matches!((h.owner, h.household), (sim_types::HomeOwner::Household(o), Some(t)) if o == t);
            push(match h.household {
                Some(t) => Trade {
                    c: owner,
                    units: 1.0,
                    amount: if own { 0 } else { h.rent.0 },
                    note: if own { Note::Owner } else { Note::None },
                    ..base(Tag::Housing, Party::Household(t.0), Party::Home(h.id.0))
                },
                None => Trade {
                    c: owner,
                    counted: false,
                    note: Note::Vacant,
                    ..base(Tag::Housing, Party::Nobody, Party::Home(h.id.0))
                },
            });
        }
        for h in s.households.iter().filter(|h| h.home.is_none()) {
            push(Trade {
                counted: false,
                note: Note::Unhoused,
                ..base(Tag::Housing, Party::Household(h.id.0), Party::Nobody)
            });
        }
        // Credit: the firms that asked.
        for t in &out.trace {
            if let TraceRecord::CreditDecision {
                firm,
                requested,
                approved,
            } = *t
            {
                if requested.0 > 0 || approved.0 > 0 {
                    push(Trade {
                        units: approved.0 as f64 / 100.0,
                        amount: approved.0,
                        extra: requested.0,
                        rate: s.firms.get(firm.idx()).map_or(0.0, |f| f.loan_rate.0),
                        note: if approved.0 <= 0 {
                            Note::Refused
                        } else if approved.0 < requested.0 {
                            Note::Partial
                        } else {
                            Note::Full
                        },
                        ..base(Tag::Credit, Party::Firm(firm.0), Party::Bank)
                    });
                }
            }
        }
        self.last_tick = Some(tick);
    }

    /// Sum the turns up to and including `through` into rolls and drop their rows.
    pub fn roll_to(&mut self, through: u32) {
        if self.rolled_through.is_some_and(|t| t >= through) {
            return;
        }
        let split = self
            .rows
            .partition_point(|t| crate::ledger::turn_of(t.tick) <= through);
        let old: Vec<Trade> = self.rows.drain(..split).collect();
        let all = self.rolls.drain(..).chain(old.iter().map(Roll::one));
        self.rolls = merge(all);
        self.rolled_through = Some(through);
    }

    fn rows_of(&self, turn: u32, month: Option<u8>) -> Vec<Roll> {
        if self.rolled_through.is_some_and(|t| turn <= t) {
            return self
                .rolls
                .iter()
                .filter(|r| r.turn == turn)
                .cloned()
                .collect();
        }
        let rows = self.rows.iter().filter(|t| {
            crate::ledger::turn_of(t.tick) == turn && month.is_none_or(|m| (t.tick % 3) as u8 == m)
        });
        match month {
            Some(_) => rows.map(Roll::one).collect(),
            None => merge(rows.map(Roll::one)),
        }
    }

    /// Periods that can be shown, newest first: each turn as a whole (when all three months are
    /// recorded or it is rolled up), and each month of the detail window.
    fn periods(&self) -> Vec<(u32, Option<u8>, bool)> {
        let mut out: Vec<(u32, Option<u8>, bool)> = Vec::new();
        let mut turns: Vec<u32> = self.rolls.iter().map(|r| r.turn).collect();
        turns.dedup();
        let mut detail: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
        for t in &self.rows {
            let m = detail.entry(crate::ledger::turn_of(t.tick)).or_default();
            let mo = (t.tick % 3) as u8;
            if !m.contains(&mo) {
                m.push(mo);
            }
        }
        let mut all: Vec<u32> = turns.into_iter().chain(detail.keys().copied()).collect();
        all.sort_unstable();
        all.dedup();
        for turn in all.into_iter().rev() {
            match detail.get(&turn) {
                None => out.push((turn, None, true)),
                Some(months) => {
                    if months.len() == 3 {
                        out.push((turn, None, false));
                    }
                    let mut ms = months.clone();
                    ms.sort_unstable();
                    for m in ms.into_iter().rev() {
                        out.push((turn, Some(m), false));
                    }
                }
            }
        }
        out
    }

    /// The latest recorded `(turn, month)`.
    fn latest(&self) -> Option<(u32, u8)> {
        self.last_tick
            .map(|t| (crate::ledger::turn_of(t), (t % 3) as u8))
    }
}

// ---------------------------------------------------------------- wire shapes

/// What `marketTrades` is asked: all fields optional. Default: the latest month, every row.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradesQuery {
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub turn: Option<u32>,
    /// 0..=2. Absent with a `turn`: the whole turn.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub month: Option<u8>,
    /// Only trades this party took part in.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub entity: Option<MarketEntity>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeColumn {
    pub id: String,
    pub label: String,
    pub help: String,
    pub numeric: bool,
}

/// One cell, already written out. `sort` is the number to sort a numeric or dated column by; a
/// text cell sorts by its text. `entity` makes the text a link to the inspector.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeCell {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sort: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub entity: Option<MarketEntity>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeRow {
    /// One cell per column, in column order.
    pub cells: Vec<TradeCell>,
    /// Counted in the totals (and so in the market card's figure).
    pub counted: bool,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePeriod {
    pub turn: u32,
    /// `None`: the whole turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "0 | 1 | 2"))]
    pub month: Option<u8>,
    /// An older turn kept only as sums.
    pub rolled: bool,
    pub label: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeTotals {
    /// Counted rows.
    pub count: u32,
    /// Counted units (for a loan market: approved dollars).
    pub units: f64,
    /// Dollars: money paid, wages, rent or lent.
    pub money: f64,
    /// The sentence from the content.
    pub text: String,
    /// The totals row, one cell per column.
    pub cells: Vec<TradeCell>,
}

/// The market card's own figure for the month, and whether the counted rows add up to it.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeAggregate {
    pub traded: f64,
    /// For example `Bought: 545`.
    pub text: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketTrades {
    pub market: String,
    pub label: String,
    pub intro: String,
    pub empty: String,
    pub uncounted: String,
    pub period: TradePeriod,
    pub periods: Vec<TradePeriod>,
    pub columns: Vec<TradeColumn>,
    pub rows: Vec<TradeRow>,
    pub totals: TradeTotals,
    /// Set for the latest month with every row shown (the figure the card has).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub aggregate: Option<TradeAggregate>,
    /// `Some` with `aggregate`: the counted rows add up to the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reconciles: Option<bool>,
}

// ---------------------------------------------------------------- building

fn dollars(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let c = cents.unsigned_abs();
    format!("{sign}${}.{:02}", thousands((c / 100) as i64), c % 100)
}

fn units_text(v: f64) -> String {
    let v = if v.abs() < 0.005 { 0.0 } else { v };
    let whole = v.trunc();
    format!(
        "{}.{:02}",
        thousands(whole as i64),
        ((v - whole).abs() * 100.0).round() as i64 % 100
    )
}

fn plain(text: impl Into<String>) -> TradeCell {
    TradeCell {
        text: text.into(),
        sort: None,
        entity: None,
    }
}

fn number(text: impl Into<String>, sort: f64) -> TradeCell {
    TradeCell {
        text: text.into(),
        sort: Some(sort),
        entity: None,
    }
}

struct Ctx<'a> {
    s: &'a State,
    firms: &'a BTreeMap<u32, String>,
    common: &'a crate::markets::TradesCommon,
}

impl Ctx<'_> {
    fn name(&self, key: &str) -> String {
        self.common.names.get(key).cloned().unwrap_or_default()
    }
    fn party(&self, p: Party, nobody: &str) -> TradeCell {
        let e = |kind, id| Some(MarketEntity { kind, id });
        let (text, entity) = match p {
            Party::Nobody => (self.name(nobody), None),
            Party::Household(h) => (
                self.s
                    .households
                    .get(h as usize)
                    .map_or(format!("Household {h}"), |x| x.label.clone()),
                e(MarketEntityKind::Household, h),
            ),
            Party::Firm(f) => (
                self.firms
                    .get(&f)
                    .cloned()
                    .unwrap_or_else(|| format!("Firm {f}")),
                e(MarketEntityKind::Firm, f),
            ),
            Party::Person(x) => (
                self.s
                    .persons
                    .get(x as usize)
                    .map_or(format!("Person {x}"), |p| p.label.clone()),
                e(MarketEntityKind::Person, x),
            ),
            Party::Home(n) => (
                fill(&self.name("home"), &[("n", (n + 1).to_string())]),
                e(MarketEntityKind::Home, n),
            ),
            Party::Government => (self.name("government"), e(MarketEntityKind::Government, 0)),
            Party::Bank => (self.name("bank"), e(MarketEntityKind::Bank, 0)),
        };
        TradeCell {
            text,
            sort: None,
            entity,
        }
    }
    fn month(&self, r: &Roll) -> TradeCell {
        let text = if r.count == 1 {
            fill(
                &self.common.month_word,
                &[("month", (r.month + 1).to_string())],
            )
        } else {
            fill(
                &self.common.quarter_word,
                &[("months", r.count.to_string())],
            )
        };
        number(text, f64::from(r.turn * 3 + u32::from(r.month)))
    }
}

fn involves(r: &Roll, e: MarketEntity) -> bool {
    let want = match e.kind {
        MarketEntityKind::Household => Party::Household(e.id),
        MarketEntityKind::Firm => Party::Firm(e.id),
        MarketEntityKind::Person => Party::Person(e.id),
        MarketEntityKind::Home => Party::Home(e.id),
        MarketEntityKind::Government => Party::Government,
        MarketEntityKind::Bank => Party::Bank,
    };
    r.a == want || r.b == want || r.c == want
}

fn cells_for(ctx: &Ctx, def: &MarketDef, r: &Roll) -> Vec<TradeCell> {
    let note = r
        .note
        .id()
        .and_then(|k| def.trades.notes.get(k).cloned())
        .unwrap_or_default();
    let per_month = |cents: i64| cents / i64::from(r.count.max(1));
    let money = |cents: i64| number(dollars(cents), cents as f64 / 100.0);
    def.trades
        .columns
        .iter()
        .map(|c| match c.id.as_str() {
            "buyer" | "borrower" => ctx.party(r.a, "nobody"),
            "worker" => ctx.party(r.a, "open_job"),
            "tenant" => ctx.party(r.a, "nobody"),
            "seller" | "employer" | "home" => ctx.party(r.b, "nobody"),
            "lender" => ctx.party(r.b, "bank"),
            "owner" => ctx.party(r.c, "nobody"),
            "units" => number(units_text(r.units), r.units),
            "unit_price" => {
                let p = if r.count == 1 {
                    r.price_sum / 100.0
                } else if r.units > 0.0 {
                    r.amount as f64 / 100.0 / r.units
                } else {
                    0.0
                };
                number(format!("${p:.2}"), p)
            }
            "total" => money(r.amount),
            "wage" | "rent" => money(per_month(r.amount)),
            "asked" => money(r.extra),
            "approved" => money(r.amount),
            "rate" => {
                let v = r.rate_sum / f64::from(r.count.max(1));
                number(
                    fill(&ctx.common.rate, &[("rate", format!("{:.2}", v * 100.0))]),
                    v,
                )
            }
            "note" => plain(note.clone()),
            "month" => ctx.month(r),
            _ => plain(String::new()),
        })
        .collect()
}

fn period_of(common: &crate::markets::TradesCommon, p: (u32, Option<u8>, bool)) -> TradePeriod {
    let (turn, month, rolled) = p;
    let t = turn.to_string();
    let label = match month {
        Some(m) => fill(
            &common.period_month,
            &[("turn", t), ("month", (m + 1).to_string())],
        ),
        None if rolled => fill(&common.period_rolled, &[("turn", t)]),
        None => fill(&common.period_turn, &[("turn", t)]),
    };
    TradePeriod {
        turn,
        month,
        rolled,
        label,
    }
}

impl TradeStore {
    /// The list behind one market for one period. `view` is the market's card (for the figure the
    /// rows must add up to); `firms` maps firm ids to their display names.
    pub fn build(
        &self,
        content: &MarketsContent,
        def: &MarketDef,
        view: Option<&MarketView>,
        s: &State,
        firms: &BTreeMap<u32, String>,
        q: &TradesQuery,
    ) -> ApiResult<MarketTrades> {
        let ctx = Ctx {
            s,
            firms,
            common: &content.trades,
        };
        let columns: Vec<TradeColumn> = def
            .trades
            .columns
            .iter()
            .map(|c| TradeColumn {
                id: c.id.clone(),
                label: c.label.clone(),
                help: c.help.clone(),
                numeric: c.numeric,
            })
            .collect();
        let periods: Vec<TradePeriod> = self
            .periods()
            .into_iter()
            .map(|p| period_of(&content.trades, p))
            .collect();
        let latest = self.latest();
        let want = match (q.turn, q.month) {
            (Some(t), m) => Some((t, m)),
            (None, Some(m)) => latest.map(|(t, _)| (t, Some(m))),
            (None, None) => latest.map(|(t, m)| (t, Some(m))),
        };
        let tag = Tag::of(&def.id);
        let (rows, period) = match (want, tag) {
            (Some((turn, month)), Some(tag)) => {
                let found = self
                    .periods()
                    .into_iter()
                    .find(|p| p.0 == turn && (p.1 == month || (p.2 && month.is_some())))
                    .ok_or_else(|| {
                        ApiError::new(
                            code::BAD_REQUEST,
                            format!("no trades recorded for turn {turn}"),
                        )
                    })?;
                let rows: Vec<Roll> = self
                    .rows_of(turn, month)
                    .into_iter()
                    .filter(|r| r.tag == tag)
                    .collect();
                (rows, period_of(&content.trades, found))
            }
            _ => (
                Vec::new(),
                period_of(
                    &content.trades,
                    (want.map_or(0, |w| w.0), want.and_then(|w| w.1), false),
                ),
            ),
        };
        let all_rows = q.entity.is_none();
        let rows: Vec<Roll> = rows
            .into_iter()
            .filter(|r| q.entity.is_none_or(|e| involves(r, e)))
            .collect();
        let (mut count, mut units, mut cents, mut asked) = (0u32, 0.0f64, 0i64, 0i64);
        let mut out_rows = Vec::with_capacity(rows.len());
        for r in &rows {
            if r.counted {
                count += 1;
                units += r.units;
                cents += r.amount;
                asked += r.extra;
            }
            out_rows.push(TradeRow {
                cells: cells_for(&ctx, def, r),
                counted: r.counted,
            });
        }
        let span = match period.month {
            Some(_) => content.trades.span_month.clone(),
            None => fill(&content.trades.span_months, &[("months", "3".to_string())]),
        };
        let text = if def.trades.totals.is_empty() || rows.is_empty() {
            String::new()
        } else {
            fill(
                &def.trades.totals,
                &[
                    ("count", count.to_string()),
                    ("units", units_text(units)),
                    ("money", dollars(cents)),
                    ("span", span),
                ],
            )
        };
        let total_cells: Vec<TradeCell> = def
            .trades
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| match c.id.as_str() {
                "units" => number(units_text(units), units),
                "total" | "wage" | "rent" | "approved" => {
                    number(dollars(cents), cents as f64 / 100.0)
                }
                "asked" => number(dollars(asked), asked as f64 / 100.0),
                _ if i == 0 => plain(count.to_string()),
                _ => plain(String::new()),
            })
            .collect();
        let is_latest =
            all_rows && latest.is_some_and(|(t, m)| period.turn == t && period.month == Some(m));
        let (aggregate, reconciles) = match (view, is_latest && tag.is_some()) {
            (Some(v), true) => {
                let counts = matches!(tag, Some(Tag::Labour | Tag::Housing));
                let got = if counts { f64::from(count) } else { units };
                (
                    Some(TradeAggregate {
                        traded: v.traded,
                        text: format!("{}: {}", v.traded_label, v.traded_text),
                    }),
                    Some(close(got, v.traded)),
                )
            }
            _ => (None, None),
        };
        Ok(MarketTrades {
            market: def.id.clone(),
            label: def.label.clone(),
            intro: def.trades.intro.clone(),
            empty: def.trades.empty.clone(),
            uncounted: if rows.iter().any(|r| !r.counted) {
                def.trades.uncounted.clone()
            } else {
                String::new()
            },
            period,
            periods,
            columns,
            rows: out_rows,
            totals: TradeTotals {
                count,
                units,
                money: cents as f64 / 100.0,
                text,
                cells: total_cells,
            },
            aggregate,
            reconciles,
        })
    }
}
