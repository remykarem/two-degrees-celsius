//! S9b political power and lobbying: wealth buys influence, which erodes policy levers.
//! effective = nominal x (1 - net_lobby), applied to carbon price, profit tax, wealth tax and caps.

use crate::util::cfg;
use sim_metrics::cohort_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let lp = &p.lobby;
    let c = cfg(s, p);

    // Capital cohorts spend a share of their capital income on lobbying, up to the cap.
    let share = lp.spend_rate.min(s.policy.campaign_cap.0);
    let lobbyist = s
        .households
        .iter()
        .filter(|h| h.class == Class::Manager)
        .max_by(|a, b| a.weight.0.partial_cmp(&b.weight.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut total_spend = 0.0;
    if let Some(lob) = lobbyist {
        for h in s.households.iter().filter(|h| matches!(h.class, Class::Capitalist | Class::Rentier)) {
            let spend = h.income.capital_income().scale(share);
            total_spend += spend.dollars();
            d.transfer(Account::Household(h.id), Account::Household(lob.id), spend, FlowKind::Lobby);
        }
    }
    let gdp_month = s.indicators.gdp_month.dollars().max(1.0);
    let lobby_capital = (lp.k * total_spend / gdp_month).min(c.lambda_max);

    // Organised labour pushes back; union density rises when unemployment is low and in co-ops.
    let u = s.indicators.unemployment.0;
    let drift = lp.union_adjust * (lp.union_target_unemployment - u) / lp.union_target_unemployment;
    let union = (s.union_density.0 + drift).clamp(lp.union_min, lp.union_max).max(c.union_floor);
    let lobby_labour = union * s.indicators.wage_share.0 * lp.labour_u;
    let net = (lobby_capital - lobby_labour).max(0.0).min(0.95);

    let eff = EffectivePolicy {
        carbon_price: s.policy.carbon_price.scale(1.0 - net),
        wealth_tax: Rate(s.policy.wealth_tax.0 * (1.0 - net)),
        tax_profit: Rate(s.policy.tax_profit.0 * (1.0 - net)),
        emissions_cap: if s.policy.emissions_cap.0 > 0.0 {
            Tonnes(s.policy.emissions_cap.0 / (1.0 - net))
        } else {
            Tonnes::ZERO
        },
    };
    for (lever, nominal, effective) in [
        (Lever::CarbonPrice, s.policy.carbon_price.dollars(), eff.carbon_price.dollars()),
        (Lever::WealthTax, s.policy.wealth_tax.0, eff.wealth_tax.0),
        (Lever::ProfitTax, s.policy.tax_profit.0, eff.tax_profit.0),
        (Lever::EmissionsCap, s.policy.emissions_cap.0, eff.emissions_cap.0),
    ] {
        d.trace.push(TraceRecord::PolicyGap { lever, nominal, effective, net_lobby: Rate(net) });
    }
    d.changes.push(Delta::Effective(eff));
    d.changes.push(Delta::UnionDensity(Rate(union)));
    d.changes.push(Delta::NetLobby(Rate(net)));

    // Political power: share of positive net wealth.
    let w = cohort_wealth(s, p);
    let total: f64 = w.iter().map(|x| x.max(0.0)).sum();
    for (i, h) in s.households.iter().enumerate() {
        d.household(
            h.id,
            HouseholdPatch {
                political_power: Some(Rate(safe_div(w[i].max(0.0), total, 0.0))),
                ..HouseholdPatch::default()
            },
        );
    }
    d
}
