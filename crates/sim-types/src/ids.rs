use serde::{Deserialize, Serialize};

/// Index of a household cohort in `State::households`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HouseholdId(pub u32);

/// Index of a firm slot in `State::firms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FirmId(pub u32);

impl HouseholdId {
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}
impl FirmId {
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}
