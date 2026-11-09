//! Small helpers shared by systems.
use sim_params::{Params, SystemConfig};
use sim_types::*;

/// The rule overrides of the active economic system.
pub fn cfg<'a>(s: &State, p: &'a Params) -> &'a SystemConfig {
    p.systems.get(s.system)
}

/// Firm monthly interest rate factor helper: annual rate to monthly.
pub fn monthly(rate: f64) -> f64 {
    rate / 12.0
}

/// Is this cohort a skilled (premium wage) tier?
pub fn is_skilled(h: &Household) -> bool {
    h.wage_premium.0 > 1.0
}

/// People on a firm's payroll weighted by their wage premium (the wage bill divided by the base wage).
pub fn paid_workers(s: &State, f: &Firm) -> f64 {
    s.households
        .iter()
        .map(|h| h.employment[f.id.idx()].0 * h.wage_premium.0)
        .sum()
}

/// Cash a firm keeps back for payroll and debt service before it spends on capital.
pub fn cash_reserve(s: &State, p: &Params, f: &Firm) -> Cents {
    let wage_bill = f.wage.scale(paid_workers(s, f));
    let service = f.debt.scale(monthly(f.loan_rate.0)) + f.amort.min(f.debt);
    (wage_bill + service).scale(p.invest.min_cash_months)
}

/// Cash a firm can spend on capital goods this tick.
pub fn capex_available(s: &State, p: &Params, f: &Firm) -> Cents {
    let avail = (f.cash - cash_reserve(s, p, f)).max(Cents::ZERO);
    if f.green_plan {
        avail.scale(1.0 - cfg(s, p).green_cash_share)
    } else {
        avail
    }
}

/// Carbon price firms plan with: the effective price, pulled toward a still-pending price
/// by `announce_discount`.
pub fn announced_carbon_price(s: &State, p: &Params) -> Cents {
    let now = s.effective.carbon_price;
    let pending = s.pending.iter().rev().find_map(|x| match x.command {
        Command::SetPolicy(PolicyChange::CarbonPrice(c)) => Some(c),
        _ => None,
    });
    match pending {
        Some(c) => now + (c - now).scale(p.turn.announce_discount),
        None => now,
    }
}

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
