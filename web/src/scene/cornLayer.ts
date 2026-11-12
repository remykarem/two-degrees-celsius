import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import type { EntityDesc } from './entities';
import { cornColours, cornLook, easeStock, slotScale, stalkSlots, SLOTS, type Slot } from './corn';

/** Ground level of a tile's top (group y 0.06 + tile slab 0.04). */
const BASE_Y = 0.1;
/** Sway: tip displacement in stalk units, and speed (rad/s). Look constants only. */
const SWAY_AMP = 0.07;
const SWAY_SPEED = 1.5;
/** Where on the stalk (0..1) the cob sits. */
const COB_AT = 0.55;
/** Stock easing rate (per second) when the commons changes. */
const EASE_RATE = 2.5;

const grey = (g: BufferGeometry, v: number) => {
  const n = g.attributes.position!.count;
  g.setAttribute('color', new THREE.BufferAttribute(new Float32Array(n * 3).fill(v), 3));
  return g;
};
type BufferGeometry = THREE.BufferGeometry;

/** One stalk, 1 unit tall (instances scale it): stem, four angled blades, a small tassel. Vertex colour is a grey shade; the instance colour gives the hue. */
function stalkGeometry(): BufferGeometry {
  const parts: BufferGeometry[] = [];
  const stem = new THREE.CylinderGeometry(0.016, 0.028, 1, 5); stem.translate(0, 0.5, 0);
  parts.push(grey(stem.toNonIndexed(), 0.82));
  [[0.3, 0], [0.46, 2.4], [0.62, 4.1], [0.76, 1.1]].forEach(([y, yaw], i) => {
    const blade = new THREE.BoxGeometry(0.34 - i * 0.04, 0.012, 0.07); // long thin leaf
    blade.translate(0.17 - i * 0.02, 0, 0);
    blade.rotateZ(-0.55 + i * 0.07);
    blade.rotateY(yaw!);
    blade.translate(0, y!, 0);
    parts.push(grey(blade.toNonIndexed(), 1));
  });
  const tassel = new THREE.ConeGeometry(0.04, 0.16, 5); tassel.translate(0, 1.0, 0);
  parts.push(grey(tassel.toNonIndexed(), 0.9));
  for (const p of parts) p.deleteAttribute('uv');
  const merged = mergeGeometries(parts, false)!;
  parts.forEach((p) => p.dispose());
  return merged;
}

/** A cob with husk base, 1 stalk unit scale, base at the origin, tilted outward. */
function cobGeometry(): BufferGeometry {
  const cob = new THREE.CylinderGeometry(0.05, 0.062, 0.26, 7); cob.translate(0, 0.13, 0);
  const tip = new THREE.ConeGeometry(0.05, 0.08, 7); tip.translate(0, 0.3, 0);
  const g = mergeGeometries([cob.toNonIndexed(), tip.toNonIndexed()].map((q) => { q.deleteAttribute('uv'); return grey(q, 1); }), false)!;
  cob.dispose(); tip.dispose();
  g.rotateZ(-0.35);
  g.translate(0.05, 0, 0);
  return g;
}

interface Tile { key: string; x: number; z: number; slots: Slot[]; shown: number; target: number; tint: number | null }

/**
 * The commons corn: two instanced meshes (stalks, cobs) for every commons tile, so two draw calls however many tiles there are.
 * Placement comes from corn.ts (seeded by tile coordinates); this class only writes instance matrices and colours from the stock the
 * snapshot reports, eases changes, and sways the stalks in the vertex shader (no per-frame matrix work; off under reduced motion).
 */
export class CornLayer {
  readonly group = new THREE.Group();
  private tiles: Tile[] = [];
  private stalks: THREE.InstancedMesh | null = null;
  private cobs: THREE.InstancedMesh | null = null;
  private readonly stalkGeo = stalkGeometry();
  private readonly cobGeo = cobGeometry();
  private readonly uTime = { value: 0 };
  private readonly uAmp = { value: SWAY_AMP };
  private readonly stalkMat = this.material(false);
  private readonly cobMat = this.material(true);
  private touched = false;
  private dry = 0;
  private lensOn = false;
  private tmpM = new THREE.Matrix4();
  private tmpQ = new THREE.Quaternion();
  private tmpE = new THREE.Euler();
  private tmpP = new THREE.Vector3();
  private tmpS = new THREE.Vector3();
  private tmpC = new THREE.Color();

  private material(cob: boolean): THREE.MeshLambertMaterial {
    const mat = new THREE.MeshLambertMaterial({ vertexColors: true });
    mat.onBeforeCompile = (sh) => {
      sh.uniforms.uTime = this.uTime;
      sh.uniforms.uAmp = this.uAmp;
      sh.vertexShader = sh.vertexShader
        .replace('#include <common>', '#include <common>\nuniform float uTime;\nuniform float uAmp;')
        .replace('#include <begin_vertex>', `#include <begin_vertex>
{ float ph = uTime * ${SWAY_SPEED.toFixed(2)} + instanceMatrix[3].x * 3.1 + instanceMatrix[3].z * 2.3;
  float h = ${cob ? (COB_AT * COB_AT).toFixed(3) : 'position.y * position.y'};
  transformed.x += sin(ph) * uAmp * h; transformed.z += sin(ph * 0.8 + 1.0) * uAmp * 0.5 * h; }`);
    };
    mat.customProgramCacheKey = () => (cob ? 'corn-cob' : 'corn-stalk');
    return mat;
  }

  get count() { return this.tiles.length; }
  /** True while stalks sway (a repaint is needed even when nothing else moves). */
  get swaying() { return this.tiles.length > 0 && this.uAmp.value > 0; }

  /** Set the commons tiles from the entity descriptions. A tile already shown keeps its shown stock (so it eases to the new one). */
  setTiles(descs: EntityDesc[], dry: number, reduced: boolean) {
    const prev = new Map(this.tiles.map((t) => [t.key, t]));
    const commons = descs.filter((d) => d.ref.kind === 'commons' && d.state === 'normal');
    this.dry = dry;
    this.tiles = commons.map((d) => {
      const target = d.stock ?? 1;
      const old = prev.get(d.key);
      return { key: d.key, x: d.x, z: d.z, slots: old?.slots ?? stalkSlots(d.ref.tile.x, d.ref.tile.y), shown: old && !reduced ? old.shown : target, target, tint: old?.tint ?? null };
    });
    this.allocate();
    this.writeAll();
  }

  /** Lens colours per entity key (EntityDesc.key). Under no lens the corn shows only its own stock colour. */
  setTints(tints: Map<string, number>, lensOn: boolean) {
    this.lensOn = lensOn;
    for (const t of this.tiles) t.tint = lensOn ? tints.get(t.key) ?? null : null;
    this.writeAll();
  }

  /** Advance easing and sway. Returns true while the shown stock is still moving. */
  update(dt: number, sec: number, reduced: boolean): boolean {
    this.uAmp.value = reduced ? 0 : SWAY_AMP;
    this.uTime.value = sec;
    let moving = false;
    for (let i = 0; i < this.tiles.length; i++) {
      const t = this.tiles[i]!;
      if (t.shown === t.target) continue;
      t.shown = reduced ? t.target : easeStock(t.shown, t.target, dt, EASE_RATE);
      this.writeTile(i);
      this.touched = true;
      if (t.shown !== t.target) moving = true;
    }
    if (this.stalks && this.touched) { this.flag(); this.touched = false; }
    return moving;
  }

  private allocate() {
    for (const m of [this.stalks, this.cobs]) if (m) { this.group.remove(m); m.dispose(); }
    this.stalks = this.cobs = null;
    const n = this.tiles.length * SLOTS;
    if (!n) return;
    this.stalks = new THREE.InstancedMesh(this.stalkGeo, this.stalkMat, n);
    this.cobs = new THREE.InstancedMesh(this.cobGeo, this.cobMat, n);
    for (const m of [this.stalks, this.cobs]) {
      m.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(n * 3), 3);
      m.frustumCulled = false;
      m.receiveShadow = true;
      this.group.add(m);
    }
  }

  private flag() { for (const m of [this.stalks, this.cobs]) if (m) { m.instanceMatrix.needsUpdate = true; m.instanceColor!.needsUpdate = true; } }
  private writeAll() { for (let i = 0; i < this.tiles.length; i++) this.writeTile(i); this.flag(); }

  private writeTile(i: number) {
    const t = this.tiles[i]!;
    const look = cornLook(t.shown);
    const col = cornColours(t.shown, { lensTint: t.tint, dry: this.dry });
    for (let k = 0; k < SLOTS; k++) {
      const s = t.slots[k]!;
      const sc = slotScale(s, look);
      const idx = i * SLOTS + k;
      this.tmpP.set(t.x + s.x, BASE_Y, t.z + s.z);
      this.tmpQ.setFromEuler(this.tmpE.set(0, s.yaw, s.lean));
      const u = Math.max(sc.stalk, 1e-5);
      this.stalks!.setMatrixAt(idx, this.tmpM.compose(this.tmpP, this.tmpQ, this.tmpS.set(u, u, u)));
      this.stalks!.setColorAt(idx, this.tmpC.setHex(col.stalk).multiplyScalar(s.shade));
      const cs = sc.cob;
      this.tmpP.y = BASE_Y + COB_AT * sc.stalk;
      this.tmpQ.setFromEuler(this.tmpE.set(0, s.yaw + 1.7, 0));
      this.cobs!.setMatrixAt(idx, this.tmpM.compose(this.tmpP, this.tmpQ, this.tmpS.set(cs, cs, cs)));
      this.cobs!.setColorAt(idx, this.tmpC.setHex(col.cob).multiplyScalar(0.94 + 0.12 * (s.shade - 0.9) / 0.2));
    }
  }

  dispose() {
    for (const m of [this.stalks, this.cobs]) if (m) { this.group.remove(m); m.dispose(); }
    this.stalkGeo.dispose(); this.cobGeo.dispose(); this.stalkMat.dispose(); this.cobMat.dispose();
  }
}
