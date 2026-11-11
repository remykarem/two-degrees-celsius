//! The entity registry (`content/entities.json`, contract v2.1): loads and validates, covers
//! every wire tile kind, matches the initial map, and a made-up entity added only in content
//! shows up in the catalog. Bad content is refused at load.
use serde_json::{json, Value};
use sim_api::entities::{self, EntityKind, ENTITIES};
use sim_api::keys::TileKindKey;
use sim_api::Api;

fn file() -> Value {
    serde_json::from_str(ENTITIES).unwrap()
}
fn parse(v: &Value) -> Result<Vec<EntityKind>, String> {
    entities::parse(&v.to_string()).map_err(|e| e.message)
}
fn list(v: &mut Value) -> &mut Vec<Value> {
    v["entities"].as_array_mut().unwrap()
}

#[test]
fn the_registry_loads_and_is_served_in_the_catalog() {
    let api = Api::init(7).unwrap();
    let c = api.catalog().unwrap();
    assert_eq!(c.entities, entities::load().unwrap());
    for id in [
        "person",
        "estate_owner",
        "pilot",
        "household",
        "estate_household",
        "firm",
        "bank",
        "government",
        "home",
        "public_housing",
        "estate_grounds",
        "runway",
        "hangar",
        "jet",
        "road",
        "fossil_plant",
        "renewable_plant",
        "fire_station",
        "forest",
        "commons",
    ] {
        assert!(c.entities.iter().any(|e| e.id == id), "missing {id}");
    }
}

#[test]
fn footprints_match_the_initial_map() {
    let snap = Api::init(7).unwrap().snapshot();
    let reg = entities::load().unwrap();
    for k in entities::ALL_TILE_KINDS {
        let e = reg.iter().find(|e| e.tile_kinds.contains(&k)).unwrap();
        let n = snap.map.tiles.iter().filter(|t| t.kind == k).count();
        assert_eq!(
            n % e.footprint.len(),
            0,
            "{k:?}: {n} tiles do not tile {} footprints of {} cells",
            e.id,
            e.footprint.len()
        );
    }
    // The estate's fixed scenery is exactly one instance of its footprint.
    let count = |k: TileKindKey| snap.map.tiles.iter().filter(|t| t.kind == k).count();
    assert_eq!(count(TileKindKey::EstateGrounds), 3);
    assert_eq!(count(TileKindKey::Runway), 3);
}

#[test]
fn a_made_up_entity_added_only_in_content_appears() {
    let mut v = file();
    list(&mut v).push(json!({
        "id": "ngo", "kind": "actor", "category": "people", "label": "Climate group",
        "plural": "Climate groups", "roles": ["payer", "payee", "labelled"], "owner": "none"
    }));
    let reg = parse(&v).unwrap();
    let ngo = reg.iter().find(|e| e.id == "ngo").unwrap();
    assert_eq!(ngo.label, "Climate group");
    assert!(ngo.tile_kinds.is_empty() && ngo.scene_model.is_empty());
}

#[test]
fn bad_content_is_refused() {
    type Change = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Change)> = vec![
        (
            "duplicate id",
            Box::new(|v| {
                let e = list(v)[0].clone();
                list(v).push(e);
            }),
        ),
        (
            "unknown role",
            Box::new(|v| {
                list(v)[0]["roles"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("flies"))
            }),
        ),
        (
            "located without footprint",
            Box::new(|v| list(v)[5]["footprint"] = json!([])),
        ),
        (
            "variant of nothing",
            Box::new(|v| list(v)[1]["variantOf"] = json!("nobody")),
        ),
        (
            "unclaimed tile kind",
            Box::new(|v| {
                list(v).retain(|e| e["id"] != "commons");
            }),
        ),
        (
            "tile kind claimed twice",
            Box::new(|v| list(v)[0]["tileKinds"] = json!(["firm"])),
        ),
        (
            "emissions section without emitter",
            Box::new(|v| list(v)[0]["inspector"] = json!(["emissions"])),
        ),
        (
            "scene model without default",
            Box::new(|v| list(v)[5]["sceneModel"] = json!({"goods": ["firm_goods"]})),
        ),
        (
            "unknown field",
            Box::new(|v| list(v)[0]["colour"] = json!("red")),
        ),
        (
            "unknown section",
            Box::new(|v| list(v)[0]["inspector"] = json!(["nope"])),
        ),
        (
            "not labelled",
            Box::new(|v| list(v)[0]["roles"] = json!(["energy_consumer"])),
        ),
        (
            "buildable without infra kind",
            Box::new(|v| list(v)[0]["roles"] = json!(["buildable", "labelled"])),
        ),
    ];
    for (name, change) in cases {
        let mut v = file();
        change(&mut v);
        assert!(parse(&v).is_err(), "{name} was accepted");
    }
}
