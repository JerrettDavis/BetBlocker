use std::sync::{Arc, Mutex};

use tracing::{debug, error};

use super::store::EventStore;
use super::AgentEvent;

/// Event emitter with an in-memory buffer that flushes to SQLite.
///
/// Thread-safe: the buffer is wrapped in `Arc<Mutex<>>` so multiple
/// threads can emit events concurrently.
pub struct EventEmitter {
    buffer: Arc<Mutex<Vec<AgentEvent>>>,
    store: EventStore,
    /// Auto-flush when the buffer exceeds this size.
    flush_threshold: usize,
}

impl EventEmitter {
    /// Create a new emitter backed by the given store.
    pub fn new(store: EventStore) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            store,
            flush_threshold: 50,
        }
    }

    /// Set the auto-flush threshold.
    #[must_use]
    pub fn with_flush_threshold(mut self, threshold: usize) -> Self {
        self.flush_threshold = threshold;
        self
    }

    /// Emit an event. Pushes to the in-memory buffer.
    /// If the buffer exceeds the flush threshold, auto-flushes.
    pub fn emit(&self, event: AgentEvent) {
        let should_flush = {
            let mut buf = self
                .buffer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            buf.push(event);
            buf.len() >= self.flush_threshold
        };

        if should_flush
            && let Err(e) = self.flush()
        {
            error!(error = %e, "Auto-flush failed");
        }
    }

    /// Flush all buffered events to SQLite. Clears the buffer on success.
    pub fn flush(&self) -> Result<usize, rusqlite::Error> {
        let events: Vec<AgentEvent> = {
            let mut buf = self
                .buffer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut *buf)
        };

        if events.is_empty() {
            return Ok(0);
        }

        let count = events.len();
        for event in &events {
            self.store.insert(event)?;
        }

        debug!(count, "Flushed events to SQLite");
        Ok(count)
    }

    /// Get a reference to the underlying store (for querying unreported events, etc.).
    pub fn store(&self) -> &EventStore {
        &self.store
    }

    /// Number of events currently buffered (not yet flushed).
    pub fn buffered_count(&self) -> usize {
        self.buffer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Create a handle that can be used to emit events from other threads.
    pub fn handle(&self) -> EventEmitterHandle {
        EventEmitterHandle {
            buffer: Arc::clone(&self.buffer),
            flush_threshold: self.flush_threshold,
        }
    }
}

/// A lightweight, cloneable handle for emitting events from any thread.
/// Events are buffered in memory; the main `EventEmitter` must be flushed
/// to persist them.
#[derive(Clone)]
pub struct EventEmitterHandle {
    buffer: Arc<Mutex<Vec<AgentEvent>>>,
    flush_threshold: usize,
}

impl EventEmitterHandle {
    /// Emit an event into the shared buffer.
    pub fn emit(&self, event: AgentEvent) {
        let mut buf = self
            .buffer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        buf.push(event);
        // Note: handles cannot flush on their own (no store reference).
        // The main emitter is responsible for flushing.
        if buf.len() >= self.flush_threshold {
            debug!(
                buffered = buf.len(),
                "Buffer exceeds threshold, flush recommended"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_and_flush() {
        let store = EventStore::in_memory().expect("store");
        let emitter = EventEmitter::new(store);

        emitter.emit(AgentEvent::dns_block("a.com", "dns.resolver"));
        emitter.emit(AgentEvent::dns_block("b.com", "dns.resolver"));

        assert_eq!(emitter.buffered_count(), 2);

        let flushed = emitter.flush().expect("flush");
        assert_eq!(flushed, 2);
        assert_eq!(emitter.buffered_count(), 0);

        let unreported = emitter.store().unreported(100).expect("unreported");
        assert_eq!(unreported.len(), 2);
    }

    #[test]
    fn test_auto_flush() {
        let store = EventStore::in_memory().expect("store");
        let emitter = EventEmitter::new(store).with_flush_threshold(3);

        emitter.emit(AgentEvent::heartbeat());
        emitter.emit(AgentEvent::heartbeat());
        assert_eq!(emitter.buffered_count(), 2);

        // Third event triggers auto-flush
        emitter.emit(AgentEvent::heartbeat());
        assert_eq!(emitter.buffered_count(), 0);

        assert_eq!(emitter.store().count().expect("count"), 3);
    }

    #[test]
    fn test_flush_empty_buffer() {
        let store = EventStore::in_memory().expect("store");
        let emitter = EventEmitter::new(store);
        let flushed = emitter.flush().expect("flush");
        assert_eq!(flushed, 0);
    }

    #[test]
    fn test_mark_reported_after_flush() {
        let store = EventStore::in_memory().expect("store");
        let emitter = EventEmitter::new(store);

        emitter.emit(AgentEvent::dns_block("test.com", "dns.resolver"));
        emitter.flush().expect("flush");

        let unreported = emitter.store().unreported(100).expect("unreported");
        assert_eq!(unreported.len(), 1);

        let ids: Vec<i64> = unreported.iter().filter_map(|e| e.id).collect();
        emitter.store().mark_reported(&ids).expect("mark");

        let unreported = emitter.store().unreported(100).expect("unreported");
        assert!(unreported.is_empty());
    }
}
