# wasm: the real Rust simulation

`crates/sim-wasm` (a thin wasm-bindgen shell over `crates/sim-api`) is built into `pkg/` here by

    npm run build:wasm        # in web/

Needs `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock> --locked` (the script checks and prints the exact command). `wasm-opt` (binaryen) is used if installed. `pkg/` is gitignored; never commit binaries.

`index.ts` implements `SimPort` over the package. Only `sim-port` knows about it. `sim-port/index.ts` `selectSimPort()` picks wasm or mock: `?sim=wasm|mock`, then `VITE_SIM`, default wasm when `pkg/` exists, else mock with a visible notice. The top bar badge reads "Real simulation" or "Mock data" from `snapshot.mock`.
