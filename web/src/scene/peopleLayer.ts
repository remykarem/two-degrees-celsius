import * as THREE from 'three';
import { buildModel, makeTintMaterial, type BuiltModel } from './geometry';
import { MODELS } from './models';
import { centreOf } from './coords';
import { AGE_SCALE, HOUSEHOLD_COLORS, MARKER_SHAPES, poseAt, type Camp, type CommutePlan, type MarkerShape, type PersonStand } from './people';
import { SELECT } from './palette';

const SKIN = 0xf0c8a0, SKIN_CHILD = 0xf5d3b3, HAIR_GREY = 0xe2e2e2;
const dummy = new THREE.Object3D();
const col = new THREE.Color();
const CAP = 48;

function markerGeometry(s: MarkerShape): THREE.BufferGeometry {
  switch (s) {
    case 'sphere': return new THREE.SphereGeometry(0.05, 8, 6);
    case 'cube': return new THREE.BoxGeometry(0.08, 0.08, 0.08);
    case 'cone': return new THREE.ConeGeometry(0.06, 0.12, 8);
    case 'octa': return new THREE.OctahedronGeometry(0.07);
    case 'disc': return new THREE.CylinderGeometry(0.075, 0.075, 0.03, 12).rotateX(Math.PI / 2);
    case 'tetra': return new THREE.TetrahedronGeometry(0.075);
    case 'torus': return new THREE.TorusGeometry(0.055, 0.02, 6, 12);
    case 'dodeca': return new THREE.DodecahedronGeometry(0.06);
  }
}

export interface PersonPos { id: number; x: number; y: number; z: number }

/** One figure per person, drawn with a handful of instanced meshes: bodies, heads, sticks (retired) and one marker mesh per household shape. */
export class PeopleLayer {
  readonly group = new THREE.Group();
  private bodies: THREE.InstancedMesh;
  private heads: THREE.InstancedMesh;
  private sticks: THREE.InstancedMesh;
  private markers = new Map<MarkerShape, THREE.InstancedMesh>();
  private halo: THREE.Mesh;
  private haloMat: THREE.MeshBasicMaterial;
  private tentModel: BuiltModel;
  private tentMat: THREE.MeshLambertMaterial;
  private tents: THREE.Mesh[] = [];
  private geos: THREE.BufferGeometry[] = [];
  private mats: THREE.Material[] = [];
  private stands: PersonStand[] = [];
  private tints = new Map<number, number>();
  private slot = new Map<number, { shape: MarkerShape; i: number }>();
  /** Where everybody was drawn last frame (world), for picking and tests. */
  positions: PersonPos[] = [];
  walking = 0;

  constructor() {
    const mk = (g: THREE.BufferGeometry, m: THREE.Material, cap: number) => {
      this.geos.push(g); this.mats.push(m);
      const im = new THREE.InstancedMesh(g, m, cap);
      im.count = 0; im.frustumCulled = false; im.castShadow = true;
      im.setColorAt(0, col.setHex(0xffffff));
      return im;
    };
    this.bodies = mk(new THREE.CylinderGeometry(0.045, 0.07, 0.18, 8).translate(0, 0.1, 0), new THREE.MeshLambertMaterial(), CAP);
    this.heads = mk(new THREE.SphereGeometry(0.058, 8, 6).translate(0, 0.25, 0), new THREE.MeshLambertMaterial(), CAP);
    this.sticks = mk(new THREE.CylinderGeometry(0.01, 0.01, 0.2, 5).translate(0, 0.1, 0), new THREE.MeshLambertMaterial({ color: 0x8a6240 }), CAP);
    this.group.add(this.bodies, this.heads, this.sticks);
    for (const s of MARKER_SHAPES) {
      const m = mk(markerGeometry(s), new THREE.MeshLambertMaterial(), CAP);
      this.markers.set(s, m);
      this.group.add(m);
    }
    this.haloMat = new THREE.MeshBasicMaterial({ color: SELECT, transparent: true, opacity: 0.9, depthTest: false, side: THREE.DoubleSide });
    const ring = new THREE.RingGeometry(0.13, 0.17, 24).rotateX(-Math.PI / 2);
    this.geos.push(ring); this.mats.push(this.haloMat);
    this.halo = new THREE.Mesh(ring, this.haloMat);
    this.halo.renderOrder = 12; this.halo.visible = false;
    this.group.add(this.halo);
    this.tentModel = buildModel(MODELS.tent);
    this.tentMat = makeTintMaterial(0xffffff);
    this.mats.push(this.tentMat);
  }

  get count() { return this.stands.length; }
  get drawCalls() { return 3 + MARKER_SHAPES.length; }

  /** New set of people (snapshot changed). `n` is the grid size for the tent positions. */
  setStands(stands: PersonStand[], camps: Camp[], n: number) {
    this.stands = stands;
    this.slot.clear();
    const per = new Map<MarkerShape, number>();
    for (const s of stands) {
      const shape = MARKER_SHAPES[s.hIndex % MARKER_SHAPES.length]!;
      const i = per.get(shape) ?? 0;
      per.set(shape, i + 1);
      this.slot.set(s.id, { shape, i });
    }
    for (const t of this.tents) this.group.remove(t);
    this.tents = camps.map((c) => {
      const m = new THREE.Mesh(this.tentModel.geometry, this.tentMat);
      const [x, z] = centreOf(c.tile, n);
      m.position.set(x, 0.01, z);
      m.castShadow = true;
      this.group.add(m);
      return m;
    });
  }

  /** Body colour per person (lens). People missing from the map keep their household colour. */
  setTints(t: Map<number, number>) { this.tints = t; }

  /** Draw everybody. `plans` are this month's commutes (null outside playback), `monthT` 0..1. */
  update(sec: number, plans: Map<number, CommutePlan> | null, monthT: number, selected: number | null, hovered: number | null, reduced: boolean) {
    const counts = new Map<MarkerShape, number>();
    this.positions = [];
    this.walking = 0;
    let n = 0;
    let haloAt: PersonPos | null = null;
    for (const st of this.stands) {
      if (n >= CAP) break;
      const pose = poseAt(st, plans?.get(st.id), monthT);
      const hot = st.id === selected || st.id === hovered;
      const sc = AGE_SCALE[st.ageBand] * (hot ? 1.3 : 1);
      const bob = pose.walking && !reduced ? Math.abs(Math.sin(sec * 9 + st.id)) * 0.03 : 0;
      if (pose.walking) this.walking++;
      const y = 0.03 + bob;
      dummy.position.set(pose.x, y, pose.z);
      dummy.rotation.set(st.ageBand === 'retired' ? 0.12 : 0, pose.heading, 0);
      dummy.scale.setScalar(sc);
      dummy.updateMatrix();
      this.bodies.setMatrixAt(n, dummy.matrix);
      this.bodies.setColorAt(n, col.setHex(this.tints.get(st.id) ?? HOUSEHOLD_COLORS[st.hIndex % HOUSEHOLD_COLORS.length]!));
      this.heads.setMatrixAt(n, dummy.matrix);
      this.heads.setColorAt(n, col.setHex(st.ageBand === 'retired' ? HAIR_GREY : st.ageBand === 'child' ? SKIN_CHILD : SKIN));
      const sl = this.slot.get(st.id)!;
      const mi = counts.get(sl.shape) ?? 0;
      counts.set(sl.shape, mi + 1);
      dummy.position.set(pose.x, y + 0.4 * sc + (reduced ? 0 : Math.sin(sec * 2 + st.id) * 0.012), pose.z);
      dummy.rotation.set(0, reduced ? 0 : sec * 0.8 + st.id, 0);
      dummy.scale.setScalar(sc > 0.9 ? 1 : 0.85);
      dummy.updateMatrix();
      const mm = this.markers.get(sl.shape)!;
      mm.setMatrixAt(mi, dummy.matrix);
      mm.setColorAt(mi, col.setHex(HOUSEHOLD_COLORS[st.hIndex % HOUSEHOLD_COLORS.length]!));
      this.positions.push({ id: st.id, x: pose.x, y: y + 0.28 * sc, z: pose.z });
      if (st.id === selected) haloAt = { id: st.id, x: pose.x, y: 0.05, z: pose.z };
      n++;
    }
    // sticks for the retired (own pass so the instance index stays dense)
    let ns = 0;
    for (const p of this.positions) {
      const st = this.stands.find((s) => s.id === p.id)!;
      if (st.ageBand !== 'retired') continue;
      const pose = poseAt(st, plans?.get(st.id), monthT);
      dummy.position.set(pose.x + Math.cos(pose.heading) * 0.085, 0.03, pose.z - Math.sin(pose.heading) * 0.085);
      dummy.rotation.set(0.1, pose.heading, 0);
      dummy.scale.setScalar(0.95);
      dummy.updateMatrix();
      this.sticks.setMatrixAt(ns, dummy.matrix);
      this.sticks.setColorAt(ns, col.setHex(0xffffff));
      ns++;
    }
    this.bodies.count = this.heads.count = n;
    this.sticks.count = ns;
    for (const [shape, m] of this.markers) { m.count = counts.get(shape) ?? 0; }
    for (const m of [this.bodies, this.heads, this.sticks, ...this.markers.values()]) {
      m.instanceMatrix.needsUpdate = true;
      if (m.instanceColor) m.instanceColor.needsUpdate = true;
    }
    this.halo.visible = !!haloAt;
    if (haloAt) this.halo.position.set(haloAt.x, haloAt.y, haloAt.z);
  }

  /** Nearest person to a screen point within `radiusPx`, or null. */
  pick(camera: THREE.Camera, w: number, h: number, px: number, py: number, radiusPx = 16): number | null {
    let best: number | null = null, bd = radiusPx;
    const v = new THREE.Vector3();
    for (const p of this.positions) {
      v.set(p.x, p.y, p.z).project(camera);
      const d = Math.hypot((v.x * 0.5 + 0.5) * w - px, (-v.y * 0.5 + 0.5) * h - py);
      if (d < bd) { bd = d; best = p.id; }
    }
    return best;
  }

  get instances() { return this.stands.length * 3 + this.tents.length; }

  dispose() {
    for (const m of [this.bodies, this.heads, this.sticks, ...this.markers.values()]) m.dispose();
    this.geos.forEach((g) => g.dispose());
    this.mats.forEach((m) => m.dispose());
    this.tentModel.geometry.dispose();
  }
}
