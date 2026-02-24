# Downstream Log — Stage 00: Skeleton

## Notes for Future Stages

**Note:** Huginn was retired in Stage 8 and replaced with the `tracing` crate (see ADR-015). The API documented below no longer exists.

### Must-Know

1. **`huginn_observe!` macro is available crate-wide via `#[macro_export]`.** In integration tests, import with `use odin_engine::huginn_observe;`. In lib.rs modules, it is automatically available.

2. **The macro has two forms depending on feature flag:**
   - Feature OFF: `huginn_observe!(any, tokens, here)` → compiles to nothing. Arguments are not evaluated.
   - Feature ON: `huginn_observe!(buffer, "gate_name", stage_u8, Phase::Variant, Level::Variant, data0, data1, ...)` → records to buffer. Data arguments are cast to `u64`.

3. **Macro arguments must be pure.** Never pass allocating expressions (e.g., `board.clone()`, `format!(...)`). Even though the OFF macro discards arguments, the ON macro evaluates them. See AGENT_CONDUCT Section 3.8 anti-pattern #6.

### API Contracts

**`HuginnBuffer`** (feature-gated behind `huginn`):
- `HuginnBuffer::new(capacity: usize) -> Self` — pre-allocates `capacity` slots. Call once at engine startup.
- `HuginnBuffer::with_default_capacity() -> Self` — creates buffer with 65,536 slots.
- `buf.new_trace() -> u64` — starts a new trace, returns trace ID. Call once per search invocation (go → bestmove).
- `buf.record(gate, stage, phase, level, data)` — records one observation. No allocation. Wraps silently when full.
- `buf.len() -> usize` — number of stored events.
- `buf.is_empty() -> bool` — whether buffer has no events.
- `buf.get(index: usize) -> Option<&TraceEvent>` — read event by logical index (0 = oldest).
- `buf.session_id() -> u64` — session ID (unique per engine process).
- `buf.current_trace_id() -> u64` — current trace ID.

**`TraceEvent`** fields: `ts: u64`, `session_id: u64`, `trace_id: u64`, `gate: &'static str`, `stage: u8`, `phase: Phase`, `level: Level`. Data accessed via `event.data() -> &[u64]`.

**`Phase`** enum: `Setup`, `MoveGen`, `Eval`, `Brs`, `Mcts`, `Summary`. Repr `u8`.

**`Level`** enum: `Minimal`, `Normal`, `Verbose`, `Everything`. Repr `u8`. Implements `Ord` for filtering.

### Known Limitations

1. **No JSON serialization yet.** The buffer stores raw `u64` values. Post-search JSON serialization (AGENT_CONDUCT Section 3.1) is not implemented. Future stages that add observation points work with raw data; the serialization pipeline can be added when the file sink is implemented.

2. **No file sink.** The optional JSONL file output (AGENT_CONDUCT Section 3.2) is not implemented. Buffer is in-memory only.

3. **No global buffer instance.** Each buffer must be created and passed explicitly. No global, thread-local, or static buffer is set up. The engine's `main.rs` will need to create and thread the buffer through when Huginn is used.

4. **Data limited to 16 u64 fields per observation.** The `MAX_DATA_FIELDS` constant is 16. Observations with more fields will be silently truncated.

### Performance Baselines

| Metric | Value | Notes |
|---|---|---|
| `cargo build` | 0.70s | Dev profile, empty project |
| `cargo build --features huginn` | 0.97s | Dev profile |
| `cargo build --release` | 1.30s | Release binary: 129,024 bytes |
| `cargo test` | 0.48s | 2 tests |

### Open Questions

None for Stage 0.

### Reasoning

- **Raw u64 data storage chosen over structured types** because it requires zero allocation, supports any future observation payload without schema changes, and defers JSON formatting to post-search processing per AGENT_CONDUCT Section 3.8.
- **AtomicU64 counter for IDs** instead of random UUIDs because it is lock-free, deterministic within a process, and sufficient for within-process trace correlation. Cross-process correlation (if ever needed) would use session timestamps.
- **Vec for buffer backing store** (allocated once) instead of a fixed-size array because Rust stack-allocated arrays of 65,536 elements would overflow the stack, and `Vec::with_capacity` followed by pushes gives exactly one heap allocation.

---

## Related

- Stage spec: [[stage_00_skeleton]]
- Audit log: [[audit_log_stage_00]]
