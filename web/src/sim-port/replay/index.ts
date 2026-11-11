import type { Command, Recording, Result, Snapshot } from '../../contract';
import type { DemoInfo, SimPort } from '../port';

/**
 * A SimPort that replays a recorded run of the REAL simulation (docs/taxonomy.md section 8.3). It is the fallback when the wasm package is not
 * available. Every reply is data the Rust simulation produced for a fixed script (content/demo_script.json, generated into recording.generated.ts by
 * `cargo run -p sim-api --example recording`); nothing here simulates. It cannot answer "what if", so calls that would diverge from the recording
 * return an error result with a plain explanation, and `demo` tells the UI to say so and to disable those controls.
 */
export const DEMO_UNAVAILABLE = 'demo_unavailable';

const refuse = (what: string): { ok: false; error: { code: string; message: string } } => ({
  ok: false, error: { code: DEMO_UNAVAILABLE, message: `${what} is not available in the recorded demo. Build the real simulation (npm run build:wasm in web/) to play your own game.` },
});

export function createReplayPort(rec: Recording): SimPort {
  let next = 0;
  const demo: DemoInfo = {
    label: 'Recorded demo',
    note: `This is a recording of the real simulation (seed ${rec.seed}, ${rec.turns.length} turns, with a fixed set of policies and buildings). Your choices cannot change it, so the policy and building controls are off. Press End turn to play the next recorded turn. Build the real simulation to play your own game.`,
    seed: rec.seed,
    lastTurn: rec.turns.length,
  };
  const ok = <T>(value: T): Result<T> => ({ ok: true, value });
  return {
    demo,
    liveTurn: false,
    init(): Result<Snapshot> { next = 0; return ok(rec.initial); },
    levers: () => rec.levers,
    actions: () => rec.actions,
    preview: () => refuse('Pricing a plan'),
    validTiles: () => [],
    submitTurn(commands: Command[]) {
      if (commands.length) return refuse('Changing the plan');
      const t = rec.turns[next];
      if (!t) return { ok: false, error: { code: 'demo_end', message: 'The recording ends here. Build the real simulation (npm run build:wasm in web/) to keep playing.' } };
      next += 1;
      return ok(t);
    },
    beginTurn: () => refuse('The live clock'),
    advanceMonth: () => refuse('The live clock'),
    finishTurn: () => refuse('The live clock'),
    cancelPending: () => refuse('Cancelling a queued item'),
    ledger: () => refuse('The transaction ledger'),
    personLedger: () => refuse('The transaction ledger'),
    assumptions: () => ok(rec.assumptions),
    agents: () => ok(rec.agents),
    catalog: () => ok(rec.catalog),
    exportState: () => refuse('Saving'),
    importState: () => refuse('Loading a save'),
  };
}

/** Loads the recording (a large file, split out of the main bundle) and wraps it. */
export async function loadReplayPort(): Promise<SimPort> {
  const { RECORDING } = await import('./recording.generated');
  return createReplayPort(RECORDING);
}
