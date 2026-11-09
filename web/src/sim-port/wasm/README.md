# wasm (stub)

Reserved for the real Rust simulation (built through `sim-api`). It must implement `SimPort`
from `../index.ts` and be selected in `createSimPort()`. Nothing else may know it exists.
