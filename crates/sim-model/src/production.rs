//! S2 production and emissions: Y = A K^a N^b, E = Y * intensity.

use libm::pow;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

/// Resource health of the farm: average of forest and commons relative to capacity.
pub fn resource_health(s: &State, p: &Params) -> f64 {
    let f = s.nature.forest.0 / p.nature.forest_max;
    let c = s.nature.commons.0 / p.nature.commons_max;
    (0.5 * (f + c)).clamp(0.0, 1.0)
}

/// Share of farm output lost to climate damage: 0.3 * (CE/B0)^2.
pub fn climate_factor(s: &State, p: &Params) -> f64 {
    let ratio = (s.nature.cumulative.0 / p.nature.budget_b0).clamp(0.0, 1.5);
    1.0 - p.production.climate_damage * ratio * ratio
}

/// Output the firm would produce with `n` workers, before any emissions cap.
pub fn capacity(s: &State, p: &Params, f: &Firm, n: f64) -> f64 {
    if n <= 0.0 || f.capital.0 <= 0.0 {
        return 0.0;
    }
    let mut y = f.tfp.0 * pow(f.capital.0, p.production.alpha_k) * pow(n, p.production.beta_n);
    if f.sector == Sector::Food {
        y *= pow(
            resource_health(s, p).max(1e-6),
            p.production.farm_health_exp,
        );
        y *= climate_factor(s, p).max(0.0);
        if let CommonsAccess::Community { .. } = s.nature.commons_access {
            y *= 1.0 - p.nature.community_coord_cost;
        }
    }
    y * (1.0 - s.output_penalty.0).max(0.0)
}

/// Emission intensity in tonnes per unit of output.
pub fn intensity(s: &State, f: &Firm) -> f64 {
    f.intensity_base.0 * (1.0 - f.green_tech.0) * s.energy.grid_factor()
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    for f in s.firms.iter().filter(|f| f.active) {
        let n: f64 = s
            .households
            .iter()
            .map(|h| h.employment[f.id.idx()].0)
            .sum();
        let mut y = capacity(s, p, f, n);
        let iota = intensity(s, f);
        let cap = s.effective.emissions_cap.0;
        if cap > 0.0 && iota > 0.0 {
            y = y.min(cap / iota);
        }
        let e = y * iota;
        let growth = match f.sector {
            Sector::Goods => p.production.tfp_growth_goods,
            Sector::Food => p.production.tfp_growth_food,
        };
        d.firm(
            f.id,
            FirmPatch {
                output: Some(Units(y)),
                emissions: Some(Tonnes(e)),
                inventory: Some(Units(f.inventory.0 * (1.0 - p.goods.holding_cost) + y)),
                tfp: Some(Rate(f.tfp.0 * (1.0 + growth))),
                workers: Some(People(n)),
                ..FirmPatch::default()
            },
        );
    }
    d
}
