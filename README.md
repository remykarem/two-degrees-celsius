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

To play against the real Rust simulation in the browser, build it first (needs Rust, the
`wasm32-unknown-unknown` target and `wasm-bindgen-cli`; the script prints the exact install command):

```
npm run build:wasm
```

Without it the app falls back to a **recorded demo**: a replay of one real game (seed 7, 12 turns) badged "Recorded demo"; your policy and building choices cannot change it, so those controls are off. Rebuild with `npm run build:wasm` after any Rust change; an old package keeps running the old rules (the top bar badge says "Real simulation" or "Recorded demo"). `?sim=replay` forces the demo.

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

**AI agents and contributors: read [AGENTS.md](AGENTS.md) and [docs/README.md](docs/README.md).**

## Where things are

- `crates/`: the Rust simulation
- `content/`: all game numbers (JSON)
- `web/`: the browser app
- `docs/`: design docs and notes. Index: `docs/README.md`

## Deploy

Pushing to `main` builds the site and publishes it to the `gh-pages` branch (`.github/workflows/deploy.yml`).
In the repo, set Settings > Pages > Source to **Deploy from a branch**, branch `gh-pages`, folder `/ (root)`.
