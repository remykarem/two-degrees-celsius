import * as THREE from 'three';
import type { EntityDesc } from './entities';
import { buildModel, makeTintMaterial, setTint, type BuiltModel } from './geometry';
import { CRANE, EDGE } from './palette';
import { MODELS, type ModelId } from './models';

const easeOutBack = (t: number) => { const c = 1.70158, x = Math.min(1, Math.max(0, t)) - 1; return 1 + (c + 1) * x ** 3 + c * x ** 2; };

interface Item {
  desc: EntityDesc;
  group: THREE.Group;
  mesh: THREE.Mesh;
  mat: THREE.MeshLambertMaterial;
  spin?: THREE.Mesh;
  extra: THREE.Object3D[];
  /** Seconds left before the pop-in starts (hidden until then); null when no pop is scheduled. */
  popDelay: number | null;
  popT: number;
}

const POP_S = 0.55;
const labelTex = new Map<string, THREE.CanvasTexture>();
function numberTexture(n: number): THREE.CanvasTexture {
  const k = String(n);
  const hit = labelTex.get(k);
  if (hit) return hit;
  const c = document.createElement('canvas');
  c.width = c.height = 96;
  const g = c.getContext('2d')!;
  g.fillStyle = '#1f2430';
  g.beginPath(); g.arc(48, 48, 44, 0, 7); g.fill();
  g.strokeStyle = '#f0b429'; g.lineWidth = 8; g.setLineDash([14, 10]);
  g.beginPath(); g.arc(48, 48, 38, 0, 7); g.stroke();
  g.setLineDash([]);
  g.fillStyle = '#ffffff'; g.font = 'bold 46px system-ui, sans-serif'; g.textAlign = 'center'; g.textBaseline = 'middle';
  g.fillText(k, 48, 52);
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  labelTex.set(k, tex);
  return tex;
}

/** Owns the building meshes: one draw call per building, plus one merged outline for the whole town. */
export class EntityLayer {
  readonly group = new THREE.Group();
  private items = new Map<string, Item>();
  private models = new Map<ModelId, BuiltModel>();
  private edges: THREE.LineSegments | null = null;
  private edgeMat = new THREE.LineBasicMaterial({ color: EDGE, transparent: true, opacity: 0.38 });
  private edgesDirty = false;
  private hover: string | null = null;
  private selected: string | null = null;
  private crane: THREE.BufferGeometry | null = null;
  private spriteMats: THREE.SpriteMaterial[] = [];

  private model(id: ModelId): BuiltModel {
    let m = this.models.get(id);
    if (!m) this.models.set(id, (m = buildModel(MODELS[id])));
    return m;
  }

  /** Meshes to raycast against (buildings only). */
  get pickables(): THREE.Object3D[] { return [...this.items.values()].filter((i) => i.group.visible && i.desc.state !== 'draft').map((i) => i.mesh); }
  keyOfMesh(o: THREE.Object3D): string | null {
    for (const [k, i] of this.items) if (i.mesh === o) return k;
    return null;
  }
  get count() { return this.items.size; }
  get hasSpinners() { return this.spinners; }
  private spinners = false;
  get popping() { return [...this.items.values()].some((i) => i.popDelay !== null); }

  /**
   * Replace the set of buildings. `popKeys` are keys that should appear with a pop-in after `delay` seconds
   * (completed builds, newly founded firms). Everything else appears at once.
   */
  rebuild(descs: EntityDesc[], popKeys: Map<string, number>) {
    this.clear();
    for (const d of descs) {
      const built = this.model(d.model);
      const ghosty = d.state !== 'normal';
      const mat = makeTintMaterial(d.tint, ghosty ? 0.46 : 1);
      const mesh = new THREE.Mesh(built.geometry, mat);
      mesh.castShadow = !ghosty;
      mesh.receiveShadow = true;
      const group = new THREE.Group();
      group.position.set(d.x, 0.06, d.z);
      group.add(mesh);
      const item: Item = { desc: d, group, mesh, mat, extra: [], popDelay: popKeys.get(d.key) ?? null, popT: 0 };
      if (built.spin) {
        const sm = makeTintMaterial(0xffffff);
        const spin = new THREE.Mesh(built.spin.geometry, sm);
        spin.position.set(...built.spin.at);
        spin.castShadow = true;
        group.add(spin);
        item.spin = spin;
        item.extra.push(spin);
      }
      if (d.state !== 'normal') this.addConstructionMarks(item, built);
      if (item.popDelay !== null) { group.visible = false; group.scale.setScalar(0.001); }
      this.items.set(d.key, item);
      this.group.add(group);
    }
    this.spinners = [...this.items.values()].some((i) => i.spin);
    this.edgesDirty = true;
    this.rebuildEdges();
  }

  private addConstructionMarks(item: Item, built: BuiltModel) {
    const d = item.desc;
    if (d.state === 'pending' && d.ref.kind !== 'road') {
      if (!this.crane) this.crane = this.model('crane' as ModelId).geometry;
      const crane = new THREE.Mesh(this.crane, makeTintMaterial(CRANE, 0.9));
      item.group.add(crane);
      item.extra.push(crane);
    }
    void built;
    const top = MODELS[d.model].height + 0.3;
    const sm = new THREE.SpriteMaterial({ map: numberTexture(Math.max(0, d.turnsLeft)), depthTest: false, transparent: true });
    this.spriteMats.push(sm);
    const sprite = new THREE.Sprite(sm);
    sprite.position.set(0, top, 0);
    sprite.scale.setScalar(0.42);
    sprite.renderOrder = 20;
    item.group.add(sprite);
    item.extra.push(sprite);
  }

  private rebuildEdges() {
    if (this.edges) { this.group.remove(this.edges); this.edges.geometry.dispose(); this.edges = null; }
    const parts: Float32Array[] = [];
    for (const it of this.items.values()) {
      if (it.desc.state !== 'normal' || it.popDelay !== null) continue;
      const src = this.model(it.desc.model).edges;
      const out = new Float32Array(src.length);
      for (let i = 0; i < src.length; i += 3) { out[i] = src[i]! + it.desc.x; out[i + 1] = src[i + 1]! + 0.06; out[i + 2] = src[i + 2]! + it.desc.z; }
      parts.push(out);
    }
    const all = new Float32Array(parts.reduce((a, p) => a + p.length, 0));
    let o = 0;
    for (const p of parts) { all.set(p, o); o += p.length; }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(all, 3));
    this.edges = new THREE.LineSegments(geo, this.edgeMat);
    this.edges.frustumCulled = false;
    this.group.add(this.edges);
    this.edgesDirty = false;
  }

  setTints(tints: Map<string, number>) {
    for (const [k, it] of this.items) { const t = tints.get(k); if (t !== undefined) setTint(it.mat, t); }
  }
  setHover(key: string | null) { this.hover = key; this.applyGlow(); }
  setSelected(key: string | null) { this.selected = key; this.applyGlow(); }
  private applyGlow() {
    for (const [k, it] of this.items) {
      const g = k === this.selected ? 0x2a3f7a : k === this.hover ? 0x3a3a22 : 0x000000;
      it.mat.emissive.setHex(g);
    }
  }

  /** Advance animation by dt seconds and spin turbines. Returns true while something is still moving. */
  update(dt: number, sec: number): boolean {
    let moving = false;
    for (const it of this.items.values()) {
      if (it.spin) it.spin.rotation.z = sec * 1.4;
      if (it.popDelay === null) continue;
      moving = true;
      if (it.popDelay > 0) { it.popDelay = Math.max(0, it.popDelay - dt); if (it.popDelay > 0) continue; }
      it.group.visible = true;
      it.popT += dt;
      const f = easeOutBack(it.popT / POP_S);
      it.group.scale.set(f, Math.max(0.001, f), f);
      if (it.popT >= POP_S) { it.popDelay = null; it.group.scale.setScalar(1); this.edgesDirty = true; }
    }
    if (this.edgesDirty && !this.popping) this.rebuildEdges();
    return moving;
  }

  /** Finish every pending pop-in immediately (skip / reduced motion). */
  finishPops() {
    for (const it of this.items.values()) if (it.popDelay !== null) { it.popDelay = null; it.group.visible = true; it.group.scale.setScalar(1); }
    this.rebuildEdges();
  }

  private clear() {
    for (const it of this.items.values()) { this.group.remove(it.group); it.mat.dispose(); for (const e of it.extra) { const m = (e as THREE.Mesh).material as THREE.Material | undefined; m?.dispose?.(); } }
    this.items.clear();
    this.spriteMats.forEach((m) => m.dispose());
    this.spriteMats = [];
    if (this.edges) { this.group.remove(this.edges); this.edges.geometry.dispose(); this.edges = null; }
  }

  dispose() {
    this.clear();
    this.models.forEach((m) => { m.geometry.dispose(); m.spin?.geometry.dispose(); });
    this.models.clear();
    this.edgeMat.dispose();
    labelTex.forEach((t) => t.dispose());
    labelTex.clear();
  }
}
