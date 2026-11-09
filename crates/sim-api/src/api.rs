//! `Api`: the `SimPort` operations over a `Sim`.
use crate::command::Command;
use crate::content;
use crate::error::{code, ApiError, ApiResult};
use crate::events::{SimEvent, SimEventKind, TraceRecord};
use crate::frames::{self, TickCapture};
use crate::history::{self, HistoryPoint};
use crate::keys::{BlockReason, LeverKey};
use crate::meta::{ActionInfo, LeverInfo, Meta};
use crate::persist;
use crate::snapshot::{self, Snapshot};
use crate::turn::{
    CancelResult, CommandPreview, MonthFrame, PendingItem, PolicyGap, RejectedCommand, TurnResult,
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
}

/// Key of a command in `params.turn.levers`.
fn param_key(c: &Command) -> String {
    match c {
        Command::SetPolicy { lever, .. } => lever.as_str().to_string(),
        Command::SwitchSystem { .. } => "switch_system".to_string(),
        Command::Place { kind, .. } => format!("place_{}", kind.as_str()),
        Command::Remove { kind, .. } => format!("remove_{}", kind.as_str()),
    }
}

/// Parse a JSON array of commands (for the wasm shell, which holds no logic of its own).
pub fn parse_commands(json: &str) -> ApiResult<Vec<Command>> {
    serde_json::from_str(json)
        .map_err(|e| ApiError::new(code::BAD_REQUEST, format!("bad command list: {e}")))
}

/// Commands split into the ones the engine sees and the ones the wire layer rejects.
struct Split {
    /// Original index of each engine command.
    index: Vec<usize>,
    cmds: Vec<st::Command>,
    /// `(original index, reason text)` of wire-level rejections.
    invalid: Vec<(usize, String)>,
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
        })
    }

    /// Start over with a new seed, keeping the content. Returns the first snapshot.
    pub fn reset(&mut self, seed: u64) -> ApiResult<Snapshot> {
        let sim = sim_engine::init(self.sim.params.clone(), seed)
            .map_err(|e| ApiError::new(code::INIT_FAILED, e))?;
        self.sim = sim;
        self.history.clear();
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
                Err(why) => s.invalid.push((i, why)),
            }
        }
        s
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
            });
        }
        for (i, _) in &split.invalid {
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
                blocked: Some(BlockReason::InvalidCommand),
            });
        }
        Ok(out.into_iter().flatten().collect())
    }

    /// Queue the affordable commands, play the turn (three monthly ticks) and report it.
    /// On any failure the game is left exactly as it was.
    pub fn submit_turn(&mut self, cmds: &[Command]) -> ApiResult<TurnResult> {
        self.ensure_running()?;
        let split = self.split(cmds);
        let backup = (
            self.sim.state.clone(),
            self.sim.rng.clone(),
            self.sim.clamped_total,
        );
        let mut ticks: Vec<TickCapture> = Vec::new();
        let run = sim_engine::submit_turn_with(&mut self.sim, &split.cmds, |sim, out| {
            ticks.push(frames::capture(sim, out));
        });
        let out = match run {
            Ok(o) => o,
            Err(v) => {
                (self.sim.state, self.sim.rng, self.sim.clamped_total) = backup;
                return Err(ApiError::new(code::VIOLATION, v.to_string()));
            }
        };
        if ticks.len() != 3 {
            (self.sim.state, self.sim.rng, self.sim.clamped_total) = backup;
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
            .map(|_| {
                SimEvent::new(SimEventKind::PolicyBlocked {
                    reason: BlockReason::InvalidCommand,
                })
            })
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
        rejected.extend(split.invalid.iter().map(|(i, _)| RejectedCommand {
            index: *i,
            command: cmds[*i],
            reason: BlockReason::InvalidCommand,
        }));
        rejected.sort_by_key(|r| r.index);

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
            trace: out.trace.iter().map(TraceRecord::from_sim).collect(),
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
    pub fn export_state(&self) -> ApiResult<String> {
        persist::export(&self.sim, &self.history)
    }

    /// Restore a blob from `export_state`. On error the current game is untouched.
    pub fn import_state(&mut self, blob: &str) -> ApiResult<Snapshot> {
        let imp = persist::import(blob, &self.sim.params)?;
        for p in &imp.state.pending {
            self.meta
                .to_sim(&Command::from_sim(&p.command))
                .map_err(|e| ApiError::new(code::BAD_STATE, format!("saved pending item: {e}")))?;
        }
        self.sim.state = imp.state;
        self.sim.rng = imp.rng;
        self.sim.initial_population = imp.initial_population;
        self.sim.clamped_total = imp.clamped_total;
        self.history = imp.history;
        Ok(self.snapshot())
    }
}
