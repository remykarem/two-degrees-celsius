//! `Api`: the `SimPort` operations over a `Sim`.
use crate::command::{Command, Tile};
use crate::content;
use crate::error::{code, ApiError, ApiResult};
use crate::events::{SimEvent, SimEventKind, TraceRecord};
use crate::frames::{self, TickCapture};
use crate::history::{self, HistoryPoint};
use crate::keys::{BlockReason, InfraKind, LeverKey};
use crate::meta::{ActionInfo, LeverInfo, Meta, Rejection};
use crate::persist;
use crate::snapshot::{self, Snapshot};
use crate::turn::{
    CancelResult, CommandPreview, MonthFrame, MonthResult, PendingItem, PolicyGap, RejectedCommand,
    TurnResult, TurnStart,
};
use sim_engine::Sim;
use sim_params::{ContentJson, Params};
use sim_types as st;

/// A running game behind the wire format.
#[derive(Clone, Debug)]
pub struct Api {
    sim: Sim,
    meta: Meta,
    history: Vec<HistoryPoint>,
    /// What the open turn has produced so far (`None` at a turn boundary).
    live: Option<Live>,
}

/// Key of a command in `params.turn.levers`.
fn param_key(c: &Command) -> String {
    match c {
        Command::SetPolicy { lever, .. } => lever.as_str().to_string(),
        Command::SwitchSystem { .. } => "switch_system".to_string(),
        Command::Place { kind, .. } => format!("place_{}", kind.as_str()),
        // What a removal costs depends on the tile; only wire-invalid removals get here.
        Command::Remove { .. } => "remove_road".to_string(),
    }
}

/// Parse a JSON array of commands (for the wasm shell, which holds no logic of its own).
pub fn parse_commands(json: &str) -> ApiResult<Vec<Command>> {
    serde_json::from_str(json)
        .map_err(|e| ApiError::new(code::BAD_REQUEST, format!("bad command list: {e}")))
}

/// A turn opened by `begin_turn`: what the months have produced so far.
#[derive(Clone, Debug)]
struct Live {
    turn: u32,
    accepted: Vec<PendingItem>,
    rejected: Vec<RejectedCommand>,
    activated: Vec<PendingItem>,
    pc_before: f64,
    /// Events raised before the first tick; they join month 0.
    pre: Vec<SimEvent>,
    months: Vec<MonthFrame>,
    trace: Vec<st::TraceRecord>,
}

/// Everything a failed call must put back.
struct Backup(st::State, sim_rng::Rng, st::Cents, sim_engine::TurnPhase);

/// Commands split into the ones the engine sees and the ones the wire layer rejects.
struct Split {
    /// Original index of each engine command.
    index: Vec<usize>,
    cmds: Vec<st::Command>,
    /// `(original index, rejection)` of wire-level rejections.
    invalid: Vec<(usize, Rejection)>,
}

impl Api {
    /// Start a new game from the embedded content.
    pub fn init(seed: u64) -> ApiResult<Api> {
        Api::from_content(&content::default_content(), content::LEVERS, seed)
    }

    /// Start a new game from explicit content (tests, tools).
    pub fn from_content(c: &ContentJson, levers_json: &str, seed: u64) -> ApiResult<Api> {
        let params = Params::from_content(c).map_err(|e| ApiError::new(code::INIT_FAILED, e))?;
        let meta = Meta::load(levers_json, &params)?;
        let sim =
            sim_engine::init(params, seed).map_err(|e| ApiError::new(code::INIT_FAILED, e))?;
        Ok(Api {
            sim,
            meta,
            history: Vec::new(),
            live: None,
        })
    }

    /// Start over with a new seed, keeping the content. Returns the first snapshot.
    pub fn reset(&mut self, seed: u64) -> ApiResult<Snapshot> {
        let sim = sim_engine::init(self.sim.params.clone(), seed)
            .map_err(|e| ApiError::new(code::INIT_FAILED, e))?;
        self.sim = sim;
        self.history.clear();
        self.live = None;
        Ok(self.snapshot())
    }

    pub fn levers(&self) -> &[LeverInfo] {
        &self.meta.levers
    }

    pub fn actions(&self) -> &[ActionInfo] {
        &self.meta.actions
    }

    pub fn snapshot(&self) -> Snapshot {
        snapshot::build(&self.sim, &self.history, &self.meta)
    }

    /// The underlying sim, read-only (tests and tools).
    pub fn sim(&self) -> &Sim {
        &self.sim
    }

    fn split(&self, cmds: &[Command]) -> Split {
        let mut s = Split {
            index: Vec::new(),
            cmds: Vec::new(),
            invalid: Vec::new(),
        };
        for (i, c) in cmds.iter().enumerate() {
            match self.meta.to_sim(c) {
                Ok(sc) => {
                    s.index.push(i);
                    s.cmds.push(sc);
                }
                Err(r) => s.invalid.push((i, r)),
            }
        }
        s
    }

    fn backup(&self) -> Backup {
        Backup(
            self.sim.state.clone(),
            self.sim.rng.clone(),
            self.sim.clamped_total,
            self.sim.turn_phase,
        )
    }

    fn restore(&mut self, b: Backup) {
        (
            self.sim.state,
            self.sim.rng,
            self.sim.clamped_total,
            self.sim.turn_phase,
        ) = (b.0, b.1, b.2, b.3);
    }

    fn turn_state(what: &str) -> ApiError {
        ApiError::new(code::TURN_STATE, what)
    }

    /// Rejections as the wire reports them: the engine's plus the wire layer's, by index.
    fn wire_rejected(
        out: &st::TurnOutput,
        split: &Split,
        cmds: &[Command],
    ) -> Vec<RejectedCommand> {
        let mut rejected: Vec<RejectedCommand> = out
            .rejected
            .iter()
            .map(|r| {
                let i = split.index[r.index];
                RejectedCommand {
                    index: i,
                    command: cmds[i],
                    reason: BlockReason::from_sim(r.reason),
                }
            })
            .collect();
        rejected.extend(split.invalid.iter().map(|(i, r)| RejectedCommand {
            index: *i,
            command: cmds[*i],
            reason: r.reason,
        }));
        rejected.sort_by_key(|r| r.index);
        rejected
    }

    /// Tiles where `kind` can be built now (empty, not scenery, nothing queued there), in
    /// row-major order. The UI shades the grid with this.
    pub fn valid_tiles(&self, kind: InfraKind) -> Vec<Tile> {
        sim_engine::turn::valid_tiles(&self.sim, kind.to_sim())
            .into_iter()
            .map(Tile::from_sim)
            .collect()
    }

    fn ensure_running(&self) -> ApiResult<()> {
        match snapshot::game_over(&self.sim) {
            Some(g) => Err(ApiError::new(
                code::GAME_OVER,
                format!("the game ended on turn {}", g.turn),
            )),
            None => Ok(()),
        }
    }

    /// Costs of `cmds` in order, as if each earlier one had been queued. Changes nothing.
    /// A command outside its lever's range (or of the wrong type) is `invalid_command`.
    pub fn preview(&self, cmds: &[Command]) -> ApiResult<Vec<CommandPreview>> {
        let split = self.split(cmds);
        let costs = sim_engine::preview(&self.sim, &split.cmds);
        let mut out: Vec<Option<CommandPreview>> = vec![None; cmds.len()];
        for (k, cost) in costs.iter().enumerate() {
            let i = split.index[k];
            out[i] = Some(CommandPreview {
                command: cmds[i],
                pc: cost.pc,
                money: cost.money.dollars(),
                lead_time: cost.lead_time,
                effective_turn: cost.effective_turn,
                opposition: cost.opposition,
                blocked: cost.blocked.map(BlockReason::from_sim),
                tile_valid: cost.tile_valid,
                tile_reason: cost.tile_reason.map(BlockReason::from_sim),
            });
        }
        for (i, rej) in &split.invalid {
            let lead = self
                .sim
                .params
                .turn
                .levers
                .get(&param_key(&cmds[*i]))
                .map_or(0, |c| c.lead_time);
            out[*i] = Some(CommandPreview {
                command: cmds[*i],
                pc: 0.0,
                money: 0.0,
                lead_time: lead,
                effective_turn: self.sim.state.turn + lead,
                opposition: 0.0,
                blocked: Some(rej.reason),
                tile_valid: rej.reason != BlockReason::TileInvalid,
                tile_reason: (rej.reason == BlockReason::TileInvalid).then_some(rej.reason),
            });
        }
        Ok(out.into_iter().flatten().collect())
    }

    /// Queue the affordable commands, play the turn (three monthly ticks) and report it.
    /// On any failure the game is left exactly as it was.
    pub fn submit_turn(&mut self, cmds: &[Command]) -> ApiResult<TurnResult> {
        self.ensure_running()?;
        if self.sim.turn_phase.open {
            return Err(Self::turn_state("a turn is open: call finishTurn first"));
        }
        let split = self.split(cmds);
        let backup = self.backup();
        let mut ticks: Vec<TickCapture> = Vec::new();
        let run = sim_engine::submit_turn_with(&mut self.sim, &split.cmds, |sim, out| {
            ticks.push(frames::capture(sim, out));
        });
        let out = match run {
            Ok(o) => o,
            Err(v) => {
                self.restore(backup);
                return Err(ApiError::new(code::VIOLATION, v.to_string()));
            }
        };
        if ticks.len() != 3 {
            self.restore(backup);
            return Err(ApiError::new(
                code::UNSUPPORTED_PARAMS,
                "a turn must have exactly 3 ticks",
            ));
        }

        // Events raised before the first tick (rejections, activations) join month 0.
        let tick_events: usize = ticks.iter().map(|t| t.sim_event_count).sum();
        let pre_n = out.events.len().saturating_sub(tick_events);
        let mut pre: Vec<SimEvent> = split
            .invalid
            .iter()
            .map(|(_, r)| SimEvent::new(SimEventKind::PolicyBlocked { reason: r.reason }))
            .collect();
        pre.extend(out.events[..pre_n].iter().map(SimEvent::from_sim));
        let months: Vec<MonthFrame> = ticks
            .into_iter()
            .enumerate()
            .map(|(m, t)| {
                let mut events = if m == 0 {
                    std::mem::take(&mut pre)
                } else {
                    Vec::new()
                };
                events.extend(t.events);
                MonthFrame {
                    month: m as u8,
                    events,
                    flows: t.flows,
                }
            })
            .collect();

        let rejected = Self::wire_rejected(&out, &split, cmds);

        self.history
            .push(history::point(&self.sim.state, &self.sim.params, out.turn));
        Ok(TurnResult {
            turn: out.turn,
            accepted: out.accepted.iter().map(PendingItem::from_sim).collect(),
            rejected,
            activated: out.activated.iter().map(PendingItem::from_sim).collect(),
            pc_before: out.pc_before,
            pc_after: out.pc_after,
            months,
            policy_gaps: out
                .policy_gaps
                .iter()
                .map(|g| PolicyGap {
                    lever: LeverKey::from_lever(g.lever),
                    nominal: g.nominal,
                    effective: g.effective,
                })
                .collect(),
            trace: out
                .trace
                .iter()
                .map(|t| TraceRecord::from_sim(t, &self.sim.state.plants))
                .collect(),
            snapshot: self.snapshot(),
        })
    }

    /// Pressure mode, step 1: queue the affordable commands and activate what is due.
    /// Opens a turn; on failure the game is left exactly as it was.
    pub fn begin_turn(&mut self, cmds: &[Command]) -> ApiResult<TurnStart> {
        self.ensure_running()?;
        if self.sim.turn_phase.open {
            return Err(Self::turn_state("a turn is already open"));
        }
        if self.sim.params.run.ticks_per_turn != 3 {
            return Err(ApiError::new(
                code::UNSUPPORTED_PARAMS,
                "a turn must have exactly 3 ticks",
            ));
        }
        let split = self.split(cmds);
        let backup = self.backup();
        let out = match sim_engine::begin_turn(&mut self.sim, &split.cmds) {
            Ok(o) => o,
            Err(v) => {
                self.restore(backup);
                return Err(ApiError::new(code::VIOLATION, v.to_string()));
            }
        };
        let mut pre: Vec<SimEvent> = split
            .invalid
            .iter()
            .map(|(_, r)| SimEvent::new(SimEventKind::PolicyBlocked { reason: r.reason }))
            .collect();
        pre.extend(out.events.iter().map(SimEvent::from_sim));
        let live = Live {
            turn: out.turn,
            accepted: out.accepted.iter().map(PendingItem::from_sim).collect(),
            rejected: Self::wire_rejected(&out, &split, cmds),
            activated: out.activated.iter().map(PendingItem::from_sim).collect(),
            pc_before: out.pc_before,
            pre,
            months: Vec::new(),
            trace: Vec::new(),
        };
        let start = TurnStart {
            turn: live.turn,
            accepted: live.accepted.clone(),
            rejected: live.rejected.clone(),
            activated: live.activated.clone(),
            pc_before: live.pc_before,
            snapshot: self.snapshot(),
        };
        self.live = Some(live);
        Ok(start)
    }

    /// Pressure mode, step 2 (three times): run one monthly tick of the open turn.
    /// A turn that ends the game still completes its remaining months and `finish_turn`.
    pub fn advance_month(&mut self) -> ApiResult<MonthResult> {
        let ph = self.sim.turn_phase;
        if !ph.open || ph.months_done >= 3 || self.live.is_none() {
            return Err(Self::turn_state(
                "advanceMonth needs an open turn with months left",
            ));
        }
        let backup = self.backup();
        let t = match sim_engine::advance_month(&mut self.sim) {
            Ok(t) => t,
            Err(v) => {
                self.restore(backup);
                return Err(ApiError::new(code::VIOLATION, v.to_string()));
            }
        };
        let cap = frames::capture(&self.sim, &t);
        let month = ph.months_done as u8;
        let snapshot = self.snapshot();
        let Some(live) = self.live.as_mut() else {
            self.restore(backup);
            return Err(Self::turn_state("no open turn"));
        };
        let mut events = if month == 0 {
            std::mem::take(&mut live.pre)
        } else {
            Vec::new()
        };
        events.extend(cap.events);
        let frame = MonthFrame {
            month,
            events,
            flows: cap.flows,
        };
        live.months.push(frame.clone());
        live.trace.extend(t.trace);
        Ok(MonthResult {
            month,
            frame,
            snapshot,
        })
    }

    /// Pressure mode, step 3: policy gaps, PC regeneration, next turn. Returns the same
    /// `TurnResult` as `submit_turn`.
    pub fn finish_turn(&mut self) -> ApiResult<TurnResult> {
        let ph = self.sim.turn_phase;
        if !ph.open || ph.months_done != 3 || self.live.is_none() {
            return Err(Self::turn_state(
                "finishTurn needs an open turn with all 3 months done",
            ));
        }
        let fin = match sim_engine::finish_turn(&mut self.sim) {
            Ok(f) => f,
            Err(v) => return Err(ApiError::new(code::VIOLATION, v.to_string())),
        };
        let Some(live) = self.live.take() else {
            return Err(Self::turn_state("no open turn"));
        };
        self.history
            .push(history::point(&self.sim.state, &self.sim.params, live.turn));
        Ok(TurnResult {
            turn: live.turn,
            accepted: live.accepted,
            rejected: live.rejected,
            activated: live.activated,
            pc_before: live.pc_before,
            pc_after: fin.pc_after,
            months: live.months,
            policy_gaps: fin
                .policy_gaps
                .iter()
                .map(|g| PolicyGap {
                    lever: LeverKey::from_lever(g.lever),
                    nominal: g.nominal,
                    effective: g.effective,
                })
                .collect(),
            trace: live
                .trace
                .iter()
                .map(|t| TraceRecord::from_sim(t, &self.sim.state.plants))
                .collect(),
            snapshot: self.snapshot(),
        })
    }

    /// Cancel a pending command; refunds its political capital share (never money).
    pub fn cancel_pending(&mut self, id: u32) -> ApiResult<CancelResult> {
        self.ensure_running()?;
        let refund_pc = sim_engine::cancel_pending(&mut self.sim, id)
            .map_err(|e| ApiError::new(code::NO_SUCH_PENDING, e))?;
        Ok(CancelResult {
            refund_pc,
            snapshot: self.snapshot(),
        })
    }

    /// Opaque save blob (state, RNG position, params hash, history).
    /// Only at a turn boundary (no turn open); otherwise `turn_state`.
    pub fn export_state(&self) -> ApiResult<String> {
        if self.sim.turn_phase.open {
            return Err(Self::turn_state(
                "exportState is only legal at a turn boundary",
            ));
        }
        persist::export(&self.sim, &self.history)
    }

    /// Restore a blob from `export_state`. On error the current game is untouched.
    pub fn import_state(&mut self, blob: &str) -> ApiResult<Snapshot> {
        let imp = persist::import(blob, &self.sim.params)?;
        for p in &imp.state.pending {
            self.meta
                .to_sim(&Command::from_sim(&p.command))
                .map_err(|e| {
                    ApiError::new(code::BAD_STATE, format!("saved pending item: {}", e.why))
                })?;
        }
        self.sim.state = imp.state;
        self.sim.rng = imp.rng;
        self.sim.initial_population = imp.initial_population;
        self.sim.clamped_total = imp.clamped_total;
        self.history = imp.history;
        // A save is always a turn boundary: drop any open turn.
        self.sim.turn_phase = sim_engine::TurnPhase::default();
        self.live = None;
        Ok(self.snapshot())
    }
}
