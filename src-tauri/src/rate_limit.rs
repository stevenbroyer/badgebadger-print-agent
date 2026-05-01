// Token-bucket rate limiter for /print. Defends against a runaway
// or hostile tab spamming the agent — the body limit caps memory but
// nothing else stops the tab from queuing 10,000 separate jobs that
// would each burn ribbon. We cap at burst=20 + 60/min steady-state,
// which is well above any realistic bulk batch (the web app fans out
// concurrency 4 — see employees-list.tsx).
//
// Single global bucket: traffic is always from 127.0.0.1 anyway, so
// per-IP buckets would all collapse onto one entry. Simpler this way.

use std::sync::Mutex;
use std::time::Instant;

const BURST: f64 = 20.0;
const REFILL_PER_SEC: f64 = 1.0;

pub struct RateLimiter {
    inner: Mutex<Bucket>,
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Bucket {
                tokens: BURST,
                last: Instant::now(),
            }),
        }
    }

    /// Returns true when the request is allowed (and consumes one
    /// token), false when the bucket is empty.
    pub fn check(&self) -> bool {
        let mut b = self.inner.lock().expect("rate-limit mutex poisoned");
        let now = Instant::now();
        let elapsed = now.duration_since(b.last).as_secs_f64();
        b.tokens = (b.tokens + elapsed * REFILL_PER_SEC).min(BURST);
        b.last = now;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
