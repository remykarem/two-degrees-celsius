//! Small helpers shared by systems.
use sim_params::{Params, SystemConfig};
use sim_types::*;

/// The rule overrides of the active economic system.
pub fn cfg<'a>(s: &State, p: &'a Params) -> &'a SystemConfig {
    p.systems.get(s.system)
}

/// Months in a year (the tick is a month); a unit conversion, not a game number.
pub const MONTHS_PER_YEAR: f64 = 12.0;

/// Floor that keeps a divisor or a power base away from exactly zero (numerical guard only).
pub const NUMERIC_FLOOR: f64 = 1e-6;

/// Smaller floor for dividing by an average of positive quantities (numerical guard only).
pub const TINY_DIVISOR: f64 = 1e-9;

/// Firm monthly interest rate factor helper: annual rate to monthly.
pub fn monthly(rate: f64) -> f64 {
    rate / MONTHS_PER_YEAR
}

/// Pay multiple of a person with this skill: 1 at the reference skill, rising with skill.
pub fn skill_mult(p: &Params, skill: f64) -> f64 {
    let lp = &p.labour;
    (1.0 + lp.skill_wage_slope * (skill - lp.skill_wage_ref)).max(lp.skill_wage_floor)
}

/// Monthly pay of a person employed at a firm paying base wage `wage`: the base wage times the
/// person's skill multiple, never below the minimum wage.
pub fn person_pay(s: &State, p: &Params, person: &Person, wage: Cents) -> Cents {
    let min_wage = s.policy.min_wage.scale(cfg(s, p).min_wage_multiplier);
    wage.scale(skill_mult(p, person.skill.0)).max(min_wage)
}

/// Persons employed by a firm, in id order.
pub fn workers_of(s: &State, f: FirmId) -> impl Iterator<Item = &Person> {
    s.persons.iter().filter(move |x| x.job == Some(f))
}

/// The firm's monthly wage bill at current pay.
pub fn wage_bill(s: &State, p: &Params, f: &Firm) -> Cents {
    Cents::sum(workers_of(s, f.id).map(|x| person_pay(s, p, x, f.wage)))
}

/// Cash a firm keeps back for payroll and debt service before it spends on capital.
pub fn cash_reserve(s: &State, p: &Params, f: &Firm) -> Cents {
    let service = f.debt.scale(monthly(f.loan_rate.0)) + f.amort.min(f.debt);
    (wage_bill(s, p, f) + service).scale(p.invest.min_cash_months)
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

/// Weights for paying construction and maintenance work: the labour-force adults of each
/// household (so every household that supplies labour earns a share).
pub fn labour_weights(s: &State) -> Vec<f64> {
    s.households
        .iter()
        .map(|h| {
            h.members
                .iter()
                .filter(|m| s.person(**m).in_labour_force)
                .count() as f64
        })
        .collect()
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

/// Tax on a base at a flat rate (wage tax on wages, capital tax on capital income).
pub fn tax_on(base: Cents, rate: f64) -> Cents {
    base.scale(rate)
}

/// Profit tax due on a firm's pre-tax profit: nothing on a loss.
pub fn profit_tax_due(pre_tax: Cents, rate: f64) -> Cents {
    pre_tax.max(Cents::ZERO).scale(rate)
}

/// Monthly wealth tax on a household's net worth (`wealth_cents`, in cents): the annual rate on
/// what is above the threshold, one twelfth a month.
pub fn wealth_tax_monthly(wealth_cents: f64, threshold: Cents, annual_rate: f64) -> Cents {
    let over = (wealth_cents / 100.0 - threshold.dollars()).max(0.0);
    Cents::from_dollars(over * annual_rate / MONTHS_PER_YEAR)
}

/// Carbon price paid on a footprint (tonnes of CO2).
pub fn carbon_cost(price: Cents, footprint_tonnes: f64) -> Cents {
    Cents::from_f64(price.as_f64() * footprint_tonnes)
}

/// The most the bank can have lent: equity over the capital ratio.
pub fn max_loans(equity: Cents, capital_ratio: f64) -> Cents {
    equity.max(Cents::ZERO).scale(1.0 / capital_ratio)
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
