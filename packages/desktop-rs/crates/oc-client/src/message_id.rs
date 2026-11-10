//! Client-minted message ids, the way the React UI mints them.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE62: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const RANDOM_LEN: usize = 14;
const COUNTER_BITS: u64 = 4096;

static LAST_STAMP: AtomicU64 = AtomicU64::new(0);
static ENTROPY: AtomicU64 = AtomicU64::new(0);

/// `msg_` + 12 hex digits of `millis * 4096 + counter` + 14 random base-62 characters,
/// as `ascendingId("msg")` in `packages/ui/src/lib/opencode/ids.ts` builds it. The
/// stamp never goes backwards, so ids minted in the same millisecond still ascend.
pub fn new_message_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64);
    let floor = millis.saturating_mul(COUNTER_BITS);
    let previous = LAST_STAMP
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
            Some(floor.max(last + 1))
        })
        .unwrap_or_default();
    let stamp = floor.max(previous + 1) & 0xffff_ffff_ffff;
    format!("msg_{stamp:012x}{}", random_base62())
}

fn random_base62() -> String {
    // `RandomState` is seeded from the OS per instance, which is all an id suffix needs.
    let mut out = String::with_capacity(RANDOM_LEN);
    while out.len() < RANDOM_LEN {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(ENTROPY.fetch_add(1, Ordering::Relaxed));
        let mut bits = hasher.finish();
        for _ in 0..8 {
            if out.len() == RANDOM_LEN {
                break;
            }
            out.push(char::from(BASE62[(bits % 62) as usize]));
            bits /= 62;
        }
    }
    out
}

#[cfg(test)]
#[path = "message_id_test.rs"]
mod message_id_test;
