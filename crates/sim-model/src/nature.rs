//! S7 natural capital and the carbon budget: forest, commons, sink, cumulative emissions.

use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

/// Maximum sustainable yield of logistic growth `r * K * x * (1 - x)` is `r * K / 4`: a
/// mathematical identity of that curve, not a game number.
const LOGISTIC_MSY_DIVISOR: f64 = 4.0;

/// Tonnes of CO2 the forest absorbs per month: `sink_sigma` per unit of standing forest.
pub fn forest_sink(p: &Params, forest: f64) -> f64 {
    p.nature.sink_sigma * forest
}

pub fn run(s: &State, p: &Params, r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let np = &p.nature;
    let mut n = s.nature.clone();
    let gi = s.energy.grid_intensity.0;

    // Emissions by source. Firms and households report only their direct emissions here: the
    // electricity they use is emitted by the plants that burn the fuel, and counted once, there.
    let mut firm_e = 0.0;
    for f in s.firms.iter().filter(|f| f.active) {
        firm_e += f.emissions.0;
        d.trace.push(TraceRecord::Emissions {
            source: EmissionSource::Firm(f.id),
            tonnes: f.emissions,
        });
    }
    let mut hh_e = 0.0;
    // Town v7: the estate household's direct figure replaces the per-person one, and a flight
    // this month is its own source (Aviation), counted in the estate's footprint.
    let estate_hh = s.estate.as_ref().map(|e| e.household);
    let flight_e = match &s.estate {
        Some(e) if e.jet.flying_this_month => p.estate.flight_tonnes,
        _ => 0.0,
    };
    for h in &s.households {
        let is_estate = estate_hh == Some(h.id);
        let direct = if is_estate {
            p.estate.household_emissions_t
        } else {
            h.members.len() as f64 * np.household_footprint_base
        };
        hh_e += direct;
        let own = if is_estate { flight_e } else { 0.0 };
        d.household(
            h.id,
            HouseholdPatch {
                footprint: Some(Tonnes(direct + own + h.energy_use.0 * gi)),
                ..HouseholdPatch::default()
            },
        );
    }
    d.trace.push(TraceRecord::Emissions {
        source: EmissionSource::Households,
        tonnes: Tonnes(hh_e),
    });
    if flight_e > 0.0 {
        d.trace.push(TraceRecord::Emissions {
            source: EmissionSource::Aviation,
            tonnes: Tonnes(flight_e),
        });
    }
    let mut plant_e = 0.0;
    for pl in &s.plants {
        plant_e += pl.emissions.0;
        d.trace.push(TraceRecord::Emissions {
            source: EmissionSource::Plant(pl.id),
            tonnes: pl.emissions,
        });
    }

    // Harvests by the farm, under the commons access rule.
    let msy = np.commons_growth * np.commons_max / LOGISTIC_MSY_DIVISOR;
    let mut harvest_c = 0.0;
    let mut harvest_f = 0.0;
    let mut enclosure_firms: Vec<FirmId> = Vec::new();
    for f in s
        .firms
        .iter()
        .filter(|f| f.active && f.sector == Sector::Food)
    {
        harvest_f += f.output.0 * np.farm_harvest_forest;
        harvest_c += f.output.0 * np.farm_harvest_commons * s.policy.commons_harvest.0;
        enclosure_firms.push(f.id);
    }
    match n.commons_access {
        CommonsAccess::Open => {}
        CommonsAccess::Enclosed(owner) => {
            let before = harvest_c;
            harvest_c *= np.enclosure_harvest_multiplier;
            // The owner charges a fee for every unit taken.
            let fee = Cents::from_dollars(np.enclosure_fee_usd * harvest_c);
            if before > 0.0 {
                for fid in &enclosure_firms {
                    d.transfer_as(
                        Account::Firm(*fid),
                        Account::Household(owner),
                        fee.scale(1.0 / enclosure_firms.len() as f64),
                        FlowKind::Rent,
                        Purpose::CommonsFee,
                    );
                }
            }
        }
        CommonsAccess::Community { cap } => {
            harvest_c = harvest_c.min(cap.0 * msy);
        }
    }
    let harvest_c = harvest_c.min(n.commons.0);

    // Stocks.
    let f0 = n.forest.0;
    let c0 = n.commons.0;
    let sink = forest_sink(p, f0);
    let df = np.forest_growth * f0 * (1.0 - f0 / np.forest_max) - harvest_f - n.pending_clearing.0;
    // Drought (S11) slows commons regrowth; a community-managed commons keeps part of it.
    let buffer = match n.commons_access {
        CommonsAccess::Community { .. } => p.weather.community_buffer,
        _ => 0.0,
    };
    let drought_factor =
        1.0 - p.weather.a_c * crate::weather::effective_drought(s, p) * (1.0 - buffer);
    let dc =
        np.commons_growth * c0 * (1.0 - c0 / np.commons_max) * drought_factor.max(0.0) - harvest_c;
    n.forest = Units((f0 + df).clamp(0.0, np.forest_max));
    n.commons = Units((c0 + dc).clamp(0.0, np.commons_max));
    n.pending_clearing = Units::ZERO;
    n.harvest_commons = Units(harvest_c);
    n.harvest_forest = Units(harvest_f);
    n.depletion = Units(((f0 + c0) - (n.forest.0 + n.commons.0)).max(0.0));
    n.sink = Tonnes(sink);
    n.gross_emissions = Tonnes(firm_e + hh_e + plant_e + flight_e);
    n.net_emissions = Tonnes(firm_e + hh_e + plant_e + flight_e - sink);
    n.cumulative = Tonnes((n.cumulative.0 + n.net_emissions.0).max(0.0));
    n.atmosphere = Tonnes((n.atmosphere.0 + n.net_emissions.0).max(0.0));
    d.trace.push(TraceRecord::Emissions {
        source: EmissionSource::Sink,
        tonnes: Tonnes(-sink),
    });

    if n.forest.0 < np.forest_shrank_share * f0 && f0 > 0.0 {
        d.events.push(Event::ForestShrank { stock: n.forest });
    }
    if n.commons.0 < np.commons_collapse_share * np.commons_max
        && c0 >= np.commons_collapse_share * np.commons_max
    {
        d.events.push(Event::CommonsCollapsed);
    }

    // Enclosure: a wealthy capital-owning cohort may fence off an open commons.
    let c = crate::util::cfg(s, p);
    // The draw is always taken, so the commons rule the player chose never shifts the shared stream.
    let enclosure_roll = r.chance(c.enclosure_prob);
    if let CommonsAccess::Open = n.commons_access {
        if c.enclosure_prob > 0.0 && enclosure_roll {
            if let Some(h) = s
                .households
                .iter()
                .filter(|h| matches!(h.class, Class::Capitalist | Class::Rentier))
                .max_by_key(|h| h.savings)
            {
                n.commons_access = CommonsAccess::Enclosed(h.id);
                d.events.push(Event::CommonsEnclosed { owner: h.id });
            }
        }
    }
    d.changes.push(Delta::Nature(n));
    d
}
