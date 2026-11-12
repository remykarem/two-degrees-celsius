//! Emission and income sources as data (taxonomy steps 1 and 2, contract v2.0).
//!
//! The wire used to name each source as a field (`firms`, `households`, `plants`, `aviation`;
//! `wages`, `dividends`, ...). Now a line is a list of `{id, label, category, amount}`. Ids are a
//! closed set the model knows how to total (`EMISSION_IDS`, `INCOME_IDS`); labels, categories and
//! visibility live in `content/ui.json` (`emission_sources`, `income_sources`) and are checked at
//! load. A new emitter or income type adds a model arm plus one content entry, not a wire field.
use crate::error::{code, ApiError, ApiResult};
use serde::{Deserialize, Serialize};

/// Source ids the model can total, in the order of the wire's own sums.
pub const EMISSION_IDS: [&str; 4] = ["firms", "households", "plants", "aviation"];
/// Income ids the wire can report.
pub const INCOME_IDS: [&str; 6] = [
    "wages",
    "domestic_wages",
    "dividends",
    "rent",
    "interest",
    "benefits",
];

/// Roll-up key of an emission source. `waste` is reserved for a future emitter (taxonomy 5.1).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmissionCategory {
    Industry,
    Homes,
    Energy,
    Aviation,
    Waste,
}

/// Roll-up key of an income source.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncomeCategory {
    Labour,
    Capital,
    Transfer,
}

/// Where an income source is reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncomeView {
    /// `HouseholdView.income.sources`.
    Household,
    /// `FinanceFlows.householdIncome`.
    Finance,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EmissionSourceDef {
    pub id: String,
    pub label: String,
    pub category: EmissionCategory,
    /// A source that is zero is left out of a line unless this is true.
    #[serde(default)]
    pub show_when_zero: bool,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct IncomeSourceDef {
    pub id: String,
    pub label: String,
    pub category: IncomeCategory,
    /// A source that is part of another (domestic wages are part of wages): not added to totals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub part_of: Option<String>,
    /// Where the source is reported (a content detail, not on the wire).
    #[serde(skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    pub views: Vec<IncomeView>,
}

/// One source of a period's emissions, in tonnes. Lines sum to `grossTotal` (the sink is separate).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmissionSourceTotal {
    pub id: String,
    pub label: String,
    pub category: EmissionCategory,
    pub tonnes: f64,
}

/// One source of income, in dollars. Entries with `partOf` are already inside the named entry.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomeSource {
    pub id: String,
    pub label: String,
    pub category: IncomeCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub part_of: Option<String>,
    pub amount: f64,
}

/// The validated definitions from `content/ui.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct Sources {
    pub emission: Vec<EmissionSourceDef>,
    pub income: Vec<IncomeSourceDef>,
    /// Town v8: the net worth statement lines and holders.
    pub net_worth: crate::networth::NetWorthDefs,
    /// Markets view: the market cards, mechanisms and cause words.
    pub markets: crate::markets::MarketsContent,
    /// Farm-from-nature: the sustainable-yield view's words.
    pub yields: crate::yields::YieldContent,
}

fn bad(msg: String) -> ApiError {
    ApiError::new(code::BAD_METADATA, format!("content/ui.json: {msg}"))
}

impl Sources {
    /// Check the content against the closed id sets: every id once, known, parts after their parent.
    pub fn new(
        emission: Vec<EmissionSourceDef>,
        income: Vec<IncomeSourceDef>,
        net_worth: crate::networth::NetWorthDefs,
        markets: crate::markets::MarketsContent,
        yields: crate::yields::YieldContent,
    ) -> ApiResult<Self> {
        for id in EMISSION_IDS {
            if emission.iter().filter(|d| d.id == id).count() != 1 {
                return Err(bad(format!("emission_sources must list {id} exactly once")));
            }
        }
        if let Some(d) = emission
            .iter()
            .find(|d| !EMISSION_IDS.contains(&d.id.as_str()))
        {
            return Err(bad(format!("unknown emission source {}", d.id)));
        }
        for id in INCOME_IDS {
            if income.iter().filter(|d| d.id == id).count() != 1 {
                return Err(bad(format!("income_sources must list {id} exactly once")));
            }
        }
        for (i, d) in income.iter().enumerate() {
            if !INCOME_IDS.contains(&d.id.as_str()) {
                return Err(bad(format!("unknown income source {}", d.id)));
            }
            if let Some(p) = &d.part_of {
                if !income[..i].iter().any(|x| &x.id == p) {
                    return Err(bad(format!(
                        "{} is part of {p}, which must come first",
                        d.id
                    )));
                }
            }
        }
        Ok(Self {
            yields,
            emission,
            income,
            net_worth,
            markets: markets.validated()?,
        })
    }

    /// Totals for the sources of an emissions line (`value(id)` gives tonnes; ids are validated).
    pub fn emission_line(&self, value: impl Fn(&str) -> f64) -> Vec<EmissionSourceTotal> {
        self.emission
            .iter()
            .filter_map(|d| {
                let tonnes = value(&d.id);
                (tonnes != 0.0 || d.show_when_zero).then(|| EmissionSourceTotal {
                    id: d.id.clone(),
                    label: d.label.clone(),
                    category: d.category,
                    tonnes,
                })
            })
            .collect()
    }

    /// The income list of one view, in content order.
    pub fn income_list(&self, view: IncomeView, amount: impl Fn(&str) -> f64) -> Vec<IncomeSource> {
        self.income
            .iter()
            .filter(|d| d.views.contains(&view))
            .map(|d| IncomeSource {
                id: d.id.clone(),
                label: d.label.clone(),
                category: d.category,
                part_of: d.part_of.clone(),
                amount: amount(&d.id),
            })
            .collect()
    }
}
