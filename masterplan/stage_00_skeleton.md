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
- [ ] `odin-engine/Cargo.toml`
- [ ] `cargo build` succeeds (empty project)
- [ ] `npm init` in `odin-ui/` (or `npx create-react-app` / Vite)
- [ ] `npm install && npm run dev` works

### 3. ~~Write the Huginn macro and buffer~~ *(Retired — replaced by `tracing` crate in Stage 8; see ADR-015)*

### 4. Write the proof-of-life test

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes

### 5. Set up linting, formatting, CI

- [ ] `cargo fmt` runs clean
- [ ] `cargo clippy` runs with zero warnings
- [ ] CI config: build, test, fmt check, clippy
- [ ] All checks green

---

## Acceptance Criteria (from [[MASTERPLAN]])

1. `cargo build` succeeds
2. UI initializes with `npm install && npm run dev`

*(Original criteria included Huginn feature-flag verification -- retired in Stage 8, replaced with `tracing` crate; see ADR-015.)*

## What You DON'T Need

- Any engine code. This is pure scaffolding.
- Any observation points. Those come from each stage as it builds the thing being observed.

## ~~Huginn Gates~~ *(Retired — replaced by `tracing` crate in Stage 8; see ADR-015)*

## Invariants Established

- Prior-stage tests never deleted (this is Stage 0, so: all future tests are additive)

---

## Related

- Audit log: [[audit_log_stage_00]]
- Downstream log: [[downstream_log_stage_00]]
- Full spec: [[MASTERPLAN]] Section 4, Stage 00
