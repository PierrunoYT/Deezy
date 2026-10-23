use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

/// Upper bound on one sleep, so a cancellation is noticed promptly even when
/// the shared budget is booked far ahead by concurrent downloads.
const MAX_SLEEP_STEP: Duration = Duration::from_millis(250);

/// Paces downloads to a shared byte rate. Every concurrent download draws from
/// the same budget, so the configured limit caps the app's total bandwidth.
pub struct BandwidthLimiter {
    next_slot: Mutex<Instant>,
}

impl BandwidthLimiter {
    pub fn new() -> Self {
        Self {
            next_slot: Mutex::new(Instant::now()),
        }
    }

    /// Books `bytes` against the budget and waits until they are due.
    /// A `limit_kbps` of 0 means unlimited. Returns early if `cancel_flag` is set.
    pub async fn acquire(&self, bytes: usize, limit_kbps: u32, cancel_flag: &AtomicBool) {
        if limit_kbps == 0 || bytes == 0 {
            return;
        }

        let bytes_per_second = f64::from(limit_kbps) * 1000.0 / 8.0;
        let cost = Duration::from_secs_f64(bytes as f64 / bytes_per_second);
        let due = {
            let mut next_slot = self.next_slot.lock().await;
            // Idle time is not banked, so a pause never allows a later burst.
            let start = (*next_slot).max(Instant::now());
            *next_slot = start + cost;
            *next_slot
        };

        while !cancel_flag.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now >= due {
                return;
            }
            tokio::time::sleep((due - now).min(MAX_SLEEP_STEP)).await;
        }

        // Cancelled: give back the part of this booking that has not elapsed
        // yet, so other downloads are not slowed by bytes nobody will write.
        let now = Instant::now();
        if due > now {
            let refund = (due - now).min(cost);
            let mut next_slot = self.next_slot.lock().await;
            if let Some(earlier) = next_slot.checked_sub(refund) {
                *next_slot = earlier.max(now);
            }
        }
    }
}

impl Default for BandwidthLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::BandwidthLimiter;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn unlimited_does_not_wait() {
        let limiter = BandwidthLimiter::new();
        let started = Instant::now();
        limiter.acquire(10_000_000, 0, &AtomicBool::new(false)).await;
        assert!(started.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn bytes_are_paced_to_the_limit() {
        // 8 kbps = 1000 bytes per second, so 200 bytes take 200 ms.
        let limiter = BandwidthLimiter::new();
        let cancel_flag = AtomicBool::new(false);
        let started = Instant::now();
        limiter.acquire(100, 8, &cancel_flag).await;
        limiter.acquire(100, 8, &cancel_flag).await;
        assert!(started.elapsed() >= Duration::from_millis(190));
    }

    #[tokio::test]
    async fn cancellation_stops_waiting() {
        let limiter = BandwidthLimiter::new();
        let started = Instant::now();
        limiter.acquire(1_000_000, 8, &AtomicBool::new(true)).await;
        assert!(started.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn a_cancelled_booking_is_given_back() {
        // 1,000,000 bytes at 1000 bytes per second would book 1000 s.
        let limiter = BandwidthLimiter::new();
        limiter.acquire(1_000_000, 8, &AtomicBool::new(true)).await;
        let started = Instant::now();
        limiter.acquire(100, 8, &AtomicBool::new(false)).await;
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
