//! The game content embedded at build time. Compiling the JSON in keeps the library free of
//! file I/O (the native CLI reads files; the browser build has no file system).
use sim_params::ContentJson;

pub const PARAMS: &str = include_str!("../../../content/params.json");
pub const INITIAL_TOWN: &str = include_str!("../../../content/initial_town.json");
pub const MARKET: &str = include_str!("../../../content/systems/market.json");
pub const REGULATED: &str = include_str!("../../../content/systems/regulated.json");
pub const COOP: &str = include_str!("../../../content/systems/coop.json");
/// Lever and action metadata (labels, ranges, arrows, availability).
pub const LEVERS: &str = include_str!("../../../content/levers.json");

/// The sim content files of the default game.
pub fn default_content() -> ContentJson<'static> {
    ContentJson {
        params: PARAMS,
        initial_town: INITIAL_TOWN,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    }
}
