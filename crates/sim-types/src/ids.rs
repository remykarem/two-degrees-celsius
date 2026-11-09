use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u32);
        impl $name {
            pub fn idx(self) -> usize {
                self.0 as usize
            }
        }
    };
}

id_type!(
    /// Index of a household in `State::households`.
    HouseholdId
);
id_type!(
    /// Index of a person in `State::persons`.
    PersonId
);
id_type!(
    /// Index of a firm slot in `State::firms`.
    FirmId
);
id_type!(
    /// Stable id of a home (not an index: homes can be removed).
    HomeId
);
id_type!(
    /// Stable id of a power plant (not an index: plants can be removed).
    PlantId
);

/// A tile of the 12 x 12 map. `x` is the column, `y` the row, both from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Tile {
    pub x: u8,
    pub y: u8,
}

impl Tile {
    pub fn new(x: u8, y: u8) -> Tile {
        Tile { x, y }
    }
    /// Distance in whole tiles along the grid (no diagonals).
    pub fn manhattan(self, o: Tile) -> u32 {
        (self.x as i32 - o.x as i32).unsigned_abs() + (self.y as i32 - o.y as i32).unsigned_abs()
    }
    /// Largest of the horizontal and vertical distances (a square neighbourhood).
    pub fn chebyshev(self, o: Tile) -> u32 {
        (self.x as i32 - o.x as i32)
            .unsigned_abs()
            .max((self.y as i32 - o.y as i32).unsigned_abs())
    }
}
