import * as THREE from 'three';
import { hash01 } from './coords';
import { mergeParts } from './geometry';
import type { Part } from './models';
import { FAR_GRASS, GROUND, HEDGE, LEAF, SKY_LOW, SKY_MID, SKY_TOP, SOIL, SOIL_DARK, WOOD } from './palette';

/** Pure: positions of rim trees and hedges around a size x size grid. Deterministic, no overlap with the grid itself. */
export interface RimItem { kind: 'tree' | 'hedge'; x: number; z: number; s: number; ry: number; len: number }
export function rimItems(size: number): RimItem[] {
  const out: RimItem[] = [];
  const edge = size / 2 + 0.55; // centre line of the one-tile rim around the grid
  const steps = size + 2;
  let k = 0;
  for (let side = 0; side < 4; side++) {
    for (let i = 0; i < steps; i++, k++) {
      const u = -edge + (i / (steps - 1)) * 2 * edge;
      const h = hash01(k * 7 + 3);
      const [x, z] = side === 0 ? [u, -edge] : side === 1 ? [edge, u] : side === 2 ? [u, edge] : [-edge, u];
      const jx = (hash01(k * 13 + 1) - 0.5) * 0.3, jz = (hash01(k * 17 + 2) - 0.5) * 0.3;
      if (h < 0.38) out.push({ kind: 'tree', x: x + jx, z: z + jz, s: 0.7 + 0.55 * hash01(k * 5 + 9), ry: hash01(k) * 6, len: 1 });
      else if (h < 0.55) out.push({ kind: 'hedge', x, z, s: 1, ry: side % 2 === 0 ? 0 : Math.PI / 2, len: 0.85 });
    }
  }
  return out;
}

const P = (s: Part['s'], x: number, y: number, z: number, w: number, h: number, d: number, c: number): Part => ({ s, p: [x, y, z], z: [w, h, d], c });

export function skyTexture(): THREE.CanvasTexture {
  const c = document.createElement('canvas');
  c.width = 4; c.height = 256;
  const g = c.getContext('2d')!;
  const grad = g.createLinearGradient(0, 0, 0, 256);
  grad.addColorStop(0, SKY_TOP); grad.addColorStop(0.55, SKY_MID); grad.addColorStop(1, SKY_LOW);
  g.fillStyle = grad; g.fillRect(0, 0, 4, 256);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}

export interface Island { group: THREE.Group; sky: THREE.CanvasTexture; dispose(): void }

/**
 * The setting: a soft sky gradient, a meadow that fades into it, and the town on a raised plot with a soil edge, ringed by trees and hedges.
 * Everything static; trees and hedges are one instanced draw call each.
 */
export function createIsland(size: number): Island {
  const group = new THREE.Group();
  const geos: THREE.BufferGeometry[] = [];
  const mats: THREE.Material[] = [];
  const own = <T extends THREE.BufferGeometry | THREE.Material>(x: T): T => { (x instanceof THREE.BufferGeometry ? geos : mats).push(x as never); return x; };
  const sky = skyTexture();

  // Meadow: grass in the middle, fading to the sky colour at the edge.
  const meadow = own(new THREE.RingGeometry(0.01, 26, 72, 8).rotateX(-Math.PI / 2));
  const pos = meadow.attributes.position!;
  const cols = new Float32Array(pos.count * 3);
  const a = new THREE.Color(FAR_GRASS), b = new THREE.Color(SKY_MID), tmp = new THREE.Color();
  for (let i = 0; i < pos.count; i++) {
    const r = Math.hypot(pos.getX(i), pos.getZ(i));
    tmp.copy(a).lerp(b, Math.min(1, Math.max(0, (r - 13) / 12)) ** 1.4);
    cols.set([tmp.r, tmp.g, tmp.b], i * 3);
  }
  meadow.setAttribute('color', new THREE.BufferAttribute(cols, 3));
  const meadowMesh = new THREE.Mesh(meadow, own(new THREE.MeshLambertMaterial({ vertexColors: true })));
  meadowMesh.position.y = -0.72;
  meadowMesh.receiveShadow = true;
  group.add(meadowMesh);

  // The raised plot: soil sides, grass top, a darker soil base.
  const w = size + 2.2;
  const side = () => own(new THREE.MeshLambertMaterial({ color: SOIL }));
  const plot = new THREE.Mesh(own(new THREE.BoxGeometry(w, 0.66, w)), [side(), side(), own(new THREE.MeshLambertMaterial({ color: GROUND[1] })), side(), side(), side()]);
  plot.position.y = -0.39;
  plot.receiveShadow = true;
  group.add(plot);
  const base = new THREE.Mesh(own(new THREE.BoxGeometry(w + 0.12, 0.14, w + 0.12)), own(new THREE.MeshLambertMaterial({ color: SOIL_DARK })));
  base.position.y = -0.66;
  group.add(base);

  const items = rimItems(size);
  const trees = items.filter((i) => i.kind === 'tree'), hedges = items.filter((i) => i.kind === 'hedge');
  const treeGeo = own(mergeParts([P('cyl', 0, 0, 0, 0.09, 0.2, 0.09, WOOD), P('cone', 0, 0.14, 0, 0.5, 0.5, 0.5, LEAF), P('cone', 0, 0.4, 0, 0.38, 0.46, 0.38, LEAF)]));
  const hedgeGeo = own(mergeParts([P('box', 0, 0, 0, 1, 0.2, 0.26, HEDGE), P('sphere', -0.25, 0.14, 0, 0.3, 0.16, 0.26, HEDGE), P('sphere', 0.2, 0.14, 0, 0.34, 0.18, 0.26, HEDGE)]));
  const mat = own(new THREE.MeshLambertMaterial({ vertexColors: true }));
  const place = (list: RimItem[], geo: THREE.BufferGeometry, y: number, tone: (i: number) => THREE.Color) => {
    const m = new THREE.InstancedMesh(geo, mat, Math.max(1, list.length));
    const d = new THREE.Object3D();
    list.forEach((it, i) => {
      d.position.set(it.x, y, it.z);
      d.rotation.set(0, it.ry, 0);
      d.scale.set(it.kind === 'hedge' ? it.len : it.s, it.kind === 'hedge' ? 1 : it.s, it.kind === 'hedge' ? 1 : it.s);
      d.updateMatrix();
      m.setMatrixAt(i, d.matrix);
      m.setColorAt(i, tone(i));
    });
    m.count = list.length;
    m.castShadow = true; m.receiveShadow = true;
    return m;
  };
  group.add(
    place(trees, treeGeo, -0.06, (i) => new THREE.Color().setHSL(0.3 + 0.03 * hash01(i * 3), 0.2 + 0.2 * hash01(i), 0.78 + 0.22 * hash01(i + 40))),
    place(hedges, hedgeGeo, -0.06, (i) => new THREE.Color().setHSL(0.3, 0.1, 0.85 + 0.15 * hash01(i + 9))),
  );

  return {
    group, sky,
    dispose() { sky.dispose(); geos.forEach((g) => g.dispose()); mats.forEach((m) => m.dispose()); group.children.forEach((c) => (c as THREE.InstancedMesh).dispose?.()); },
  };
}
