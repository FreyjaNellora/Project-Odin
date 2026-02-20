// Huginn observability framework — Stage 0 core, grows per stage.
//
// When the `huginn` feature is OFF, `huginn_observe!` compiles to nothing.
// When ON, it writes raw data into a pre-allocated ring buffer.
//
// See AGENT_CONDUCT Section 3 for the full Huginn specification.

#[cfg(feature = "huginn")]
mod buffer;

#[cfg(feature = "huginn")]
pub use buffer::{HuginnBuffer, Level, Phase, TraceEvent, DEFAULT_BUFFER_CAPACITY};

/// Observation macro — compiles to nothing when `huginn` feature is disabled.
///
/// Arguments must be pure references or copies, never allocating expressions.
/// This ensures zero cost when the feature is off (macro expands to nothing,
/// so arguments are never evaluated).
#[cfg(not(feature = "huginn"))]
#[macro_export]
macro_rules! huginn_observe {
    ($($args:tt)*) => {};
}

/// Observation macro — writes raw data into the `HuginnBuffer` when enabled.
///
/// Usage: `huginn_observe!(buffer, "gate_name", stage, phase, level, data0, data1, ...)`
///
/// All data arguments are cast to `u64`. Arguments must be pure values
/// (references, copies, literals) — never function calls that allocate.
#[cfg(feature = "huginn")]
#[macro_export]
macro_rules! huginn_observe {
    ($buffer:expr, $gate:expr, $stage:expr, $phase:expr, $level:expr $(, $field:expr)*) => {
        $buffer.record($gate, $stage, $phase, $level, &[$($field as u64),*]);
    };
}
