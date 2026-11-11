//! Space: the grid, roads and commutes (docs/town-v2.md sections 4 and 5).
//!
//! A commute follows the cheapest path over the 12 x 12 grid. Every tile entered costs
//! `slow_minutes`, or `fast_minutes` if it holds a road. Anything can be walked through; only
//! roads are quicker. Ties are broken by fewest tiles, so the result is deterministic.

use sim_params::{Params, SpaceParams};
use sim_types::*;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Cost of the cheapest route between two tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route {
    pub minutes: u32,
    pub tiles: u32,
}

/// Minutes to enter a tile.
fn enter_cost(map: &Map, sp: &SpaceParams, t: Tile) -> u32 {
    if map.kind_at(t) == Some(TileKind::Road) {
        sp.fast_minutes
    } else {
        sp.slow_minutes
    }
}

/// The cheapest route from every tile to `dest` (`None` only for tiles off the map, which are
/// never asked for). The cost of a route counts every tile entered on the way from the start
/// tile to `dest`: the start tile itself is free, `dest` is counted.
pub fn routes_to(map: &Map, sp: &SpaceParams, dest: Tile) -> Vec<Option<Route>> {
    let mut best: Vec<Option<Route>> = vec![None; map.cells.len()];
    if !map.in_bounds(dest) {
        return best;
    }
    // Dijkstra from the destination. Moving from `n` to its neighbour `u` (towards the
    // destination) enters `u`, so the edge costs `enter_cost(u)`.
    let mut heap = BinaryHeap::new();
    best[map.idx(dest)] = Some(Route {
        minutes: 0,
        tiles: 0,
    });
    heap.push(Reverse((0u32, 0u32, dest.y, dest.x)));
    while let Some(Reverse((minutes, tiles, y, x))) = heap.pop() {
        let u = Tile { x, y };
        let cur = best[map.idx(u)].expect("popped tiles have a route");
        if (minutes, tiles) != (cur.minutes, cur.tiles) {
            continue;
        }
        let cost = enter_cost(map, sp, u);
        for (dx, dy) in [(0i32, -1i32), (-1, 0), (1, 0), (0, 1)] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 {
                continue;
            }
            let n = Tile {
                x: nx as u8,
                y: ny as u8,
            };
            if !map.in_bounds(n) {
                continue;
            }
            let cand = Route {
                minutes: minutes + cost,
                tiles: tiles + 1,
            };
            let slot = &mut best[map.idx(n)];
            if slot.is_none_or(|old| (cand.minutes, cand.tiles) < (old.minutes, old.tiles)) {
                *slot = Some(cand);
                heap.push(Reverse((cand.minutes, cand.tiles, n.y, n.x)));
            }
        }
    }
    best
}

/// Share of working time that survives the commute.
pub fn effective_hours(sp: &SpaceParams, minutes: u32) -> f64 {
    (1.0 - sp.k_commute * (minutes as f64 - sp.free_minutes).max(0.0)).max(sp.min_effective_hours)
}

/// Commute of every person for the given jobs: (tiles, minutes). Domestic staff (the estate's
/// pilot) commute to the hangar tile. People without a job (and the job guarantee, which has
/// no workplace) travel nowhere. A worker whose household has
/// no home is assumed to commute the unhoused default.
pub fn commutes(s: &State, p: &Params, job: &[Option<FirmId>]) -> Vec<(u32, u32)> {
    let sp = &p.space;
    let mut routes: Vec<Option<Vec<Option<Route>>>> = vec![None; s.firms.len()];
    let mut hangar_routes: Option<Vec<Option<Route>>> = None;
    let mut out = vec![(0u32, 0u32); s.persons.len()];
    for x in &s.persons {
        let Some(f) = job[x.id.idx()] else {
            if let (Some(_), Some(e)) = (x.staff_of, s.estate.as_ref()) {
                let route = match s.households[x.household.idx()]
                    .home
                    .and_then(|h| s.home(h))
                    .map(|h| h.tile)
                {
                    Some(t) => hangar_routes.get_or_insert_with(|| routes_to(&s.map, sp, e.hangar))
                        [s.map.idx(t)],
                    None => None,
                };
                let r = route.unwrap_or(Route {
                    minutes: sp.unhoused_commute_minutes,
                    tiles: sp.unhoused_commute_tiles,
                });
                out[x.id.idx()] = (r.tiles, r.minutes);
            }
            continue;
        };
        let home = s.households[x.household.idx()]
            .home
            .and_then(|h| s.home(h))
            .map(|h| h.tile);
        let route = match home {
            Some(t) => {
                let table = routes[f.idx()]
                    .get_or_insert_with(|| routes_to(&s.map, sp, s.firms[f.idx()].tile));
                table[s.map.idx(t)]
            }
            None => Some(Route {
                minutes: sp.unhoused_commute_minutes,
                tiles: sp.unhoused_commute_tiles,
            }),
        };
        let r = route.unwrap_or(Route {
            minutes: sp.unhoused_commute_minutes,
            tiles: sp.unhoused_commute_tiles,
        });
        out[x.id.idx()] = (r.tiles, r.minutes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp() -> SpaceParams {
        SpaceParams {
            slow_minutes: 6,
            fast_minutes: 2,
            free_minutes: 10.0,
            k_commute: 0.004,
            min_effective_hours: 0.5,
            unhoused_commute_tiles: 6,
            unhoused_commute_minutes: 36,
        }
    }

    fn road(map: &mut Map, x: u8, y: u8) {
        map.set(
            Tile { x, y },
            MapTile {
                kind: TileKind::Road,
                id: None,
            },
        );
    }

    #[test]
    fn straight_commute_without_and_with_roads() {
        let mut map = Map::new(5, 1);
        let dest = Tile { x: 4, y: 0 };
        let home = Tile { x: 0, y: 0 };
        let r = routes_to(&map, &sp(), dest)[map.idx(home)].unwrap();
        assert_eq!(
            r,
            Route {
                minutes: 24,
                tiles: 4
            }
        );
        for x in 1..=3 {
            road(&mut map, x, 0);
        }
        let r = routes_to(&map, &sp(), dest)[map.idx(home)].unwrap();
        assert_eq!(
            r,
            Route {
                minutes: 12,
                tiles: 4
            },
            "3 road tiles at 2 plus the slow last tile"
        );
    }

    #[test]
    fn a_longer_road_route_beats_a_shorter_slow_one() {
        // 5 x 3 grid. The direct way along row 1 has no road (4 slow tiles = 24 minutes); a
        // detour over the road along row 0 is longer in tiles but quicker in minutes.
        let mut map = Map::new(5, 3);
        for x in 0..5 {
            road(&mut map, x, 0);
        }
        let dest = Tile { x: 4, y: 1 };
        let home = Tile { x: 0, y: 1 };
        let r = routes_to(&map, &sp(), dest)[map.idx(home)].unwrap();
        // Up to (0,0) 2, along the road to (4,0) 4 x 2 = 8, down to the destination 6: 16 < 24.
        assert_eq!(r.minutes, 16);
        assert_eq!(r.tiles, 6);
    }

    #[test]
    fn ties_prefer_fewer_tiles_and_results_are_repeatable() {
        let map = Map::new(6, 6);
        let a = routes_to(&map, &sp(), Tile { x: 5, y: 5 });
        let b = routes_to(&map, &sp(), Tile { x: 5, y: 5 });
        assert_eq!(a, b);
        let r = a[map.idx(Tile { x: 0, y: 0 })].unwrap();
        assert_eq!(r.tiles, 10, "Manhattan distance when nothing is faster");
        assert_eq!(r.minutes, 60);
    }

    #[test]
    fn hours_fall_with_commute_time_above_the_free_minutes() {
        let sp = sp();
        assert_eq!(effective_hours(&sp, 5), 1.0);
        assert_eq!(effective_hours(&sp, 10), 1.0);
        assert!((effective_hours(&sp, 30) - 0.92).abs() < 1e-12);
        assert_eq!(effective_hours(&sp, 10_000), 0.5);
    }
}
