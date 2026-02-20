# Stage 00: Project Skeleton

**Tier:** 1 — Foundation
**Dependencies:** None
**Full spec:** [[MASTERPLAN]] Section 4, Stage 00

---

## Build Order Checklist

### 1. Create directory structure

```
Project_Odin/
├── odin-engine/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── main.rs
│       ├── board/        # Stage 1
│       ├── movegen/      # Stage 2
│       ├── gamestate/    # Stage 3
│       ├── protocol/     # Stage 4
│       ├── eval/         # Stage 6, 14-16
│       ├── search/       # Stage 7-11
│       ├── huginn/       # Stage 0 core, grows per stage
│       └── variants/     # Stage 17
│   └── tests/
├── odin-ui/
│   ├── package.json
│   └── src/
├── odin-nnue/            # Stage 14-15
└── tools/
```

- [ ] `git init` (if not already done)
- [ ] Create Cargo workspace at project root
- [ ] `cargo init --lib odin-engine`
- [ ] Create module directories (empty `mod.rs` files)
- [ ] Create `odin-ui/` with React scaffold
- [ ] Create `odin-nnue/` placeholder
- [ ] Create `tools/` placeholder
- [ ] `.gitignore`: target/, dist/, node_modules/, *.onnue, *.jsonl

### 2. Initialize Cargo workspace + React project

- [ ] `Cargo.toml` at workspace root with `members = ["odin-engine"]`
- [ ] `odin-engine/Cargo.toml` with `huginn` feature flag (default off)
- [ ] `cargo build` succeeds (empty project)
- [ ] `npm init` in `odin-ui/` (or `npx create-react-app` / Vite)
- [ ] `npm install && npm run dev` works

### 3. Write the Huginn macro and buffer

**Macro pattern (feature OFF — compiles to nothing):**
```rust
#[cfg(not(feature = "huginn"))]
macro_rules! huginn_observe {
    ($($args:tt)*) => {};
}
```

**When feature ON:** macro writes raw data into `HuginnBuffer` ring buffer.

- [ ] `huginn/mod.rs`: feature-gated module
- [ ] `huginn_observe!` macro (both branches: on and off)
- [ ] `HuginnBuffer` struct: pre-allocated ring buffer, 65,536 slots default
- [ ] `TraceEvent` struct: ts, session_id, trace_id, gate, stage, phase, level, data
- [ ] Buffer wraps silently when full — no panic, no allocation
- [ ] Macro arguments must be pure references, never allocating expressions

### 4. Write the proof-of-life test

- [ ] `cargo build` succeeds
- [ ] `cargo build --features huginn` succeeds
- [ ] `cargo test` passes
- [ ] `cargo test --features huginn` passes
- [ ] Binary without huginn contains zero Huginn symbols (verified by inspection)
- [ ] `huginn_observe!` call in a test function compiles to nothing when flag is off

### 5. Set up linting, formatting, CI

- [ ] `cargo fmt` runs clean
- [ ] `cargo clippy` runs with zero warnings
- [ ] CI config: build, test, build with huginn, test with huginn, fmt check, clippy
- [ ] All checks green

---

## Acceptance Criteria (from [[MASTERPLAN]])

1. `cargo build` succeeds, `cargo build --features huginn` succeeds
2. `huginn_observe!` compiles to nothing without the feature (verified by binary inspection)
3. UI initializes with `npm install && npm run dev`

## What You DON'T Need

- Any engine code. This is pure scaffolding.
- Any observation points. Those come from each stage as it builds the thing being observed.

## Huginn Gates

- `search_summary` (Stage 0+, minimal level): per-search headline. Infrastructure only — no actual searches exist yet.

## Invariants Established

- Prior-stage tests never deleted (this is Stage 0, so: all future tests are additive)
- Huginn compiles to nothing when feature flag is off

---

## Related

- Audit log: [[audit_log_stage_00]]
- Downstream log: [[downstream_log_stage_00]]
- Full spec: [[MASTERPLAN]] Section 4, Stage 00
