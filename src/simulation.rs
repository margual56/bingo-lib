use std::cmp::Ordering;

use rand::seq::SliceRandom;
use serde::Serialize;

use crate::card::{Card, NUM_ROWS};
use crate::rng::make_rng;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationResult {
    pub iterations: u32,
    pub p_double_bingo: f64,
    pub p_triple_plus_bingo: f64,
    pub p_double_linea: f64,
    pub avg_balls_to_bingo: f64,
}

struct CardRows {
    numbers: Vec<u16>,
    rows: [Vec<u16>; NUM_ROWS],
}

fn extract_card_rows(cards: &[Card]) -> Vec<CardRows> {
    cards
        .iter()
        .map(|card| {
            let numbers = card.all_numbers();
            let rows = std::array::from_fn(|r| card.row_numbers(r));
            CardRows { numbers, rows }
        })
        .collect()
}

/// Monte Carlo estimate of "double bingo"/"double línea" (simultaneous-win)
/// probability for the given set of cards. Per iteration, shuffles the 90
/// balls into a draw order; each card's bingo-completion position is the
/// draw-index of the last of its 15 numbers to appear, and its línea
/// (row)-completion position is the earliest of its 3 rows' own
/// last-number draw-index. The card(s) with the minimum completion
/// position win; a tie for the minimum is a double/multi win.
pub fn simulate_game(cards: &[Card], iterations: u32, seed: Option<u64>) -> SimulationResult {
    if cards.is_empty() || iterations == 0 {
        return SimulationResult {
            iterations,
            p_double_bingo: 0.0,
            p_triple_plus_bingo: 0.0,
            p_double_linea: 0.0,
            avg_balls_to_bingo: 0.0,
        };
    }

    let mut rng = make_rng(seed);
    let card_rows = extract_card_rows(cards);

    let mut double_bingo_count: u32 = 0;
    let mut triple_plus_count: u32 = 0;
    let mut double_linea_count: u32 = 0;
    let mut total_balls_to_bingo: u64 = 0;

    let mut balls: Vec<u16> = (1..=90).collect();
    let mut position_of_ball = [0u16; 91];

    for _ in 0..iterations {
        balls.shuffle(&mut rng);
        for (idx, &ball) in balls.iter().enumerate() {
            position_of_ball[ball as usize] = idx as u16;
        }

        let mut min_bingo_pos = u16::MAX;
        let mut bingo_winners: u32 = 0;
        let mut min_linea_pos = u16::MAX;
        let mut linea_winners: u32 = 0;

        for cr in &card_rows {
            let bingo_pos = cr
                .numbers
                .iter()
                .map(|&num| position_of_ball[num as usize])
                .max()
                .unwrap_or(0);
            let linea_pos = cr
                .rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|&num| position_of_ball[num as usize])
                        .max()
                        .unwrap_or(0)
                })
                .min()
                .unwrap_or(0);

            match bingo_pos.cmp(&min_bingo_pos) {
                Ordering::Less => {
                    min_bingo_pos = bingo_pos;
                    bingo_winners = 1;
                }
                Ordering::Equal => bingo_winners += 1,
                Ordering::Greater => {}
            }
            match linea_pos.cmp(&min_linea_pos) {
                Ordering::Less => {
                    min_linea_pos = linea_pos;
                    linea_winners = 1;
                }
                Ordering::Equal => linea_winners += 1,
                Ordering::Greater => {}
            }
        }

        if bingo_winners >= 2 {
            double_bingo_count += 1;
        }
        if bingo_winners >= 3 {
            triple_plus_count += 1;
        }
        if linea_winners >= 2 {
            double_linea_count += 1;
        }
        total_balls_to_bingo += min_bingo_pos as u64 + 1;
    }

    let iters = iterations as f64;
    SimulationResult {
        iterations,
        p_double_bingo: double_bingo_count as f64 / iters,
        p_triple_plus_bingo: triple_plus_count as f64 / iters,
        p_double_linea: double_linea_count as f64 / iters,
        avg_balls_to_bingo: total_balls_to_bingo as f64 / iters,
    }
}

/// Sensible default iteration count that scales inversely with N so total
/// work (iterations * n) stays roughly bounded.
pub fn default_iterations(n: u32) -> u32 {
    if n == 0 {
        return 500;
    }
    let scaled = 2_000_000u64 / n as u64;
    scaled.clamp(500, 8_000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate_cards;

    #[test]
    fn probabilities_are_within_bounds() {
        let cards = generate_cards(10, Some(42));
        let result = simulate_game(&cards, 200, Some(7));
        assert!((0.0..=1.0).contains(&result.p_double_bingo));
        assert!((0.0..=1.0).contains(&result.p_triple_plus_bingo));
        assert!((0.0..=1.0).contains(&result.p_double_linea));
        assert!(result.avg_balls_to_bingo >= 15.0 && result.avg_balls_to_bingo <= 90.0);
    }

    #[test]
    fn double_bingo_probability_increases_with_more_cards() {
        let small = generate_cards(5, Some(1));
        let large = generate_cards(200, Some(1));
        let small_result = simulate_game(&small, 5_000, Some(99));
        let large_result = simulate_game(&large, 5_000, Some(99));
        assert!(large_result.p_double_bingo >= small_result.p_double_bingo);
    }
}
