mod card;
mod generation;
mod rng;
mod simulation;
mod stats;

use wasm_bindgen::prelude::*;

use card::Card;

#[wasm_bindgen(start)]
fn wasm_start() {
    console_error_panic_hook::set_once();
}

/// Generates `n` bingo cards (no repeated rows across the batch) and
/// returns them as a flat, row-major `Uint16Array` of length `n * 27`
/// (3 rows * 9 columns per card, 0 = blank cell).
#[wasm_bindgen(js_name = generateCards)]
pub fn generate_cards(n: u32, seed: Option<u64>) -> Vec<u16> {
    let cards = generation::generate_cards(n, seed);
    generation::flatten_cards(&cards)
}

/// Instant combinatorial statistics for a set of `n` cards (does not
/// require actually generating the cards).
#[wasm_bindgen(js_name = combinatorialStats)]
pub fn combinatorial_stats(n: u32) -> Result<JsValue, JsValue> {
    let stats = stats::combinatorial_stats(n);
    serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// A sensible default Monte Carlo iteration count for `n` cards.
#[wasm_bindgen(js_name = defaultIterations)]
pub fn default_iterations(n: u32) -> u32 {
    simulation::default_iterations(n)
}

/// Runs a Monte Carlo simulation of real bingo games over the given set of
/// cards (as returned by `generateCards`) to estimate the probability of a
/// simultaneous "doble bingo" / "doble línea". The card count is derived
/// from `cards_flat`'s length rather than taken as a separate parameter, so
/// there's only one source of truth for how many cards it describes.
#[wasm_bindgen(js_name = simulateGame)]
pub fn simulate_game(
    cards_flat: &[u16],
    iterations: u32,
    seed: Option<u64>,
) -> Result<JsValue, JsValue> {
    if cards_flat.len() % card::CARD_CELLS != 0 {
        return Err(JsValue::from_str(&format!(
            "cards_flat length {} is not a multiple of {} (rows * columns per card)",
            cards_flat.len(),
            card::CARD_CELLS
        )));
    }

    let cards: Vec<Card> = cards_flat
        .chunks_exact(card::CARD_CELLS)
        .map(Card::from_flat)
        .collect();

    let result = simulation::simulate_game(&cards, iterations, seed);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
