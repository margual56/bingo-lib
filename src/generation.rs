use std::collections::HashSet;

use crate::card::{generate_card, Card, CARD_CELLS};
use crate::rng::make_rng;

/// Generates `n` structurally valid cards such that no row is repeated
/// across the whole batch (which by construction also means no two full
/// cards can be identical). Collisions are checked against a global set of
/// row keys and colliding cards are regenerated from scratch; this is cheap
/// because the total number of distinct possible rows (12,565,000) is
/// enormous relative to any realistic `n`.
pub fn generate_cards(n: u32, seed: Option<u64>) -> Vec<Card> {
    let mut rng = make_rng(seed);
    let mut used_rows: HashSet<u128> = HashSet::with_capacity(n as usize * 3);
    let mut cards = Vec::with_capacity(n as usize);

    for _ in 0..n {
        loop {
            let card = generate_card(&mut rng);
            let keys = card.row_keys();
            if keys.iter().all(|k| !used_rows.contains(k)) {
                for k in keys {
                    used_rows.insert(k);
                }
                cards.push(card);
                break;
            }
        }
    }

    cards
}

/// Convenience for the wasm boundary: flattens a batch of cards into a
/// single row-major `Vec<u16>` of length `n * CARD_CELLS` (0 = blank cell).
pub fn flatten_cards(cards: &[Card]) -> Vec<u16> {
    let mut out = Vec::with_capacity(cards.len() * CARD_CELLS);
    for card in cards {
        out.extend(card.flatten());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicate_rows_in_a_large_batch() {
        let n = 2000;
        let cards = generate_cards(n, Some(2024));
        let mut all_keys = HashSet::new();
        for card in &cards {
            for key in card.row_keys() {
                assert!(all_keys.insert(key), "duplicate row detected");
            }
        }
        assert_eq!(all_keys.len(), (n as usize) * 3);
    }

    #[test]
    fn generation_is_deterministic_given_a_seed() {
        let a = generate_cards(50, Some(555));
        let b = generate_cards(50, Some(555));
        for (ca, cb) in a.iter().zip(b.iter()) {
            assert_eq!(ca.grid, cb.grid);
        }
    }

    #[test]
    fn different_seeds_produce_different_output() {
        let a = generate_cards(50, Some(1));
        let b = generate_cards(50, Some(2));
        let any_different = a.iter().zip(b.iter()).any(|(ca, cb)| ca.grid != cb.grid);
        assert!(any_different);
    }
}
