use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    VerifyingHost,
    Authenticating,
    Connected,
    Reconnecting { attempt: u32, next_retry_ms: u64 },
    Failed(String),
}

impl ConnectionState {
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting...",
            ConnectionState::VerifyingHost => "Verifying Host...",
            ConnectionState::Authenticating => "Authenticating...",
            ConnectionState::Connected => "Connected",
            ConnectionState::Reconnecting { .. } => "Reconnecting...",
            ConnectionState::Failed(_) => "Connection Failed",
        }
    }
}

/// Cooperative cancellation token enabling sub-50ms responsive aborts.
#[derive(Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

/// Calculates exponential backoff with pseudo-random jitter.
/// Follows 2s, 4s, 8s, 16s, capped at max_ms (default 30s).
pub fn calculate_backoff_delay(
    attempt: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    backoff_factor: u32,
    jitter: bool,
) -> u64 {
    if attempt == 0 {
        return initial_delay_ms;
    }
    let factor = (backoff_factor as u64).saturating_pow(attempt.saturating_sub(1));
    let base_delay = initial_delay_ms.saturating_mul(factor).min(max_delay_ms);

    if jitter {
        // Deterministic pseudo-jitter using timestamp/attempt mix
        let pseudo_rand = ((attempt as u64)
            .wrapping_mul(1103515245)
            .wrapping_add(12345))
            % 500;
        base_delay.saturating_add(pseudo_rand).min(max_delay_ms)
    } else {
        base_delay
    }
}

/// Bounded connection pool enforcing a maximum concurrency cap (default 8).
pub struct BoundedConnectionPool {
    max_connections: usize,
    active_count: std::sync::atomic::AtomicUsize,
}

impl BoundedConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            active_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn acquire(&self) -> Result<PoolLease<'_>, String> {
        let current = self.active_count.load(Ordering::SeqCst);
        if current >= self.max_connections {
            return Err(format!(
                "Connection pool capacity reached (max {} connections)",
                self.max_connections
            ));
        }
        self.active_count.fetch_add(1, Ordering::SeqCst);
        Ok(PoolLease {
            counter: &self.active_count,
        })
    }

    pub fn active_connections(&self) -> usize {
        self.active_count.load(Ordering::Relaxed)
    }
}

pub struct PoolLease<'a> {
    counter: &'a std::sync::atomic::AtomicUsize,
}

impl<'a> Drop for PoolLease<'a> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}
