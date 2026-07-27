//! Central nonce-range allocator for CUDA GPU mining.
//!
//! Guarantees:
//! - ranges leased for a given template do not overlap;
//! - template change invalidates further leases (callers must drop old work);
//! - exhaustion is reported without silent wrap of the search window.

use std::sync::atomic::{AtomicU64, Ordering};

/// Identity of a mining work package (tip + PoW preimage fields).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkIdentity {
    pub template_id: String,
    pub fingerprint: String,
}

/// A contiguous half-open nonce range `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonceRange {
    pub start: u64,
    pub end: u64,
}

impl NonceRange {
    pub fn len(self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

/// Atomically leases non-overlapping nonce ranges for one work package.
pub struct NonceAllocator {
    generation: AtomicU64,
    cursor: AtomicU64,
    /// Exclusive end of the current lease window (optional cap).
    /// `u64::MAX` means "no hard end"; wrapping is never used.
    window_end: AtomicU64,
    work_generation: AtomicU64,
}

impl Default for NonceAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl NonceAllocator {
    pub fn new() -> Self {
        Self {
            generation: AtomicU64::new(1),
            cursor: AtomicU64::new(0),
            window_end: AtomicU64::new(u64::MAX),
            work_generation: AtomicU64::new(0),
        }
    }

    /// Bind the allocator to a new template / work fingerprint.
    ///
    /// Resets the cursor to `nonce_start`. Previous leases become stale once
    /// callers observe a generation change (via `generation()`).
    pub fn reset_work(&self, nonce_start: u64, window_size: Option<u64>) {
        let end = match window_size {
            Some(size) if size > 0 => nonce_start.saturating_add(size),
            _ => u64::MAX,
        };
        self.cursor.store(nonce_start, Ordering::SeqCst);
        self.window_end.store(end, Ordering::SeqCst);
        self.work_generation.fetch_add(1, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn cursor(&self) -> u64 {
        self.cursor.load(Ordering::SeqCst)
    }

    /// Lease up to `count` nonces. Returns `None` when the window is exhausted.
    pub fn lease(&self, count: u64) -> Option<NonceRange> {
        if count == 0 {
            return None;
        }
        let end_cap = self.window_end.load(Ordering::SeqCst);
        loop {
            let start = self.cursor.load(Ordering::SeqCst);
            if start >= end_cap {
                return None;
            }
            let available = end_cap.saturating_sub(start);
            let take = count.min(available);
            if take == 0 {
                return None;
            }
            let new_cursor = start.saturating_add(take);
            match self.cursor.compare_exchange_weak(
                start,
                new_cursor,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    return Some(NonceRange {
                        start,
                        end: new_cursor,
                    });
                }
                Err(_) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leases_do_not_overlap() {
        let alloc = NonceAllocator::new();
        alloc.reset_work(100, Some(50));
        let a = alloc.lease(10).expect("a");
        let b = alloc.lease(10).expect("b");
        assert_eq!(
            a,
            NonceRange {
                start: 100,
                end: 110
            }
        );
        assert_eq!(
            b,
            NonceRange {
                start: 110,
                end: 120
            }
        );
        assert!(a.end <= b.start);
    }

    #[test]
    fn exhaustion_without_wrap() {
        let alloc = NonceAllocator::new();
        alloc.reset_work(0, Some(5));
        assert!(alloc.lease(5).is_some());
        assert!(alloc.lease(1).is_none());
    }
}
