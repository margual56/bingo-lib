use rand::rngs::SmallRng;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::Rng;

pub const NUM_COLUMNS: usize = 9;
pub const NUM_ROWS: usize = 3;
pub const CARD_CELLS: usize = NUM_ROWS * NUM_COLUMNS;
pub const NUMBERS_PER_ROW: usize = 5;
pub const NUMBERS_PER_CARD: usize = 15;
pub const MAX_PER_COLUMN: usize = 3;

/// Inclusive (start, end) number range per column, standard 90-ball layout.
pub const COLUMN_RANGES: [(u16, u16); NUM_COLUMNS] = [
    (1, 9),
    (10, 19),
    (20, 29),
    (30, 39),
    (40, 49),
    (50, 59),
    (60, 69),
    (70, 79),
    (80, 90),
];

pub fn column_sizes() -> [u32; NUM_COLUMNS] {
    let mut sizes = [0u32; NUM_COLUMNS];
    for (i, &(start, end)) in COLUMN_RANGES.iter().enumerate() {
        sizes[i] = (end - start + 1) as u32;
    }
    sizes
}

#[derive(Debug, Clone)]
pub struct Card {
    /// grid[row][col] = 0 means blank, otherwise the number (1..=90).
    pub grid: [[u16; NUM_COLUMNS]; NUM_ROWS],
}

impl Card {
    pub fn flatten(&self) -> Vec<u16> {
        let mut out = Vec::with_capacity(CARD_CELLS);
        for row in &self.grid {
            out.extend_from_slice(row);
        }
        out
    }

    pub fn from_flat(flat: &[u16]) -> Self {
        let mut grid = [[0u16; NUM_COLUMNS]; NUM_ROWS];
        for row in 0..NUM_ROWS {
            for col in 0..NUM_COLUMNS {
                grid[row][col] = flat[row * NUM_COLUMNS + col];
            }
        }
        Card { grid }
    }

    /// Canonical hashable identity of a row: a 90-bit mask (fits in u128)
    /// with bit (number - 1) set for each of the row's filled numbers.
    /// Because the column ranges are disjoint, the set of numbers alone
    /// fully determines which columns were used, so no separate
    /// column-choice encoding is needed.
    pub fn row_key(&self, row: usize) -> u128 {
        let mut key: u128 = 0;
        for col in 0..NUM_COLUMNS {
            let n = self.grid[row][col];
            if n != 0 {
                key |= 1u128 << (n - 1);
            }
        }
        key
    }

    pub fn row_keys(&self) -> [u128; NUM_ROWS] {
        std::array::from_fn(|row| self.row_key(row))
    }

    /// The filled (non-blank) numbers of a single row, in column order.
    pub fn row_numbers(&self, row: usize) -> Vec<u16> {
        self.grid[row].iter().copied().filter(|&n| n != 0).collect()
    }

    pub fn all_numbers(&self) -> Vec<u16> {
        (0..NUM_ROWS).flat_map(|row| self.row_numbers(row)).collect()
    }
}

/// Generates a random valid 3x9 boolean fill pattern: each row has exactly
/// `NUMBERS_PER_ROW` filled cells, each column has between 1 and
/// `MAX_PER_COLUMN` filled cells, and the total is `NUMBERS_PER_CARD`.
/// Retries from scratch (cheap at this scale) if a partial assignment gets
/// stuck rather than doing real backtracking.
pub fn generate_fill_pattern(rng: &mut SmallRng) -> [[bool; NUM_COLUMNS]; NUM_ROWS] {
    loop {
        if let Some(pattern) = try_generate_fill_pattern(rng) {
            return pattern;
        }
    }
}

fn try_generate_fill_pattern(rng: &mut SmallRng) -> Option<[[bool; NUM_COLUMNS]; NUM_ROWS]> {
    let mut matrix = [[false; NUM_COLUMNS]; NUM_ROWS];
    let mut row_count = [0usize; NUM_ROWS];
    let mut col_count = [0usize; NUM_COLUMNS];

    // Seed phase: every column gets exactly one filled cell, in a random row
    // that still has capacity (rows must not exceed NUMBERS_PER_ROW even
    // during seeding, or the distribution phase below can get stuck).
    let mut columns: [usize; NUM_COLUMNS] = std::array::from_fn(|i| i);
    columns.shuffle(rng);
    for &col in &columns {
        // Fixed-size scratch buffer: at most NUM_ROWS candidates, no heap
        // allocation in this per-attempt, potentially hot retry loop.
        let mut row_candidates = [0usize; NUM_ROWS];
        let mut len = 0;
        for r in 0..NUM_ROWS {
            if row_count[r] < NUMBERS_PER_ROW {
                row_candidates[len] = r;
                len += 1;
            }
        }
        let row = *row_candidates[..len].choose(rng)?;
        matrix[row][col] = true;
        row_count[row] += 1;
        col_count[col] += 1;
    }

    // Distribution phase: place the remaining marks one at a time, picking
    // uniformly among still-valid (row, col) slots.
    let remaining = NUMBERS_PER_CARD - NUM_COLUMNS;
    for _ in 0..remaining {
        // Fixed-size scratch buffer: at most CARD_CELLS candidates, no heap
        // allocation in this per-attempt, potentially hot retry loop.
        let mut candidates = [(0usize, 0usize); CARD_CELLS];
        let mut len = 0;
        for r in 0..NUM_ROWS {
            for c in 0..NUM_COLUMNS {
                if !matrix[r][c] && row_count[r] < NUMBERS_PER_ROW && col_count[c] < MAX_PER_COLUMN
                {
                    candidates[len] = (r, c);
                    len += 1;
                }
            }
        }
        let &(row, col) = candidates[..len].choose(rng)?;
        matrix[row][col] = true;
        row_count[row] += 1;
        col_count[col] += 1;
    }

    debug_assert!(row_count.iter().all(|&c| c == NUMBERS_PER_ROW));
    debug_assert!(col_count.iter().all(|&c| (1..=MAX_PER_COLUMN).contains(&c)));
    debug_assert_eq!(col_count.iter().sum::<usize>(), NUMBERS_PER_CARD);

    Some(matrix)
}

/// Generates a single structurally valid card (fill pattern + numbers),
/// without any awareness of global row uniqueness — that's handled by the
/// caller in `generation.rs`.
pub fn generate_card(rng: &mut SmallRng) -> Card {
    let pattern = generate_fill_pattern(rng);
    let mut grid = [[0u16; NUM_COLUMNS]; NUM_ROWS];

    for col in 0..NUM_COLUMNS {
        let rows_filled: Vec<usize> = (0..NUM_ROWS).filter(|&r| pattern[r][col]).collect();
        let k = rows_filled.len();
        let (start, end) = COLUMN_RANGES[col];
        let size = (end - start + 1) as usize;

        // Partial Fisher-Yates: only need the first k positions shuffled to
        // get k distinct random numbers from the column's range.
        let mut pool: Vec<u16> = (start..=end).collect();
        for i in 0..k {
            let j = rng.random_range(i..size);
            pool.swap(i, j);
        }
        pool[0..k].sort_unstable();

        for (idx, &row) in rows_filled.iter().enumerate() {
            grid[row][col] = pool[idx];
        }
    }

    Card { grid }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::make_rng;

    #[test]
    fn generated_cards_are_structurally_valid() {
        let mut rng = make_rng(Some(123));
        for _ in 0..500 {
            let card = generate_card(&mut rng);

            // Each row has exactly NUMBERS_PER_ROW filled cells.
            for row in &card.grid {
                let filled = row.iter().filter(|&&n| n != 0).count();
                assert_eq!(filled, NUMBERS_PER_ROW);
            }

            let mut seen = std::collections::HashSet::new();
            for col in 0..NUM_COLUMNS {
                let (start, end) = COLUMN_RANGES[col];
                let mut prev = 0u16;
                let mut count = 0usize;
                for row in 0..NUM_ROWS {
                    let n = card.grid[row][col];
                    if n == 0 {
                        continue;
                    }
                    count += 1;
                    assert!(n >= start && n <= end, "number {n} out of range for column {col}");
                    assert!(n > prev, "column {col} not strictly ascending");
                    prev = n;
                    assert!(seen.insert(n), "number {n} repeated within the same card");
                }
                assert!((1..=MAX_PER_COLUMN).contains(&count), "column {col} has {count} numbers");
            }

            assert_eq!(seen.len(), NUMBERS_PER_CARD);
        }
    }
}
