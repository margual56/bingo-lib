use serde::Serialize;

use crate::card::column_sizes;

/// Computes the elementary symmetric polynomial `e_k` of the given values,
/// i.e. the sum, over every k-subset of `values`, of the product of the
/// subset's elements. Used here to count how many distinct valid bingo rows
/// exist: choose 5 of the 9 columns and one number from each.
pub fn elementary_symmetric_k(values: &[u32], k: usize) -> u64 {
    let mut e = vec![0u64; k + 1];
    e[0] = 1;
    for &v in values {
        let mut j = k.min(e.len() - 1);
        while j >= 1 {
            e[j] += e[j - 1] * v as u64;
            j -= 1;
        }
    }
    e[k]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombinatorialStats {
    pub n: u32,
    pub total_rows_possible: u64,
    pub rows_used: u64,
    pub usage_percent: f64,
    pub max_theoretical_n: u64,
    /// Illustrative-only: the probability a *naive* generator (no
    /// deduplication) would have produced at least one duplicate row for
    /// this many rows, via the birthday-paradox approximation. Our actual
    /// generator guarantees 0% via the global uniqueness `HashSet`.
    pub naive_duplicate_probability: f64,
}

pub fn combinatorial_stats(n: u32) -> CombinatorialStats {
    let sizes = column_sizes();
    let total_rows_possible = elementary_symmetric_k(&sizes, 5);
    let rows_used = n as u64 * 3;
    let usage_percent = if total_rows_possible == 0 {
        0.0
    } else {
        rows_used as f64 / total_rows_possible as f64 * 100.0
    };
    let max_theoretical_n = total_rows_possible / 3;
    let naive_duplicate_probability = naive_duplicate_probability(rows_used, total_rows_possible);

    CombinatorialStats {
        n,
        total_rows_possible,
        rows_used,
        usage_percent,
        max_theoretical_n,
        naive_duplicate_probability,
    }
}

fn naive_duplicate_probability(rows: u64, total_rows_possible: u64) -> f64 {
    if rows == 0 || total_rows_possible == 0 {
        return 0.0;
    }
    let k = rows as f64;
    let r = total_rows_possible as f64;
    let exponent = -(k * (k - 1.0)) / (2.0 * r);
    1.0 - exponent.exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_rows_possible_matches_known_value() {
        let sizes = column_sizes();
        assert_eq!(elementary_symmetric_k(&sizes, 5), 12_565_000);
    }

    #[test]
    fn max_theoretical_n_matches_known_value() {
        let sizes = column_sizes();
        let r = elementary_symmetric_k(&sizes, 5);
        assert_eq!(r / 3, 4_188_333);
    }
}
