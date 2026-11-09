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

Without it the app falls back to a mock economy and shows a "Mock data" badge.

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

## Deploy

Pushing to the `deploy` branch builds and publishes the site to GitHub Pages (`.github/workflows/deploy.yml`).
In the repo, set Settings > Pages > Source to **GitHub Actions** once.
