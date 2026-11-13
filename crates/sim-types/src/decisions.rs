//! Decision points (`docs/features/decision-points.md`): data only. A decision is a pure function in
//! `sim-model` over a read-only view; its `DecisionSpec` declares what it reads, what it writes and
//! which parameters it uses, so that "who decides what" and a future causality view can be generated
//! from code. Labels, units and prose live in `content/decisions.json` (AGENTS.md rule 9).

use crate::enums::SystemId;

/// Declares a closed enum with a snake_case wire name per variant, `ALL` (declaration order) and
/// `as_str`. One macro for three enums: the variant list is written once, so the list, the
/// wire name and `ALL` cannot disagree.
macro_rules! named_enum {
    ($(#[$m:meta])* pub enum $name:ident { $($(#[$vm:meta])* $v:ident => $s:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name { $($(#[$vm])* $v),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$v),+];
            /// The wire name (snake_case).
            pub const fn as_str(self) -> &'static str {
                match self { $($name::$v => $s),+ }
            }
        }
    };
}

named_enum! {
    /// Who makes a decision. Only `Firm` has registered decisions so far (stage 1, the labour pilot).
    pub enum OwnerKind {
        Firm => "firm",
        Household => "household",
        Bank => "bank",
        Government => "government",
        Estate => "estate",
        Nature => "nature",
    }
}

named_enum! {
    /// Every decision registered in `sim-model::decisions::DECISIONS`.
    pub enum DecisionId {
        FirmHire => "firm_hire",
        FirmFire => "firm_fire",
        FirmWage => "firm_wage",
    }
}

impl DecisionId {
    /// The owner is derived from the id (an exhaustive match), never stored, so the two cannot disagree.
    pub const fn owner(self) -> OwnerKind {
        match self {
            DecisionId::FirmHire | DecisionId::FirmFire | DecisionId::FirmWage => OwnerKind::Firm,
        }
    }
}

named_enum! {
    /// The vocabulary of state variables and flows that decisions read or write.
    pub enum QuantityId {
        ExpectedSales => "expected_sales",
        Inventory => "inventory",
        InventoryCover => "inventory_cover",
        Workers => "workers",
        SkilledWorkers => "skilled_workers",
        EffectiveLabour => "effective_labour",
        OutputCapacity => "output_capacity",
        WantedWorkers => "wanted_workers",
        Vacancies => "vacancies",
        OverTargetMonths => "over_target_months",
        Unemployment => "unemployment",
        Wage => "wage",
        MinWage => "min_wage",
        SectorTfpGrowth => "sector_tfp_growth",
    }
}

/// The declaration of one decision. `inputs` and `outputs` are the edges of the causal graph;
/// `params` are dotted paths into `content/params.json`; `rule` is the id of the default rule
/// (its words are in `content/decisions.json`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionSpec {
    pub id: DecisionId,
    pub system: SystemId,
    pub inputs: &'static [QuantityId],
    pub outputs: &'static [QuantityId],
    pub params: &'static [&'static str],
    pub rule: &'static str,
}

impl DecisionSpec {
    pub const fn owner(&self) -> OwnerKind {
        self.id.owner()
    }
}
