// Stage 00 — Proof-of-life integration tests.
//
// Verifies that the project skeleton compiles and the Huginn macro
// behaves correctly in both feature-on and feature-off configurations.

use odin_engine::huginn_observe;

/// The huginn_observe! macro must compile without errors when huginn
/// feature is OFF. It expands to nothing — arbitrary tokens are consumed.
#[cfg(not(feature = "huginn"))]
#[test]
fn test_huginn_observe_compiles_to_nothing_when_off() {
    // These calls must all compile to nothing. The macro eats any tokens.
    huginn_observe!("anything", "can", "go", "here", 42, true);
    huginn_observe!();
    huginn_observe!(some, arbitrary, tokens, 1 + 2);

    // Verify no side effects: if the macro produced any code,
    // this test would fail or behave unexpectedly.
}

/// When huginn is enabled, the macro writes to a buffer and the buffer
/// is functional.
#[cfg(feature = "huginn")]
#[test]
fn test_huginn_observe_records_with_feature() {
    use odin_engine::huginn::{HuginnBuffer, Level, Phase};

    let mut buf = HuginnBuffer::new(16);
    buf.new_trace();

    huginn_observe!(buf, "test_gate", 0u8, Phase::Setup, Level::Minimal, 42u64);

    assert_eq!(buf.len(), 1);
    let event = buf.get(0).expect("should have one event");
    assert_eq!(event.gate, "test_gate");
    assert_eq!(event.data(), &[42]);
}

/// When huginn is enabled, the buffer wraps without panicking.
#[cfg(feature = "huginn")]
#[test]
fn test_huginn_buffer_wraps_without_panic() {
    use odin_engine::huginn::{HuginnBuffer, Level, Phase};

    let mut buf = HuginnBuffer::new(4);
    buf.new_trace();

    // Write more events than capacity — must not panic
    for i in 0u64..100 {
        huginn_observe!(buf, "wrap_test", 0u8, Phase::Setup, Level::Minimal, i);
    }

    // Buffer should contain only the last 4 events
    assert_eq!(buf.len(), 4);
    let newest = buf.get(3).expect("newest event");
    assert_eq!(newest.data(), &[99]);
}

/// Verify the engine crate can be linked. If this test compiles and runs,
/// the library was built successfully.
#[test]
fn test_engine_crate_links() {
    // The mere existence and execution of this test proves the crate compiled.
}
