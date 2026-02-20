// Huginn ring buffer — pre-allocated, zero-allocation during search.
//
// This module is only compiled when `cfg(feature = "huginn")` is active.
// See AGENT_CONDUCT Section 3.2 for the storage specification.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// Maximum number of raw u64 data fields per observation.
const MAX_DATA_FIELDS: usize = 16;

/// Default ring buffer capacity (2^16 = 65,536 entries).
pub const DEFAULT_BUFFER_CAPACITY: usize = 65_536;

/// Search/engine phase that emitted the observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    Setup = 0,
    MoveGen = 1,
    Eval = 2,
    Brs = 3,
    Mcts = 4,
    Summary = 5,
}

/// Verbosity level for the observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Minimal = 0,
    Normal = 1,
    Verbose = 2,
    Everything = 3,
}

/// A single Huginn observation event.
///
/// Stores raw data (integers, enum values as u8, hashes as u64).
/// JSON serialization happens during post-search processing, not here.
pub struct TraceEvent {
    /// Monotonic clock nanoseconds from engine start.
    pub ts: u64,
    /// Generated once per engine process.
    pub session_id: u64,
    /// Generated per search invocation (go -> bestmove).
    pub trace_id: u64,
    /// Observation point name (matches MASTERPLAN gate names).
    pub gate: &'static str,
    /// Stage number that defined this gate.
    pub stage: u8,
    /// Which engine phase emitted this.
    pub phase: Phase,
    /// Verbosity level.
    pub level: Level,
    /// Raw data payload as u64 values.
    data: [u64; MAX_DATA_FIELDS],
    /// How many fields in `data` are used.
    data_len: u8,
}

impl TraceEvent {
    /// Create an empty event for buffer pre-allocation.
    const fn empty() -> Self {
        Self {
            ts: 0,
            session_id: 0,
            trace_id: 0,
            gate: "",
            stage: 0,
            phase: Phase::Setup,
            level: Level::Minimal,
            data: [0u64; MAX_DATA_FIELDS],
            data_len: 0,
        }
    }

    /// Read the data payload as a slice.
    pub fn data(&self) -> &[u64] {
        &self.data[..self.data_len as usize]
    }
}

/// Pre-allocated ring buffer for Huginn observations.
///
/// Fixed capacity, wraps silently when full. No allocation after construction.
/// See AGENT_CONDUCT Section 3.2 for specification.
pub struct HuginnBuffer {
    events: Vec<TraceEvent>,
    head: usize,
    count: usize,
    capacity: usize,
    session_id: u64,
    current_trace_id: u64,
}

impl HuginnBuffer {
    /// Create a new buffer with the given capacity. Pre-allocates all slots.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        let mut events = Vec::with_capacity(cap);
        for _ in 0..cap {
            events.push(TraceEvent::empty());
        }
        Self {
            events,
            head: 0,
            count: 0,
            capacity: cap,
            session_id: Self::generate_id(),
            current_trace_id: 0,
        }
    }

    /// Create a buffer with default capacity (65,536 entries).
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_BUFFER_CAPACITY)
    }

    /// Start a new trace (one per search invocation, go -> bestmove).
    pub fn new_trace(&mut self) -> u64 {
        self.current_trace_id = Self::generate_id();
        self.current_trace_id
    }

    /// Record an observation into the next slot. Wraps silently when full.
    ///
    /// Called from the `huginn_observe!` macro. Must never panic.
    pub fn record(
        &mut self,
        gate: &'static str,
        stage: u8,
        phase: Phase,
        level: Level,
        data: &[u64],
    ) {
        let slot = &mut self.events[self.head];
        slot.ts = Self::monotonic_ns();
        slot.session_id = self.session_id;
        slot.trace_id = self.current_trace_id;
        slot.gate = gate;
        slot.stage = stage;
        slot.phase = phase;
        slot.level = level;

        // Copy data, truncating if it exceeds capacity
        let copy_len = data.len().min(MAX_DATA_FIELDS);
        slot.data[..copy_len].copy_from_slice(&data[..copy_len]);
        slot.data_len = copy_len as u8;

        // Advance head, wrap around silently
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Number of events currently stored.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get the current session ID.
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Get the current trace ID.
    pub fn current_trace_id(&self) -> u64 {
        self.current_trace_id
    }

    /// Read event at the given logical index (0 = oldest stored event).
    /// Returns `None` if index is out of range.
    pub fn get(&self, index: usize) -> Option<&TraceEvent> {
        if index >= self.count {
            return None;
        }
        // Calculate physical index: oldest event is at (head - count) wrapped
        let start = (self.head + self.capacity - self.count) % self.capacity;
        let physical = (start + index) % self.capacity;
        Some(&self.events[physical])
    }

    /// Generate a unique ID within this process. Monotonically increasing.
    fn generate_id() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Monotonic nanoseconds since first call. No allocation after first call.
    fn monotonic_ns() -> u64 {
        static START: OnceLock<Instant> = OnceLock::new();
        let start = START.get_or_init(Instant::now);
        start.elapsed().as_nanos() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation_and_empty_state() {
        let buf = HuginnBuffer::new(8);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert!(buf.get(0).is_none());
    }

    #[test]
    fn test_buffer_record_and_read() {
        let mut buf = HuginnBuffer::new(8);
        buf.new_trace();
        buf.record("test_gate", 0, Phase::Setup, Level::Minimal, &[42, 100]);

        assert_eq!(buf.len(), 1);
        assert!(!buf.is_empty());

        let event = buf.get(0).expect("event should exist");
        assert_eq!(event.gate, "test_gate");
        assert_eq!(event.stage, 0);
        assert_eq!(event.phase, Phase::Setup);
        assert_eq!(event.level, Level::Minimal);
        assert_eq!(event.data(), &[42, 100]);
        // ts is monotonic nanoseconds — any value is valid (including 0 on first call)
        let _ = event.ts;
    }

    #[test]
    fn test_buffer_wraps_silently_when_full() {
        let mut buf = HuginnBuffer::new(4);
        buf.new_trace();

        // Fill beyond capacity
        for i in 0u64..6 {
            buf.record("gate", 0, Phase::Setup, Level::Minimal, &[i]);
        }

        // Count should be capped at capacity
        assert_eq!(buf.len(), 4);

        // Oldest two (0, 1) should be overwritten; we should see 2, 3, 4, 5
        let oldest = buf.get(0).expect("oldest event");
        assert_eq!(oldest.data(), &[2]);

        let newest = buf.get(3).expect("newest event");
        assert_eq!(newest.data(), &[5]);
    }

    #[test]
    fn test_buffer_data_truncation() {
        let mut buf = HuginnBuffer::new(4);
        buf.new_trace();

        // Try to record more than MAX_DATA_FIELDS values
        let big_data: Vec<u64> = (0..32).collect();
        buf.record("gate", 0, Phase::Setup, Level::Minimal, &big_data);

        let event = buf.get(0).expect("event should exist");
        assert_eq!(event.data().len(), MAX_DATA_FIELDS);
        // First MAX_DATA_FIELDS values should be preserved
        assert_eq!(event.data()[0], 0);
        assert_eq!(
            event.data()[MAX_DATA_FIELDS - 1],
            (MAX_DATA_FIELDS - 1) as u64
        );
    }

    #[test]
    fn test_trace_ids_are_unique() {
        let mut buf = HuginnBuffer::new(4);
        let id1 = buf.new_trace();
        let id2 = buf.new_trace();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_set_on_construction() {
        let buf1 = HuginnBuffer::new(4);
        let buf2 = HuginnBuffer::new(4);
        // Each buffer should get a unique session ID
        assert_ne!(buf1.session_id(), buf2.session_id());
    }

    #[test]
    fn test_events_carry_session_and_trace_ids() {
        let mut buf = HuginnBuffer::new(4);
        let session = buf.session_id();
        let trace = buf.new_trace();
        buf.record("gate", 0, Phase::Setup, Level::Minimal, &[]);

        let event = buf.get(0).expect("event should exist");
        assert_eq!(event.session_id, session);
        assert_eq!(event.trace_id, trace);
    }

    #[test]
    fn test_phase_and_level_ordering() {
        // Level should be orderable for filtering
        assert!(Level::Minimal < Level::Normal);
        assert!(Level::Normal < Level::Verbose);
        assert!(Level::Verbose < Level::Everything);
    }
}
