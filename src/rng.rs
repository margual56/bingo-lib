use std::sync::atomic::{AtomicU64, Ordering};

use rand::rngs::SmallRng;
use rand::SeedableRng;

static FALLBACK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Builds a `SmallRng`, either from an explicit seed or from a fallback seed
/// derived from the current time plus a call counter. Deliberately avoids
/// `getrandom`/OS entropy so this crate needs no special wasm RNG wiring and
/// its logic can be unit tested with plain `cargo test` on the host target.
pub fn make_rng(seed: Option<u64>) -> SmallRng {
    let seed = seed.unwrap_or_else(fallback_seed);
    SmallRng::seed_from_u64(seed)
}

#[cfg(target_arch = "wasm32")]
fn fallback_seed() -> u64 {
    let now = js_sys::Date::now() as u64;
    let counter = FALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);
    now ^ counter.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(not(target_arch = "wasm32"))]
fn fallback_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = FALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);
    now ^ counter.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}
