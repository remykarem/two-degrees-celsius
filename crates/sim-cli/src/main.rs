//! Headless runner: `cargo run -p sim-cli -- --seed 42`
//! Runs the monthly simulation, prints a summary, optionally writes a CSV.
//! This is the only crate that does file I/O.
#![forbid(unsafe_code)]

use clap::Parser;
use sim_engine::{series, Script, Sim};
use sim_params::{ContentJson, Params};
use sim_types::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "sim", about = "Headless ecological-economics town simulation")]
struct Args {
    /// Random seed.
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Number of monthly ticks (default: from params.json, 480).
    #[arg(long)]
    ticks: Option<u32>,
    /// JSON file of scripted player actions.
    #[arg(long)]
    script: Option<PathBuf>,
    /// Write the per-tick series to this CSV file.
    #[arg(long)]
    csv: Option<PathBuf>,
    /// Directory holding params.json, initial_town.json and systems/*.json.
    #[arg(long, default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content"))]
    content: PathBuf,
    /// Override a parameter, e.g. --set labour.wage_up=1.01 (prefix town., market., regulated. or coop. for the other files).
    #[arg(long = "set", value_name = "PATH=VALUE")]
    overrides: Vec<String>,
    /// Print only the final summary line block.
    #[arg(long)]
    quiet: bool,
}

fn read(dir: &Path, name: &str) -> Result<String, String> {
    let p = dir.join(name);
    fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))
}

/// Apply `a.b.c=value` to a JSON document. The value is parsed as JSON, else kept as a string.
fn set_path(doc: &mut serde_json::Value, path: &str, value: &str) -> Result<(), String> {
    let v: serde_json::Value =
        serde_json::from_str(value).unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
    let mut cur = doc;
    let parts: Vec<&str> = path.split('.').collect();
    for (i, key) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let next = if let Ok(idx) = key.parse::<usize>() {
            cur.get_mut(idx)
        } else {
            cur.get_mut(*key)
        };
        let Some(next) = next else { return Err(format!("no such parameter '{path}'")) };
        if last {
            *next = v;
            return Ok(());
        }
        cur = next;
    }
    Ok(())
}

fn load_params(dir: &Path, overrides: &[String]) -> Result<Params, String> {
    let mut docs: Vec<(&str, serde_json::Value)> = Vec::new();
    for (prefix, file) in [
        ("", "params.json"),
        ("town", "initial_town.json"),
        ("market", "systems/market.json"),
        ("regulated", "systems/regulated.json"),
        ("coop", "systems/coop.json"),
    ] {
        let text = read(dir, file)?;
        docs.push((prefix, serde_json::from_str(&text).map_err(|e| format!("{file}: {e}"))?));
    }
    for o in overrides {
        let (path, value) = o.split_once('=').ok_or_else(|| format!("bad --set '{o}'"))?;
        let (prefix, rest) = path.split_once('.').unwrap_or((path, ""));
        let target = docs
            .iter_mut()
            .find(|(p, _)| !p.is_empty() && *p == prefix && !rest.is_empty());
        match target {
            Some((_, doc)) => set_path(doc, rest, value)?,
            None => set_path(&mut docs[0].1, path, value)?,
        }
    }
    let ser = |i: usize| docs[i].1.to_string();
    let (params, initial_town, market, regulated, coop) = (ser(0), ser(1), ser(2), ser(3), ser(4));
    Params::from_content(&ContentJson {
        params: &params,
        initial_town: &initial_town,
        market: &market,
        regulated: &regulated,
        coop: &coop,
    })
}

fn summary_line(sim: &Sim) -> String {
    let s = &sim.state;
    let i = &s.indicators;
    format!(
        "{:>5.1} {:>12.0} {:>7.1}% {:>6.3} {:>9.0} {:>6.1}% {:>12.0} {:>13.0} {:>6.3}  {:?}",
        s.tick as f64 / 12.0,
        i.gdp_real_annual,
        i.unemployment.0 * 100.0,
        i.gini.0,
        s.nature.gross_emissions.0 * 12.0,
        100.0 * i.remaining_budget.0 / sim.params.nature.budget_b0,
        s.bank.equity.dollars(),
        i.money.dollars(),
        s.gov.legitimacy.0,
        s.system,
    )
}

fn run(args: Args) -> Result<(), String> {
    let params = load_params(&args.content, &args.overrides)?;
    let ticks = args.ticks.unwrap_or(params.run.ticks);
    let script = match &args.script {
        Some(p) => {
            Script::from_json(&fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?)?
        }
        None => Script::default(),
    };
    let mut sim = sim_engine::init(params, args.seed)?;
    let mut csv = String::new();
    csv.push_str(series::CSV_HEADER);
    csv.push('\n');
    if !args.quiet {
        println!(
            "seed {}  ticks {}  script {}",
            args.seed,
            ticks,
            if script.name.is_empty() {
                "(none)"
            } else {
                &script.name
            }
        );
        println!(
            "{:>5} {:>12} {:>8} {:>6} {:>9} {:>7} {:>12} {:>13} {:>6}  system",
            "year",
            "real GDP/yr",
            "unemp",
            "gini",
            "tCO2/yr",
            "budget",
            "bank equity",
            "money",
            "legit"
        );
        println!("{}", summary_line(&sim));
    }
    let mut exhausted: Option<u32> = None;
    let mut events_count = 0usize;
    sim_engine::run(&mut sim, ticks, &script, |s, out| {
        csv.push_str(&series::csv_row(s, &out.trace));
        csv.push('\n');
        events_count += out.events.len();
        if exhausted.is_none() && matches!(s.state.outcome, Some((_, Outcome::BudgetExhausted))) {
            exhausted = s.state.outcome.map(|(t, _)| t);
        }
        if !args.quiet && s.state.tick % 60 == 0 {
            println!("{}", summary_line(s));
        }
    })
    .map_err(|v| v.to_string())?;

    let s = &sim.state;
    let i = &s.indicators;
    println!(
        "---- summary after {} ticks ({:.1} years) ----",
        s.tick,
        s.tick as f64 / 12.0
    );
    println!("outcome: {:?}", sim_engine::final_outcome(&sim));
    if let Some(t) = exhausted {
        println!(
            "carbon budget exhausted at tick {t} (year {:.1})",
            t as f64 / 12.0
        );
    }
    println!(
        "unemployment {:.1}%  gini {:.3}  wage share {:.1}%  wellbeing {:.2}  legitimacy {:.2}",
        i.unemployment.0 * 100.0,
        i.gini.0,
        i.wage_share.0 * 100.0,
        i.avg_wellbeing.0,
        s.gov.legitimacy.0
    );
    println!(
        "cumulative emissions {:.0} t of {:.0} t budget; temperature {:.2} C; bank equity ${:.0}; money ${:.0}",
        s.nature.cumulative.0, sim.params.nature.budget_b0, i.temperature, s.bank.equity.dollars(), i.money.dollars()
    );
    println!(
        "events emitted: {events_count}; unpaid obligations clamped: ${:.0}",
        sim.clamped_total.dollars()
    );
    if let Some(p) = &args.csv {
        fs::write(p, csv).map_err(|e| format!("{}: {e}", p.display()))?;
        println!("wrote {}", p.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
