//! Tests of the decisions registered outside labour: the per-decision view-only reads
//! (`check_view_only`) and the shared helpers the labour tests use too.
//! `docs/features/decision-points.md` section 11.

use crate::class::*;
use crate::credit::*;
use crate::goods::*;
use crate::income::*;
use crate::investment::*;
use crate::lobbying::*;
use sim_types::QuantityId as Q;
use sim_types::*;

use sim_params::{ContentJson, Params};

pub(crate) fn params() -> Params {
    Params::from_content(&ContentJson {
        params: include_str!("../../../content/params.json"),
        initial_town: include_str!("../../../content/initial_town.json"),
        market: include_str!("../../../content/systems/market.json"),
        regulated: include_str!("../../../content/systems/regulated.json"),
        coop: include_str!("../../../content/systems/coop.json"),
    })
    .unwrap()
}

pub(crate) type Field<V> = (&'static str, Q, fn(&mut V));

/// View-only reads (`docs/features/decision-points.md` section 6). Three checks on one decision:
/// every view field is a quantity the spec lists as an input (the table below is built from an
/// exhaustive destructure at each call site, so a new view field fails to compile until it is
/// added); every listed input is a view field; and perturbing each field changes the choice
/// for at least one base view (so the declaration is not decoration).
pub(crate) fn check_view_only<V: Copy + std::fmt::Debug, C: PartialEq + std::fmt::Debug>(
    spec: &DecisionSpec,
    bases: &[V],
    fields: &[Field<V>],
    decide: impl Fn(&V) -> C,
) {
    for (name, q, _) in fields {
        assert!(
            spec.inputs.contains(q),
            "{:?}: view field {name} reads {} which the spec does not list",
            spec.id,
            q.as_str()
        );
    }
    for q in spec.inputs {
        assert!(
            fields.iter().any(|(_, fq, _)| fq == q),
            "{:?} lists input {} that no view field carries",
            spec.id,
            q.as_str()
        );
    }
    for (name, _, bump) in fields {
        let moved = bases.iter().any(|b| {
            let mut v = *b;
            bump(&mut v);
            decide(&v) != decide(b)
        });
        assert!(
            moved,
            "{:?}: perturbing {name} never changed the choice",
            spec.id
        );
    }
    // No hidden state: the same view gives the same choice.
    for b in bases {
        assert_eq!(decide(b), decide(b));
    }
}

#[test]
fn price_reads_only_its_declared_inputs() {
    let p = params();
    let gp = &p.goods;
    let base = |price: i64, markup, unit_cost| PriceView {
        price: Cents(price),
        markup,
        unit_cost,
    };
    let bases = [
        base(1_000, 0.2, 500.0),
        base(1_000, 0.2, 0.0),
        base(1, 0.2, 500.0),
    ];
    let PriceView {
        price: _,
        markup: _,
        unit_cost: _,
    } = bases[0];
    let fields: [Field<PriceView>; 3] = [
        ("price", Q::Price, |v| {
            v.price = Cents(v.price.0 * 2 + 5_000)
        }),
        ("markup", Q::Markup, |v| v.markup += 0.5),
        ("unit_cost", Q::UnitCost, |v| {
            v.unit_cost = v.unit_cost * 2.0 + 100.0
        }),
    ];
    check_view_only(&FIRM_PRICE, &bases, &fields, |v| decide_price(v, gp));
}

#[test]
fn green_reads_only_its_declared_inputs() {
    let p = params();
    let ip = &p.invest;
    let base = |footprint, cash, cap, tech| GreenView {
        footprint,
        carbon_price: Cents(10_000),
        revenue: Cents(1_000_000),
        emissions_cap: cap,
        green_subsidy: 0.5,
        treasury_cash: Cents(10_000_000),
        cash: Cents(cash),
        cash_reserve: Cents(100_000),
        green_tech: tech,
    };
    let bases = [
        base(1_000.0, 50_000_000, 0.0, 0.1),
        base(1_000.0, 100_000, 0.0, 0.1),
        base(0.0, 50_000_000, 0.0, 0.1),
        base(5.0, 50_000_000, 6.0, 0.1),
        base(1_000.0, 50_000_000, 0.0, 0.0),
        base(1.5, 50_000_000, 0.0, 0.1),
        base(1.5, 50_000_000, 1.0, 0.1),
    ];
    let GreenView {
        footprint: _,
        carbon_price: _,
        revenue: _,
        emissions_cap: _,
        green_subsidy: _,
        treasury_cash: _,
        cash: _,
        cash_reserve: _,
        green_tech: _,
    } = bases[0];
    let fields: [Field<GreenView>; 9] = [
        ("footprint", Q::Footprint, |v| {
            v.footprint = v.footprint * 3.0 + 1.0
        }),
        ("carbon_price", Q::CarbonPrice, |v| {
            v.carbon_price = Cents(v.carbon_price.0 * 3)
        }),
        ("revenue", Q::Revenue, |v| {
            v.revenue = Cents(v.revenue.0 * 3)
        }),
        ("emissions_cap", Q::EmissionsCap, |v| {
            v.emissions_cap = v.emissions_cap * 0.5 + 100.0
        }),
        ("green_subsidy", Q::GreenSubsidy, |v| v.green_subsidy = 0.0),
        ("treasury_cash", Q::TreasuryCash, |v| {
            v.treasury_cash = Cents(0)
        }),
        ("cash", Q::Cash, |v| v.cash = Cents(v.cash.0 / 1_000)),
        ("cash_reserve", Q::CashReserve, |v| {
            v.cash_reserve = Cents(v.cash_reserve.0 + 60_000_000)
        }),
        ("green_tech", Q::GreenTech, |v| v.green_tech = 1.0),
    ];
    check_view_only(&FIRM_GREEN, &bases, &fields, |v| decide_green(v, ip));
}

#[test]
fn capex_reads_only_its_declared_inputs() {
    let p = params();
    let ip = &p.invest;
    let base = |capital, sales, per_sale, distressed, plan| CapexView {
        capital,
        profit_trend: Cents(100_000),
        profit: Cents(200_000),
        loan_rate: 0.06,
        invest_hurdle: 0.02,
        expected_sales: sales,
        capital_per_sale: per_sale,
        distressed,
        cash: Cents(5_000_000),
        cash_reserve: Cents(1_000_000),
        green_plan: plan,
        green_cash_share: 0.5,
    };
    let bases = [
        base(100.0, 100.0, 2.0, false, false),
        base(100.0, 51.0, 2.0, false, false),
        {
            let mut v = base(100.0, 100.0, 2.0, false, false);
            v.cash = Cents(1_000_000);
            v
        },
        {
            let mut v = base(100.0, 100.0, 2.0, false, false);
            v.cash = Cents(1_500_000);
            v
        },
        {
            let mut v = base(100.0, 100.0, 2.0, false, true);
            v.cash = Cents(2_000_000);
            v
        },
        base(100.0, 100.0, 2.0, false, true),
        base(100.0, 100.0, 2.0, true, false),
        base(100.0, 10.0, 2.0, false, false),
        base(100.0, 100.0, 0.0, false, false),
    ];
    let CapexView {
        capital: _,
        profit_trend: _,
        profit: _,
        loan_rate: _,
        invest_hurdle: _,
        expected_sales: _,
        capital_per_sale: _,
        distressed: _,
        cash: _,
        cash_reserve: _,
        green_plan: _,
        green_cash_share: _,
    } = bases[0];
    let fields: [Field<CapexView>; 12] = [
        ("capital", Q::Capital, |v| v.capital *= 0.5),
        ("profit_trend", Q::ProfitTrend, |v| {
            v.profit_trend = Cents(v.profit_trend.0 * -20)
        }),
        ("profit", Q::Profit, |v| v.profit = Cents(v.profit.0 * -20)),
        ("loan_rate", Q::LoanRate, |v| v.loan_rate += 0.5),
        ("invest_hurdle", Q::InvestHurdle, |v| v.invest_hurdle += 0.5),
        ("expected_sales", Q::ExpectedSales, |v| {
            v.expected_sales *= 3.0
        }),
        ("capital_per_sale", Q::CapitalPerSale, |v| {
            v.capital_per_sale += 1.0
        }),
        ("distressed", Q::Distressed, |v| {
            v.distressed = !v.distressed
        }),
        ("cash", Q::Cash, |v| v.cash = Cents(v.cash.0 * 20)),
        ("cash_reserve", Q::CashReserve, |v| {
            v.cash_reserve = Cents(v.cash_reserve.0 * 3)
        }),
        ("green_plan", Q::GreenPlan, |v| v.green_plan = !v.green_plan),
        ("green_cash_share", Q::GreenCashShare, |v| {
            v.green_cash_share = 0.0
        }),
    ];
    check_view_only(&FIRM_CAPEX, &bases, &fields, |v| decide_capex(v, ip));
}

#[test]
fn payout_reads_only_its_declared_inputs() {
    let p = params();
    let base = |profit, cash, plan| PayoutView {
        profit: Cents(profit),
        cash: Cents(cash),
        payroll: Cents(1_000_000),
        debt_service: Cents(100_000),
        green_plan: plan,
        revenue: Cents(2_000_000),
        green_subsidy: 0.0,
        payout_ratio: 0.5,
    };
    let bases = [
        base(500_000, 5_000_000, false),
        base(500_000, 5_000_000, true),
        base(500_000, 1_500_000, true),
        base(500_000, 100_000_000, false),
        base(-500_000, 100_000_000, false),
    ];
    let PayoutView {
        profit: _,
        cash: _,
        payroll: _,
        debt_service: _,
        green_plan: _,
        revenue: _,
        green_subsidy: _,
        payout_ratio: _,
    } = bases[0];
    let fields: [Field<PayoutView>; 8] = [
        ("profit", Q::Profit, |v| {
            v.profit = Cents(v.profit.0 * 3 + 7)
        }),
        ("cash", Q::Cash, |v| v.cash = Cents(v.cash.0 / 2)),
        ("payroll", Q::Payroll, |v| {
            v.payroll = Cents(v.payroll.0 * 2)
        }),
        ("debt_service", Q::DebtService, |v| {
            v.debt_service = Cents(v.debt_service.0 * 20)
        }),
        ("green_plan", Q::GreenPlan, |v| v.green_plan = !v.green_plan),
        ("revenue", Q::Revenue, |v| {
            v.revenue = Cents(v.revenue.0 * 5)
        }),
        ("green_subsidy", Q::GreenSubsidy, |v| v.green_subsidy = 0.5),
        ("payout_ratio", Q::PayoutRatio, |v| v.payout_ratio = 0.9),
    ];
    check_view_only(&FIRM_PAYOUT, &bases, &fields, |v| {
        decide_payout(v, &p.income, &p.invest)
    });
}

#[test]
fn exit_reads_only_its_declared_inputs() {
    let p = params();
    let cp = &p.credit;
    let base = |short, months, debt| ExitView {
        payment_shortfall: short,
        distress_months: months,
        debt: Cents(debt),
    };
    let bases = [
        base(true, cp.default_after_months.saturating_sub(1), 1_000),
        base(true, 0, 1_000),
        base(false, 3, 1_000),
    ];
    let ExitView {
        payment_shortfall: _,
        distress_months: _,
        debt: _,
    } = bases[0];
    let fields: [Field<ExitView>; 3] = [
        ("payment_shortfall", Q::PaymentShortfall, |v| {
            v.payment_shortfall = !v.payment_shortfall
        }),
        ("distress_months", Q::DistressMonths, |v| {
            v.distress_months += 1
        }),
        ("debt", Q::Debt, |v| v.debt = Cents(0)),
    ];
    check_view_only(&FIRM_EXIT, &bases, &fields, |v| decide_exit(v, cp));
}

#[test]
fn found_firm_reads_only_its_declared_inputs() {
    let p = params();
    let cp = &p.class;
    let base = |ret, rate, draw| FoundView {
        sector_return: ret,
        base_rate: rate,
        random_draw: draw,
    };
    let bases = [
        base(0.3, 0.05, 0.0),
        base(0.06, 0.05, 0.5),
        base(0.3, 0.05, 0.99),
        base(0.04, 0.05, 0.0),
    ];
    let FoundView {
        sector_return: _,
        base_rate: _,
        random_draw: _,
    } = bases[0];
    let fields: [Field<FoundView>; 3] = [
        ("sector_return", Q::SectorReturn, |v| v.sector_return += 1.0),
        ("base_rate", Q::BaseRate, |v| v.base_rate += 1.0),
        ("random_draw", Q::RandomDraw, |v| {
            v.random_draw = 1.0 - v.random_draw
        }),
    ];
    check_view_only(&HOUSEHOLD_FOUND_FIRM, &bases, &fields, |v| {
        decide_found(v, cp)
    });
}

#[test]
fn spend_reads_only_its_declared_inputs() {
    let p = params();
    let gp = &p.goods;
    let base = |size, savings, disposable, wealth, rate| SpendView {
        household_size: size,
        food_price: 1_000.0,
        savings: Cents(savings),
        last_disposable: Cents(disposable),
        spending_propensity: 0.9,
        net_wealth: wealth,
        wealth_spending_rate: rate,
    };
    let bases = [
        base(2, 1_000_000, 800_000, 5.0e6, 0.005),
        base(2, 60_000, 800_000, 5.0e6, 0.005),
        base(2, 1_000_000, 10_000, 5.0e6, 0.0),
    ];
    let SpendView {
        household_size: _,
        food_price: _,
        savings: _,
        last_disposable: _,
        spending_propensity: _,
        net_wealth: _,
        wealth_spending_rate: _,
    } = bases[0];
    let fields: [Field<SpendView>; 7] = [
        ("household_size", Q::HouseholdSize, |v| {
            v.household_size += 1
        }),
        ("food_price", Q::FoodPrice, |v| v.food_price *= 2.0),
        ("savings", Q::Savings, |v| {
            v.savings = Cents(v.savings.0 / 3)
        }),
        ("last_disposable", Q::LastDisposable, |v| {
            v.last_disposable = Cents(v.last_disposable.0 * 2)
        }),
        ("spending_propensity", Q::SpendingPropensity, |v| {
            v.spending_propensity = 0.3
        }),
        ("net_wealth", Q::NetWealth, |v| v.net_wealth *= 3.0),
        ("wealth_spending_rate", Q::WealthSpendingRate, |v| {
            v.wealth_spending_rate += 0.01
        }),
    ];
    check_view_only(&HOUSEHOLD_SPEND, &bases, &fields, |v| decide_spend(v, gp));
}

#[test]
fn shop_reads_only_its_declared_inputs() {
    let p = params();
    let gp = &p.goods;
    let base = |budget, stock: &'static [f64], price: &'static [f64]| ShopView {
        budget,
        stock,
        price,
    };
    let bases = [
        base(100_000.0, &[50.0, 80.0], &[1_000.0, 1_500.0]),
        base(100_000.0, &[0.0, 0.0], &[1_000.0, 1_500.0]),
        base(100_000.0, &[1.0, 1.0], &[1_000.0, 1_500.0]),
    ];
    let ShopView {
        budget: _,
        stock: _,
        price: _,
    } = bases[0];
    let fields: [Field<ShopView<'static>>; 3] = [
        ("budget", Q::ConsumptionBudget, |v| v.budget *= 2.0),
        ("stock", Q::Inventory, |v| v.stock = &[200.0, 10.0]),
        ("price", Q::Price, |v| v.price = &[2_000.0, 900.0]),
    ];
    check_view_only(&HOUSEHOLD_SHOP, &bases, &fields, |v| decide_shop(v, gp));
}

#[test]
fn expect_reads_only_its_declared_inputs() {
    let p = params();
    let bases = [
        ExpectView {
            demand_units: 100.0,
            expected_sales: 80.0,
        },
        ExpectView {
            demand_units: 0.0,
            expected_sales: 5.0,
        },
    ];
    let ExpectView {
        demand_units: _,
        expected_sales: _,
    } = bases[0];
    let fields: [Field<ExpectView>; 2] = [
        ("demand_units", Q::DemandUnits, |v| v.demand_units += 40.0),
        ("expected_sales", Q::ExpectedSales, |v| {
            v.expected_sales += 40.0
        }),
    ];
    check_view_only(&FIRM_EXPECT, &bases, &fields, |v| {
        decide_expect(v, &p.labour)
    });
}

#[test]
fn lobby_reads_only_its_declared_inputs() {
    let p = params();
    let base = |income, savings, cap| LobbyView {
        capital_income: Cents(income),
        savings: Cents(savings),
        campaign_cap: cap,
    };
    let bases = [
        base(1_000_000, 50_000_000, 1.0),
        base(1_000_000, 100, 1.0),
        base(1_000_000, 50_000_000, 0.001),
    ];
    let LobbyView {
        capital_income: _,
        savings: _,
        campaign_cap: _,
    } = bases[0];
    let fields: [Field<LobbyView>; 3] = [
        ("capital_income", Q::CapitalIncome, |v| {
            v.capital_income = Cents(v.capital_income.0 * 2)
        }),
        ("savings", Q::Savings, |v| {
            v.savings = Cents(v.savings.0 / 100_000)
        }),
        ("campaign_cap", Q::CampaignCap, |v| v.campaign_cap = 0.0001),
    ];
    check_view_only(&HOUSEHOLD_LOBBY, &bases, &fields, |v| {
        decide_lobby(v, &p.lobby)
    });
}

fn bank_bases() -> [HeadroomView; 3] {
    let base = |equity, loans, ratio| HeadroomView {
        bank_equity: Cents(equity),
        bank_loans: Cents(loans),
        capital_ratio: ratio,
    };
    [
        base(100_000_000, 500_000_000, 0.12),
        base(100_000_000, 10_000_000, 0.12),
        base(100_000_000, 10_000_000, 0.0),
    ]
}

fn bank_fields() -> [Field<HeadroomView>; 3] {
    [
        ("bank_equity", Q::BankEquity, |v| {
            v.bank_equity = Cents(v.bank_equity.0 * 4)
        }),
        ("bank_loans", Q::BankLoans, |v| {
            v.bank_loans = Cents(v.bank_loans.0 / 4)
        }),
        ("capital_ratio", Q::CapitalRatio, |v| {
            v.capital_ratio = 0.2 + v.capital_ratio * 0.5
        }),
    ]
}

#[test]
fn headroom_reads_only_its_declared_inputs() {
    let bases = bank_bases();
    let HeadroomView {
        bank_equity: _,
        bank_loans: _,
        capital_ratio: _,
    } = bases[0];
    check_view_only(&BANK_HEADROOM, &bases, &bank_fields(), decide_headroom);
}

#[test]
fn bank_dividend_reads_only_its_declared_inputs() {
    let p = params();
    let cp = &p.credit;
    let mut bases = bank_bases();
    for b in &mut bases {
        b.bank_loans = Cents(b.bank_loans.0 / 10);
    }
    check_view_only(&BANK_DIVIDEND, &bases, &bank_fields(), |v| {
        decide_bank_dividend(v, cp)
    });
}

#[test]
fn lend_reads_only_its_declared_inputs() {
    let p = params();
    let cp = &p.credit;
    let base = |request, headroom, assets, debt| LendView {
        loan_request: Cents(request),
        credit_headroom: Cents(headroom),
        firm_assets: assets,
        debt: Cents(debt),
    };
    let bases = [
        base(100_000, 10_000_000, 10_000_000.0, 0),
        base(100_000_000, 10_000_000, 100_000_000.0, 0),
        base(100_000_000, 100_000_000, 1_000_000.0, 100_000),
    ];
    let LendView {
        loan_request: _,
        credit_headroom: _,
        firm_assets: _,
        debt: _,
    } = bases[0];
    let fields: [Field<LendView>; 4] = [
        ("loan_request", Q::LoanRequest, |v| {
            v.loan_request = Cents(v.loan_request.0 / 2)
        }),
        ("credit_headroom", Q::CreditHeadroom, |v| {
            v.credit_headroom = Cents(1_000)
        }),
        ("firm_assets", Q::FirmAssets, |v| v.firm_assets /= 10.0),
        ("debt", Q::Debt, |v| v.debt = Cents(v.debt.0 + 500_000)),
    ];
    check_view_only(&BANK_LEND, &bases, &fields, |v| decide_lend(v, cp));
}

#[test]
fn spread_reads_only_its_declared_inputs() {
    let p = params();
    let cp = &p.credit;
    let bases = [
        SpreadView {
            base_rate: 0.03,
            debt: Cents(1_000_000),
            firm_assets: 5_000_000.0,
            credit_spread_add: 0.0,
        },
        SpreadView {
            base_rate: 0.03,
            debt: Cents(0),
            firm_assets: 0.0,
            credit_spread_add: 0.01,
        },
    ];
    let SpreadView {
        base_rate: _,
        debt: _,
        firm_assets: _,
        credit_spread_add: _,
    } = bases[0];
    let fields: [Field<SpreadView>; 4] = [
        ("base_rate", Q::PolicyRate, |v| v.base_rate += 0.01),
        ("debt", Q::Debt, |v| v.debt = Cents(v.debt.0 + 1_000_000)),
        ("firm_assets", Q::FirmAssets, |v| {
            v.firm_assets += 1_000_000.0
        }),
        ("credit_spread_add", Q::CreditSpreadAdd, |v| {
            v.credit_spread_add += 0.02
        }),
    ];
    check_view_only(&BANK_SPREAD, &bases, &fields, |v| decide_spread(v, cp));
}

#[test]
fn rescue_reads_only_its_declared_inputs() {
    let p = params();
    let cp = &p.credit;
    let base = |equity, bailout| RescueView {
        bank_equity: Cents(equity),
        bank_loans: Cents(500_000_000),
        bank_deposits: Cents(600_000_000),
        bailout_policy: bailout,
    };
    let bases = [
        base(-10_000_000, true),
        base(-10_000_000, false),
        base(10_000_000, false),
    ];
    let RescueView {
        bank_equity: _,
        bank_loans: _,
        bank_deposits: _,
        bailout_policy: _,
    } = bases[0];
    let fields: [Field<RescueView>; 4] = [
        ("bank_equity", Q::BankEquity, |v| {
            v.bank_equity = Cents(-v.bank_equity.0 * 2 + 1)
        }),
        ("bank_loans", Q::BankLoans, |v| {
            v.bank_loans = Cents(v.bank_loans.0 / 2)
        }),
        ("bank_deposits", Q::BankDeposits, |v| {
            v.bank_deposits = Cents(v.bank_deposits.0 / 2)
        }),
        ("bailout_policy", Q::BailoutPolicy, |v| {
            v.bailout_policy = !v.bailout_policy
        }),
    ];
    check_view_only(&BANK_RESCUE, &bases, &fields, |v| decide_rescue(v, cp));
}
