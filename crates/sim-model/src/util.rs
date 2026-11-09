//! Small helpers shared by systems.
use sim_types::*;

/// Split `total` cents across `weights` so the parts sum exactly to `total`
/// (largest remainder method, ties broken by index). Non-positive weights get zero.
pub fn apportion(total: Cents, weights: &[f64]) -> Vec<Cents> {
    let n = weights.len();
    let mut out = vec![Cents::ZERO; n];
    let sum: f64 = weights.iter().map(|w| w.max(0.0)).sum();
    if total <= Cents::ZERO || sum <= 0.0 || !sum.is_finite() {
        return out;
    }
    let t = total.0 as f64;
    let mut rem: Vec<(f64, usize)> = Vec::with_capacity(n);
    let mut used = 0i64;
    for (i, w) in weights.iter().enumerate() {
        let exact = t * w.max(0.0) / sum;
        let fl = exact.floor();
        out[i] = Cents(fl as i64);
        used += fl as i64;
        rem.push((exact - fl, i));
    }
    let mut left = total.0 - used;
    rem.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
    });
    for (_, i) in rem {
        if left <= 0 {
            break;
        }
        if weights[i] > 0.0 {
            out[i] += Cents(1);
            left -= 1;
        }
    }
    // Any leftover from float error goes to the largest weight.
    if left != 0 {
        if let Some((i, _)) = weights
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            out[i] += Cents(left);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apportion_sums_exactly() {
        let parts = apportion(Cents(1000), &[1.0, 1.0, 1.0]);
        assert_eq!(Cents::sum(parts.iter().copied()), Cents(1000));
        let parts = apportion(Cents(7), &[0.3, 0.3, 0.4, 0.0]);
        assert_eq!(Cents::sum(parts.iter().copied()), Cents(7));
        assert_eq!(parts[3], Cents(0));
    }

    #[test]
    fn apportion_empty_weights() {
        let parts = apportion(Cents(10), &[0.0, 0.0]);
        assert_eq!(Cents::sum(parts.iter().copied()), Cents(0));
    }
}
