import * as THREE from 'three';
import { nearFire, type WeatherLook } from '../store/scene-weather';
import { hash01, worldOf } from './coords';
import { mix } from './palette';

/** Capped pools (instance and draw-call budget): three instanced meshes for every fire, however many tiles burn. */
export const WEATHER_CAPS = { flames: 48, smoke: 96, haze: 40 } as const;
const FLAMES_PER_TILE = 3, PUFFS_PER_TILE = 7;
const FLAME_A = 0xff6a1a, FLAME_B = 0xffc43a;
const SMOKE_LOW = 0x4b4540, SMOKE_HIGH = 0x9a928a;

export interface Flame { x: number; z: number; h: number; phase: number }
export interface Puff { x: number; z: number; phase: number }

/** Pure: flame and smoke-column emitters for the burning tiles (deterministic, capped). */
export function planFire(look: WeatherLook, size: number): { flames: Flame[]; puffs: Puff[] } {
  const flames: Flame[] = [], puffs: Puff[] = [];
  look.fires.forEach((f, fi) => {
    const [cx, , cz] = worldOf(f.tile, size);
    for (let i = 0; i < FLAMES_PER_TILE && flames.length < WEATHER_CAPS.flames; i++) {
      const k = fi * 17 + i * 5 + f.tile.x * 3 + f.tile.y;
      flames.push({ x: cx + (hash01(k) - 0.5) * 0.6, z: cz + (hash01(k + 91) - 0.5) * 0.6, h: 0.7 + 0.6 * hash01(k + 7), phase: hash01(k + 33) });
    }
    for (let i = 0; i < PUFFS_PER_TILE && puffs.length < WEATHER_CAPS.smoke; i++) puffs.push({ x: cx, z: cz, phase: i / PUFFS_PER_TILE });
  });
  return { flames, puffs };
}

/** Pure: one haze puff over each person standing within the smoke radius of a fire (capped). */
export function planHaze(look: WeatherLook, stands: readonly { id: number; tile: { x: number; y: number }; at: readonly [number, number] }[]): { id: number; x: number; z: number }[] {
  const out: { id: number; x: number; z: number }[] = [];
  if (!look.fires.length) return out;
  for (const s of stands) if (out.length < WEATHER_CAPS.haze && nearFire(look, s.tile)) out.push({ id: s.id, x: s.at[0], z: s.at[1] });
  return out;
}

const dummy = new THREE.Object3D();

/** Flames, smoke columns, haze over people near a fire, and the fire-station coverage ring. Static when `animate` is false (reduced motion). */
export class WeatherLayer {
  readonly group = new THREE.Group();
  private flames: THREE.InstancedMesh;
  private smoke: THREE.InstancedMesh;
  private haze: THREE.InstancedMesh;
  private ring: THREE.Mesh;
  private disc: THREE.Mesh;
  private geos: THREE.BufferGeometry[] = [];
  private mats: THREE.Material[] = [];
  private fire: { flames: Flame[]; puffs: Puff[] } = { flames: [], puffs: [] };
  private hazePlan: { id: number; x: number; z: number }[] = [];
  private c = new THREE.Color();
  /** Counts drawn last frame, for stats and tests. */
  drawn = { flames: 0, smoke: 0, haze: 0 };

  constructor() {
    const flame = new THREE.ConeGeometry(0.15, 0.62, 6).translate(0, 0.31, 0);
    const puff = new THREE.IcosahedronGeometry(0.5, 1);
    const ring = new THREE.RingGeometry(0.94, 1, 64).rotateX(-Math.PI / 2);
    const disc = new THREE.CircleGeometry(1, 48).rotateX(-Math.PI / 2);
    this.geos.push(flame, puff, ring, disc);
    const flameMat = new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.92, depthWrite: false, fog: false });
    const smokeMat = new THREE.MeshLambertMaterial({ transparent: true, opacity: 0.55, depthWrite: false });
    const hazeMat = new THREE.MeshBasicMaterial({ color: 0x8c8176, transparent: true, opacity: 0.3, depthWrite: false });
    const ringMat = new THREE.MeshBasicMaterial({ color: 0x2f83c4, transparent: true, opacity: 0.9, depthTest: false, side: THREE.DoubleSide });
    const discMat = new THREE.MeshBasicMaterial({ color: 0x4aa3d8, transparent: true, opacity: 0.14, depthTest: false, side: THREE.DoubleSide });
    this.mats.push(flameMat, smokeMat, hazeMat, ringMat, discMat);
    const inst = (g: THREE.BufferGeometry, m: THREE.Material, cap: number) => {
      const mesh = new THREE.InstancedMesh(g, m, cap);
      mesh.count = 0; mesh.frustumCulled = false;
      mesh.setColorAt(0, new THREE.Color(1, 1, 1));
      return mesh;
    };
    this.flames = inst(flame, flameMat, WEATHER_CAPS.flames);
    this.smoke = inst(puff, smokeMat, WEATHER_CAPS.smoke);
    this.haze = inst(puff, hazeMat, WEATHER_CAPS.haze);
    this.ring = new THREE.Mesh(ring, ringMat);
    this.disc = new THREE.Mesh(disc, discMat);
    for (const m of [this.ring, this.disc]) { m.visible = false; m.renderOrder = 9; m.frustumCulled = false; }
    this.group.add(this.flames, this.smoke, this.haze, this.ring, this.disc);
  }

  /** Current fires and the people breathing their smoke. */
  setLook(look: WeatherLook, stands: readonly { id: number; tile: { x: number; y: number }; at: readonly [number, number] }[], size: number) {
    this.fire = planFire(look, size);
    this.hazePlan = planHaze(look, stands);
  }
  /** Show the coverage ring around a fire station tile (or hide it with null). The radius (tiles) is the wire's weather.coverageRadius. */
  setCover(tile: { x: number; y: number } | null, size: number, radius: number) {
    this.ring.visible = this.disc.visible = !!tile;
    if (!tile) return;
    const [x, , z] = worldOf(tile, size);
    const r = radius + 0.5;
    for (const m of [this.ring, this.disc]) { m.position.set(x, 0.05, z); m.scale.set(r, 1, r); }
  }

  get instances() { const d = this.drawn; return d.flames + d.smoke + d.haze; }
  /** True while something here moves (fires burn): the scene keeps repainting. */
  get active() { return this.fire.flames.length > 0 || this.hazePlan.length > 0; }

  update(sec: number, animate: boolean) {
    const t = animate ? sec : 0;
    let n = 0;
    for (const f of this.fire.flames) {
      const flick = animate ? 0.75 + 0.25 * Math.sin(t * 9 + f.phase * 6.28) + 0.1 * Math.sin(t * 17 + f.phase * 3) : 1;
      dummy.position.set(f.x, 0.06, f.z);
      dummy.rotation.set(0, f.phase * 6.28 + (animate ? t * 0.8 : 0), 0);
      dummy.scale.set(flick, f.h * flick, flick);
      dummy.updateMatrix();
      this.flames.setMatrixAt(n, dummy.matrix);
      this.flames.setColorAt(n, this.c.setHex(f.phase > 0.5 ? FLAME_A : FLAME_B));
      n++;
    }
    this.flames.count = n;
    let m = 0;
    for (const p of this.fire.puffs) {
      const u = animate ? (t * 0.22 + p.phase) % 1 : p.phase;
      dummy.position.set(p.x + u * 0.35 + Math.sin(p.phase * 20) * 0.08, 0.55 + u * 1.9, p.z - u * 0.2);
      dummy.rotation.set(p.phase * 9, p.phase * 5, 0);
      dummy.scale.setScalar(0.22 + u * 0.5);
      dummy.updateMatrix();
      this.smoke.setMatrixAt(m, dummy.matrix);
      this.smoke.setColorAt(m, this.c.setHex(mix(SMOKE_LOW, SMOKE_HIGH, u)));
      m++;
    }
    this.smoke.count = m;
    let h = 0;
    for (const p of this.hazePlan) {
      dummy.position.set(p.x + (animate ? Math.sin(t * 0.6 + p.id) * 0.04 : 0), 0.62, p.z);
      dummy.rotation.set(0, p.id, 0);
      dummy.scale.set(0.75, 0.3, 0.75);
      dummy.updateMatrix();
      this.haze.setMatrixAt(h++, dummy.matrix);
    }
    this.haze.count = h;
    for (const mesh of [this.flames, this.smoke, this.haze]) {
      mesh.instanceMatrix.needsUpdate = true;
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }
    this.drawn = { flames: n, smoke: m, haze: h };
  }

  dispose() {
    for (const m of [this.flames, this.smoke, this.haze]) m.dispose();
    this.geos.forEach((g) => g.dispose());
    this.mats.forEach((x) => x.dispose());
  }
}
