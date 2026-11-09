import * as THREE from 'three';
import type { GameState } from '../store';
import { worldOf } from './coords';
import { INFRA, SCENERY } from './palette';

const boxGeo = new THREE.BoxGeometry(0.8, 1, 0.8);

function box(color: number, height: number, tile: { x: number; y: number }, size: number, opacity = 1) {
  const mat = new THREE.MeshLambertMaterial({ color, transparent: opacity < 1, opacity });
  const m = new THREE.Mesh(boxGeo, mat);
  m.scale.y = height;
  const [x, , z] = worldOf(tile, size, height / 2);
  m.position.set(x, height / 2 + 0.02, z);
  return m;
}

/** Rebuilds the building meshes for the current layout and draft. Cheap enough for 144 tiles. */
export function buildEntities(s: GameState, turn: number): THREE.Group {
  const g = new THREE.Group();
  const n = s.layout.gridSize;
  for (const sc of s.layout.scenery) g.add(box(SCENERY[sc.kind].color, SCENERY[sc.kind].height, sc, n));
  for (const p of s.layout.placed) {
    const building = p.effectiveTurn > turn;
    g.add(box(INFRA[p.kind].color, INFRA[p.kind].height, p, n, building ? 0.45 : 1));
  }
  for (const d of s.draft) {
    if (d.command.type === 'place' && d.command.tile) g.add(box(INFRA[d.command.kind].color, INFRA[d.command.kind].height, d.command.tile, n, 0.5));
  }
  return g;
}
