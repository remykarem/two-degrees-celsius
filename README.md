# Town game

A turn-based game about running a small town's economy while staying under the 1.5°C carbon budget.
Rust simulation, TypeScript/Svelte/Three.js front end.

## Run the game

You need [Node 22+](https://nodejs.org).

```
cd web
npm install
npm run dev
```

Open http://localhost:5173.

The browser currently plays against a **mock** economy. The real simulation is being moved to the
new "Town v2" model, so `npm run build:wasm` will not work until that port is finished.

## Run the simulation (no graphics)

You need [Rust](https://rustup.rs).

```
cargo run -p sim-cli -- --seed 42
```

## Test

```
cargo test --workspace          # simulation
cd web && npm test              # front end
```

## Where things are

- `crates/`: the Rust simulation
- `content/`: all game numbers (JSON)
- `web/`: the browser app
- `docs/`: design docs. Start with `SPEC.md`, then `docs/town-v2.md`
