# Causal Model v0.1

Companion to `SPEC.md` (sections 7, 11, 13.2). Essential models only. Every number marked **[P]** is a placeholder to be tuned, not calibrated data. All parameters live in `content/params.json`, never inline in code.

**Conventions:** monthly tick. Money in $ (stored as integer cents). Closed economy (no trade) for v1. A `weight` is the number of people an agent stands for.

---

## 0. Decisions (D1 to D3 confirmed)

**D1. What is a "household" agent?** 10 agents cannot be 10 individual people and also be a town. Proposal: a household is a **cohort**: a representative group with a `weight` (people it stands for). Example town of 1,000:

| Household | Role | weight [P] |
|---|---|---|
| A | Capitalist | 20 |
| B | Landlord | 20 |
| C | Manager | 150 |
| D | Worker | 350 |
| E | Worker | 350 |
| F | Dependent | 110 |

Each cohort has its own savings, debt and behaviour. Class mobility (S9) moves a small share of `weight` and its per-capita assets between cohorts. This is coarse but keeps 10 agents meaningful. *Recommended.* The alternative is a hamlet of 6 literal households, which is small and fragile.

**D2. Agent cap.** Class mobility needs a way to found new firms. Proposal: reserve **2 dormant firm slots**, so the hard cap is 12 agents, 10 active at the start.

**D3. Closed economy.** No imports or exports in v1. Simpler, and it keeps money conservation easy to test. It also means the town cannot "offshore" its emissions, which is a real limitation to note in the UI.

---

## 1. The cast and their state

| Agent | Key state |
|---|---|
| Household h | `weight`, `savings`, `debt`, `owns[]`, `skill`, `employed`, `class` (derived) |
| Firm f | `owner`, `capital K`, `cash`, `workers N`, `wage w`, `price p`, `inventory`, `intensity ι`, `debt` |
| Bank | `equity`, `loans`, `deposits`, `base_rate` |
| Government | `cash`, `debt`, `legitimacy` |
| Nature | `forest F`, `commons C`, `cumulative_emissions CE` |

Two firms at start: **Factory** (sells *goods*, high emissions) and **Farm** (sells *food*, uses forest and commons).

---

## 2. The ten systems

Each system: inputs, rule, outputs, player levers. They run in the tick order of SPEC section 11.

### S1. Labour market

```
target_workers_f  = ceil(expected_sales_units_f / output_per_worker_f)
                    expected_sales = 3-month average sales × (1 + growth_aspiration [0.3%/mo])
vacancies_f       = max(0, target_workers_f − N_f)
```
- Hire from the unemployed pool (lowest wage demanded first, ties by highest skill). Fire when `N_f > target` for 2 months in a row.
- Labour supply per cohort = `weight × labour_force_share` [P]: capitalist 0, landlord 0, manager 0.9, worker 0.9, dependent 0.6.
- **Wage adjustment** (the reserve-army effect):
```
if vacancies unfilled this month:        w_f *= 1.02
else if unemployment_rate > 8%:          w_f *= 0.995
w_f = max(w_f, min_wage)                 # lever
```
- **Skill:** employed +0.1%/mo, unemployed −0.5%/mo (long unemployment scars).

*Outputs:* employment per cohort, wage bill. *Levers:* minimum wage, benefits (S8), job guarantee (optional).

### S2. Production and emissions

```
Y_factory = A · K^0.3 · N^0.7
Y_farm    = A · K^0.3 · N^0.7 · (resource_health)^0.5
            resource_health = average of (F / Fmax) and (C / Cmax)
E_f       = Y_f · ι_f                        (tCO2 per month)
ι_f       = ι_base_f · (1 − green_tech_f) · grid_factor
grid_factor = fossil_share of the power mix (power plants placed by the player)
```
*Outputs:* units produced, emissions per firm. *Levers:* build power plants (fossil / renewable), carbon price (S8).

### S3. Goods and housing market

- **Food** (farm) is a need: `food_demand_h = weight × food_need` (inelastic).
- **Goods** (factory) are discretionary: spend `c_class × (disposable income − needs spending)`.
  `c_class` [P]: capitalist 0.40, landlord 0.45, manager 0.70, worker 0.95, dependent 1.00. (Rich households save more, so inequality can depress demand.)
- **Price:**
```
p_f *= 1 + clamp(0.10 · (demand − supply) / supply, −5%, +5%)
sales_f = min(demand, production + inventory)
unsold stock goes to inventory (holding cost 2%/mo)
```
- **Housing:** each person needs a place. Private rent rises 1%/mo when occupancy > 95%, falls 0.5%/mo otherwise. Public housing charges a fixed lower rent to the government. Unhoused people lose wellbeing (S10).

*Outputs:* sales, prices, rents, unmet need. *Levers:* build public housing, rent controls (optional).

### S4. Income distribution (this is where the economic *system* plugs in)

```
profit_f   = sales·p − wages − input_costs − interest − carbon_cost − tax − upkeep
carbon_cost = effective_carbon_price · E_f          (S8, after lobbying S9b)
dividends  = payout_ratio [0.6] · max(0, profit_f)
retained   = profit_f − dividends                   (stays as firm cash)
```
**Who receives `dividends` depends on the system:**

| System | dividends go to |
|---|---|
| Market / Regulated capitalism | the firm's owner household |
| Worker cooperative | employed households, pro-rata to headcount |
| State ownership | the government |

Household disposable income:
```
disposable = wages + dividends + rent_received + interest_received + benefits
           − income_tax − rent_paid − loan_payments
```
Track each term separately per household. This is what the inspector's income breakdown and the class function use.

### S5. Credit and money

```
bank can lend while:  loans ≤ equity / κ         (κ = 0.10 capital ratio [P])
loan rate  i = base_rate + spread,   spread = 0.5% + 2% · leverage_of_borrower
```
- **Lending creates money:** new loan → borrower's deposit +L and bank loan +L. `money_in_circulation` = total deposits.
- **Repayment** (amortising over 60 months for firms): interest goes to bank profit (to the bank's owner), principal repayment *destroys* money.
- **Default:** borrower cash < 0 for 3 months → write-off. Bank equity falls by the unpaid principal; the firm closes; the owner loses their equity.
- **Bank failure:** if equity < 0, savers lose a share of deposits unless the government bails out (lever, costs legitimacy). Large shock by design.
- **Deposit rate** `r_d = base_rate − 1%`.

*Invariant:* money is created and destroyed only here (and via government borrowing from the bank). A conservation check should pass every tick.
*Levers:* base interest rate, bank capital ratio, bailout policy.

### S6. Investment and technology (the growth engine)

```
expected_return_f = 12 · profit_f / K_f
invest if expected_return_f > i + hurdle [5%]
I_f = min(0.7 · retained_cash + available_loan, max_invest)
K_f ← K_f + I_f − δ·K_f             (δ = 0.4%/mo depreciation)
```
- **Green investment:** reduces `ι_f` by a step (5%) per spend of 10% of annual revenue. A firm chooses it when
  `annual_carbon_cost × payback_years > cost`, or when regulation forces it.
- **Growth imperative (emergent):** debt service plus the hurdle rate means a firm that does not earn more than its interest and hurdle gets starved of credit and may fail. Nothing "forces growth" explicitly; the debt does.

*Levers:* green subsidy (share of green investment paid by government), carbon price, emissions cap per firm.

### S7. Natural capital and the carbon budget

```
Forest:   dF = g · F · (1 − F/Fmax) − harvest_F − cleared_by_building
Sink:     S  = σ · F                               (tCO2 absorbed per month)
Commons:  dC = g_c · C · (1 − C/Cmax) − harvest_C
Net emissions this month:  ΔCE = Σ E_f + Σ E_h − S
Cumulative:  CE ← CE + ΔCE
Household emissions: E_h = weight · footprint_pc,  footprint_pc = f0 + f1 · consumption_pc · grid_factor
Budget remaining:  B_rem = B0 − CE
Displayed warming: T = T0 + (1.5 − T0) · CE / B0       (T0 ≈ 1.3 [P]; hits 1.5 exactly when the budget is gone)
```
- **Tragedy of the commons:** maximum sustainable yield is `g_c · Cmax / 4`. Harvest above that and the stock collapses (the logistic curve turns negative below a threshold).
- **Access rule for the commons (player-set):** *open* (anyone, depletes), *enclosed* (one owner, who harvests for profit, often more), *community-managed* (cap set to sustainable yield, needs coordination cost).
- **Climate feedback (v1, simple):** `farm A *= (1 − 0.3 · (CE/B0)²)`; wellbeing environment term falls (S10). This makes overshoot hurt even before the game ends.
- **Calibration target:** `B0` is set so a default market-capitalism run with no policy exhausts the budget around **year 28 (±5)**.

*Levers:* conservation areas, planting forest, commons access rule, power mix.

### S8. Government

```
Revenue   = τ_w·wages + τ_p·(dividends+profit) + τ_wealth·(wealth above threshold) + P_c_eff·ΣE
Spending  = benefits·unemployed + upkeep(infra) + build costs + green_subsidy + UBI
Deficit   → government borrows from bank at base_rate (counts toward debt)
Legitimacy L ∈ [0,1]:  L += 0.01·(avg_wellbeing − 0.5)
                         − 0.02·max(0, unemployment − 0.10)
                         − 0.02·max(0, Gini − 0.45)
                         − 0.05 if a bank bailout happened
                         − 0.02·max(0, debt/GDP − 1)
```
Legitimacy at 0 is a loss condition (town collapse).
*Levers:* all tax rates, carbon price, benefits, UBI, subsidies, public spending, bailouts.

### S9. Class and mobility

**Class is computed, never assigned** (6-month averages):
```
capital_share = (dividends + rent + interest) / total_income
if owns firm/bank/housing and capital_share > 0.5 :  capitalist   (rentier if mostly rent)
elif employed and skill > 0.7 and wage_pc > skilled_threshold :  manager
elif employed :                                      worker
else :                                               dependent
```
**Mobility (v1):**
- **Up:** each month, a worker/manager cohort with `savings_pc > startup_cost + buffer` transfers a small share of weight (about 0.5% × (sector_profit_rate − i) per month [P]) into an *owner* cohort, creating a firm in a dormant slot (D2).
- **Down:** firm failure wipes the owner's equity. If net wealth is negative, the owner's share of weight transfers to the worker or dependent cohort. Debt follows the person.
- **No inheritance in v1.**
- **Co-op system:** employees of a co-op firm count as owners of it, so their `capital_share` rises. This is the mechanism by which the system switch changes class structure.

### S9b. Political power and lobbying (the capture loop)

```
lobby_spend_h   = 2% of capital income of capitalist/landlord cohorts   (limited by campaign-finance cap, a lever)
lobby_capital   = min(λ_max [0.6], k · Σ lobby_spend / monthly_GDP)
lobby_labour    = union_density · labour_share · u_k                    (organised workers push back)
net_lobby       = max(0, lobby_capital − lobby_labour)
effective_policy = nominal_policy · (1 − net_lobby)
```
This applies to the **carbon price, wealth tax, profit tax and emissions caps**. The player sets 50 and gets, say, 31, and the UI shows the gap and who caused it. Countermeasures: campaign-finance cap, higher union density (rises when unemployment is low and in co-ops), wealth tax (reduces the lobby budget).

### S10. Wellbeing and indicators

```
wellbeing_h = 0.30·min(1, consumption_pc / needs)
            + 0.20·employment_term      (employed share; 1 for non-labour cohorts)
            + 0.20·housed_share
            + 0.15·environment          (forest health × (1 − local pollution))
            + 0.15·security             (savings ≥ 3 months of needs)
GDP (annualised)  = 12 · [ Σ_f (sales·p − input costs) + government service spending ]
Gini              = over person-weighted wealth
GPI               = GDP · (1 − Gini) − SCC · ΣE − depletion_cost         (SCC ≈ $200/t [P])
money_in_circulation = Σ deposits
```

---

## 3. How the economic systems change the rules

A system is a config that overrides parameters and some rule choices.

| | Market capitalism | Regulated capitalism | Worker cooperatives |
|---|---|---|---|
| S4 dividends | owner | owner | employees |
| S1 wages | market (reserve-army) | market + higher floor | share of surplus, floored |
| S6 investment hurdle | 5% | 5%, green subsidy on | 2%, higher retained share for green tech |
| S5 credit | normal | normal | spread +1% (banks distrust co-ops) |
| S9b `λ_max` | 0.6 | 0.4, campaign cap available | 0.2 |
| S7 default commons rule | open or enclosed | community cap allowed | community-managed |
| Transition | n/a | 12 months, −5% output, resistance from capitalists | 24 months, −10% output, high resistance, workers need training |

---

## 4. The loops that produce the game

| Loop | Type | Story |
|---|---|---|
| Profit → investment → output → profit | **Reinforcing** | Growth. Also drives emissions up (S2). |
| Wealth → lobbying → weaker policy → profit → wealth | **Reinforcing** | Capture. The carbon price you set erodes. |
| Debt → interest → need for profit → more investment | **Reinforcing** | Growth imperative. |
| Capital income grows, wage share falls → demand falls → layoffs | **Balancing (crisis)** | Inequality starves demand. |
| Unemployment → lower wages → higher profit → hiring | **Balancing** | Reserve army. Fixes jobs, deepens inequality. |
| Scarcity → price rises → demand falls | **Balancing** | Normal market. |
| Over-harvest → commons collapses → farm output falls | **Balancing (late)** | Tragedy of the commons. |
| Emissions → budget used → climate damage → lower farm output and wellbeing | **Reinforcing** | Overshoot starts hurting before game over. |
| Carbon price → green investment → lower ι → fewer emissions | **Balancing** | The main lever, weakened by lobbying. |

---

## 5. Targets and tests

**Design targets (v0.1; superseded by section 9 below, which reflects the turn-based game):**
- Default market run, no policy: budget gone around year 28 (±5), unemployment between 6% and 12%, Gini rises from about 0.40 to 0.55, GDP grows about 2% a year.
- A competent player can stay inside the budget in regulated capitalism, at the cost of lower GDP growth, and fighting lobbying.
- Cooperatives give lower Gini and slower growth, and are not a free pass on emissions.

**Invariant tests (must hold every tick):**
1. Total money = total deposits; money changes only through S5.
2. No stock (F, C, cash of solvent agents, population) is negative.
3. Same seed gives identical state after 480 ticks.
4. Sum of cohort weights is constant.
5. Class labels are consistent with the S9 function.

**Sign tests** (derived from the causal map):
- Raise carbon price (no lobbying) → emissions fall within 12 months.
- Raise minimum wage → wage share up; unemployment up or flat.
- Raise base rate → credit and investment fall.
- Increase commons harvest above sustainable yield → commons stock eventually falls.
- Raise lobbying cap → effective carbon price falls.

---

## 6. Deliberately left out of v1

Trade and migration, inflation beyond relative prices, stock markets, multiple firms per sector, demographics (births and deaths), technology spillovers, taxes on goods, random events. Each can be added as a new system later without changing the others.

---

## 7. S0: turns, political capital and policy delay

The game is turn-based (SPEC 4.1). One turn = 1 quarter = 3 ticks. This system runs once per turn, before the first tick.

```
submit_turn(commands):
  for each command c in order:
      cost = lever_base_cost(c) · size(c) · (1 + net_lobby_against(c))      # PC; net_lobby from S9b
      reject c if cost > PC  or money_cost(c) > available funds
      PC −= cost;  push c onto pending queue with effective_turn = current_turn + lead_time(c)
  activate every pending item whose effective_turn == current_turn
  run 3 ticks
  PC = min(PC_cap, PC + pc_base · legitimacy)
```
- Policy values the model uses each tick are the **active** values. Pending items change nothing until activated.
- Agents may respond to *announced* policies (firms read the pending carbon price when deciding investment), using a discount of 0.5 on a policy that is still pending.
- Lobbying (S9b) is evaluated when a command activates: `effective = nominal · (1 − net_lobby)`. The turn summary reports nominal vs effective.
- Cancelling a pending command refunds 50% of its PC.
- Parameters (placeholders in `content/`): `pc_start 60`, `pc_cap 100`, `pc_base 8`, `lead_time` per lever, `lever_base_cost` per lever.

---

## 8. Revision v0.2: the demand side (replaces parts of S1, S3, S5, S6, S8)

**Problem found in the first build:** rich households saved most of their income, nothing recycled those savings into spending, and the economy slid to wherever government and investment spending happened to land. Parameter searching could not fix it.

**Fix: make every payment someone's income, and send savings round the circle.** Rules:

1. **Investment is demand.** Firm investment buys capital goods from the Factory. That spending is the Factory's revenue and pays its workers. So saving, if lent out, becomes someone else's income.
2. **Savings recycle through the bank.** Household saving builds deposits. The bank lends against deposits and equity (S5). Firms borrow to invest. This is how the savings of the rich re-enter the circle.
3. **Accelerator investment.** `I_f = replacement (δ·K) + a · (desired_K − K)`, with `desired_K = expected_sales / capital_productivity` and `a` smoothed (about 5% of the gap per month). Investment is driven by expected demand and capacity utilisation, not only by profit. Profit and the interest rate gate it (S6 hurdle).
4. **Wealth effect on consumption.** Capitalist and landlord cohorts also consume `c_w ≈ 0.2% per month` of net wealth, so wealth cannot be hoarded without limit. Their `propensity_to_consume` applies to income as before.
5. **Government spending is a player policy, not a reaction rule.** Real purchases `G` are set by a lever (queued like any policy). Automatic stabilisers remain: benefits per unemployed person and income-linked taxes. The government has **no hidden rule** reacting to its own revenue or debt (that caused spirals). Debt consequences work through bank lending limits and legitimacy (S8).
6. **Wages and prices.** Wage growth follows employment: `w ← w · (1 + k · (u_ref − u))`, with `k` small and the monthly change capped at ±1%. Prices follow unit labour cost plus a markup: `p → (1 + μ) · ULC`, adjusting 5% of the gap per month. Inventories absorb short-run mismatches. Remove the older price and wage rules where they conflict.
7. **Production follows expected sales.** `expected_sales` is an exponential average (λ ≈ 0.3). Employment follows output. Smoothing is what stops 12-month boom-bust cycles. Some cycle is allowed; violent limit cycles are not.
8. **Job guarantee (optional player lever).** The government can hire up to X% of the unemployed at the minimum wage. It costs PC, is off by default, and is a *policy*, not a hidden stabiliser. (This is the legitimate version of the shortcut: the player chooses it and pays for it.)
9. **Household credit** is off in v1 (a flag `household_credit` exists in params). Revisit if demand still collapses.

**Conservation:** with these rules the sector balances should sum to zero each tick: household saving = firm investment financed + government deficit + change in bank-held assets. Add this identity to `sim-checks`.

---

## 9. Revised targets and baseline runs

Because the player steers every turn, the model does not need to be stable on its own. It needs to be **steerable**: sensible play works, neglect fails, and nothing crashes or produces `NaN`.

**Three baseline runs (each a scripted turn plan, 160 turns, fixed seed):**

| Run | Script | Required behaviour |
|---|---|---|
| **A. No action** | Player queues nothing | Does not crash or break invariants. Carbon budget is exhausted between roughly turns 90 and 140 (year 22 to 35). Inequality rises. Unemployment cycles within roughly 4% to 15%. |
| **B. Sensible play** | Ramp the carbon price, green subsidy, public housing, benefits, some commons protection, modest redistribution, all within the PC budget | Survives all 160 turns with budget intact, legitimacy above 0.3, unemployment below 15% most of the time. Proves the game is winnable. |
| **C. Neglect and extraction** | Build fossil plants, enclose the commons, cut taxes and benefits | Fails (budget gone or collapse) by roughly year 30 or earlier. |

**Plus (still required):** the money-conservation identity and sector balances, determinism, non-negativity, finite numbers, the sign tests in section 5, and a golden snapshot of run A.

**Soft targets for run A (informal):** GDP growth around 1% to 2% a year when not in recession; Gini starting near 0.40 and rising to 0.50 to 0.60. If these cannot be reached without implausible rules, say so rather than forcing it.

**Winnable but not trivial:** at least one plausible strategy (for example, only a carbon tax and nothing else) should *fail*, so the player has to combine tools.
