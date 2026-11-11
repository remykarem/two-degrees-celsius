//! The entity registry (platform cleanup phase 2, contract v2.1): what kinds of actor, facility
//! and natural stock the town has, authored in `content/entities.json` and served in
//! `Catalog.entities`. It holds identity, labels, roles, tiles and footprint, the default owner
//! edge, the scene model ids and the inspector sections. It holds no numbers (those stay in
//! `params.json`), and instances stay in `initial_town.json`.
//!
//! Closed enums (`EntityAxis`, `EntityCategory`, `Role`, `OwnerEdge`, `InspectorSection`) keep the
//! registry checkable: an unknown word fails at load, and the generated TypeScript has the union.
use crate::error::{code, ApiError, ApiResult};
use crate::industry::InfraKindKey;
use crate::keys::TileKindKey;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The file, embedded at build time.
pub const ENTITIES: &str = include_str!("../../../content/entities.json");

/// What fundamentally is it (taxonomy section 3). Institutions, flows and processes have no entry.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityAxis {
    Actor,
    Facility,
    Stock,
}

/// Roll-up key of an entity kind, for grouping lists.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityCategory {
    People,
    Firms,
    Finance,
    Government,
    Housing,
    Energy,
    Transport,
    Safety,
    Estate,
    Nature,
}

/// What an entity can do in the model (taxonomy section 3). Data, not a Rust trait.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Located,
    Owned,
    Emitter,
    Sink,
    Payer,
    Payee,
    Employer,
    IncomeSource,
    EnergyProducer,
    EnergyConsumer,
    HazardExposed,
    HazardMitigator,
    Buildable,
    LeverTarget,
    Labelled,
}

/// Who owns it by default. `varies` means each instance says (firms, homes).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerEdge {
    None,
    Household,
    Public,
    State,
    Workers,
    Bank,
    Varies,
}

/// A block of inspector rows. The web has one builder per id; an entity lists the ones it wants.
/// `emissions` is generic (any `emitter` on a tile); the rest read the snapshot of their kind.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorSection {
    Emissions,
    PersonWork,
    PersonCommute,
    PersonLife,
    HouseholdHome,
    HouseholdWealth,
    HouseholdMoney,
    HouseholdLife,
    EstateOwner,
    Pilot,
    FirmFirm,
    FirmOutput,
    FirmBooks,
    BankBalance,
    GovernmentState,
    GovernmentPolitics,
    HousingTown,
    FireCover,
    PlantThis,
    EnergyTown,
    Forest,
    Commons,
}

/// A tile offset from the instance's anchor tile.
pub type Offset = [i32; 2];

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityKind {
    pub id: String,
    pub kind: EntityAxis,
    pub category: EntityCategory,
    pub label: String,
    pub plural: String,
    pub roles: Vec<Role>,
    /// Map tile kinds this entity stands on (none for a person or a jet).
    #[serde(default)]
    pub tile_kinds: Vec<TileKindKey>,
    /// Tiles of one instance relative to its anchor; non-empty for every `located` entity.
    #[serde(default)]
    pub footprint: Vec<Offset>,
    /// The buildable kind of the wire (`InfraKind`), for the entities the player can build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub infra_kind: Option<InfraKindKey>,
    /// An entity that specialises another (the pilot is a person); it must name an existing id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub variant_of: Option<String>,
    /// A family several kinds share (`power_plant` for fossil and renewable plants).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    pub owner: OwnerEdge,
    /// Free labelled attributes (`fuel: fossil`); words only, no numbers.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
    /// Scene model ids by variant key (`default` is required when any are listed); several ids are
    /// alternatives the scene picks between by position.
    #[serde(default)]
    pub scene_model: BTreeMap<String, Vec<String>>,
    /// Wording by state (`burning`, `closed`); `{name}` is filled by the web.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    /// Inspector sections, in order.
    #[serde(default)]
    pub inspector: Vec<InspectorSection>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntitiesFile {
    entities: Vec<EntityKind>,
}

fn bad(msg: String) -> ApiError {
    ApiError::new(code::BAD_METADATA, format!("content/entities.json: {msg}"))
}

/// Every tile kind the wire has (checked complete by `check`).
pub const ALL_TILE_KINDS: [TileKindKey; 14] = [
    TileKindKey::Empty,
    TileKindKey::Home,
    TileKindKey::PublicHousing,
    TileKindKey::Firm,
    TileKindKey::Bank,
    TileKindKey::TownHall,
    TileKindKey::FossilPlant,
    TileKindKey::RenewablePlant,
    TileKindKey::Road,
    TileKindKey::Forest,
    TileKindKey::Commons,
    TileKindKey::FireStation,
    TileKindKey::EstateGrounds,
    TileKindKey::Runway,
];
const ALL_INFRA: [InfraKindKey; 5] = [
    InfraKindKey::Road,
    InfraKindKey::PublicHousing,
    InfraKindKey::FossilPlant,
    InfraKindKey::RenewablePlant,
    InfraKindKey::FireStation,
];

/// Validate a registry: unique ids, known variants, a tile kind claimed once and every wire tile
/// kind and buildable kind claimed, footprints and roles consistent.
pub fn check(list: &[EntityKind]) -> ApiResult<()> {
    let mut ids = BTreeSet::new();
    for e in list {
        if !ids.insert(e.id.as_str()) {
            return Err(bad(format!("duplicate id {}", e.id)));
        }
        if e.id.is_empty() || e.label.trim().is_empty() || e.plural.trim().is_empty() {
            return Err(bad(format!("{} needs an id, a label and a plural", e.id)));
        }
        let roles: BTreeSet<Role> = e.roles.iter().copied().collect();
        if roles.len() != e.roles.len() {
            return Err(bad(format!("{} lists a role twice", e.id)));
        }
        if !roles.contains(&Role::Labelled) {
            return Err(bad(format!("{} must have the labelled role", e.id)));
        }
        if roles.contains(&Role::Located) != !e.footprint.is_empty() {
            return Err(bad(format!(
                "{}: the located role and a footprint go together",
                e.id
            )));
        }
        if !e.tile_kinds.is_empty() && !roles.contains(&Role::Located) {
            return Err(bad(format!("{} has tile kinds but is not located", e.id)));
        }
        if roles.contains(&Role::Buildable) != e.infra_kind.is_some() {
            return Err(bad(format!(
                "{}: the buildable role and an infra kind go together",
                e.id
            )));
        }
        if e.inspector.contains(&InspectorSection::Emissions) && !roles.contains(&Role::Emitter) {
            return Err(bad(format!(
                "{}: the emissions section needs the emitter role",
                e.id
            )));
        }
        let mut sections = BTreeSet::new();
        if e.inspector.iter().any(|s| !sections.insert(*s as u8)) {
            return Err(bad(format!("{} lists a section twice", e.id)));
        }
        if !e.scene_model.is_empty() && !e.scene_model.contains_key("default") {
            return Err(bad(format!("{}: scene_model needs a default", e.id)));
        }
        if e.scene_model
            .values()
            .any(|v| v.is_empty() || v.iter().any(|m| m.is_empty()))
        {
            return Err(bad(format!("{}: empty scene model id", e.id)));
        }
    }
    for e in list {
        if let Some(v) = &e.variant_of {
            if v == &e.id || !ids.contains(v.as_str()) {
                return Err(bad(format!(
                    "{}: variant_of {v} is not another entity",
                    e.id
                )));
            }
        }
    }
    for k in ALL_TILE_KINDS {
        let n = list.iter().filter(|e| e.tile_kinds.contains(&k)).count();
        if n != 1 {
            return Err(bad(format!(
                "tile kind {k:?} is claimed by {n} entities, not 1"
            )));
        }
    }
    for k in ALL_INFRA {
        let n = list.iter().filter(|e| e.infra_kind == Some(k)).count();
        if n != 1 {
            return Err(bad(format!(
                "infra kind {k:?} is claimed by {n} entities, not 1"
            )));
        }
    }
    Ok(())
}

/// Parse and validate the embedded registry.
pub fn load() -> ApiResult<Vec<EntityKind>> {
    parse(ENTITIES)
}

/// Parse and validate registry text (the tests feed variants of the real file).
pub fn parse(json: &str) -> ApiResult<Vec<EntityKind>> {
    let f: EntitiesFile = serde_json::from_str(json).map_err(|e| bad(e.to_string()))?;
    check(&f.entities)?;
    Ok(f.entities)
}
