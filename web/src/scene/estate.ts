import * as THREE from 'three';
import type { EstateTiles } from '../store/scene-estate';
import { centreOf } from './coords';
import { ESTATE_DRIVE, ESTATE_GARDEN, ESTATE_POOL, HANGAR, JET, runwayModel, WINDSOCK, WINDSOCK_POLE } from './estateModels';
import { frameToWorld, placeInFrame, placeOnTile, runwayFrame, type RunwayFrame } from './estateLayout';
import { makeTintMaterial, mergeParts } from './geometry';
import type { JetPose, TrailPuff } from './jetPath';
import { LANE_V } from './jetPath';
import type { Part } from './models';
import { mix, SMOKE, SMOKE_DARK } from './palette';

const dummy = new THREE.Object3D();
const col = new THREE.Color();
const TRAIL_CAP = 24;

/** What the frame loop tells the layer about the jet each frame. */
export interface JetDraw { pose: JetPose; trail: TrailPuff[]; /** 0..1 from the aviation emissions of the month: how dark the exhaust is. */ weight: number; /** Emissions lens: the trail takes the lens colour. */ lens: boolean; sec: number; still: boolean }

/** The estate's fixed scenery (grounds, runway, hangar, windsock), the private jet and its vapour trail. Static parts are merged into one mesh. */
export class EstateLayer {
  readonly group = new THREE.Group();
  private statics: THREE.Mesh | null = null;
  private jet: THREE.Mesh;
  private sock: THREE.Mesh;
  private pole: THREE.Mesh;
  private trail: THREE.InstancedMesh;
  private mat = makeTintMaterial(0xffffff);
  private trailMat = new THREE.MeshLambertMaterial({ transparent: true, opacity: 0.62, depthWrite: false });
  private geos: THREE.BufferGeometry[] = [];
  private frame: RunwayFrame | null = null;
  /** Tiles from the hangar to the runway's far end (0 without an estate). */
  get runLen() { return this.frame?.len ?? 0; }
  /** Instances drawn last frame (stats). */
  drawn = 0;
  private key = '';

  constructor() {
    this.jet = new THREE.Mesh(mergeParts(JET.parts), this.mat);
    this.jet.castShadow = true;
    this.jet.visible = false;
    this.sock = new THREE.Mesh(mergeParts(WINDSOCK.parts), this.mat);
    this.pole = new THREE.Mesh(mergeParts(WINDSOCK_POLE.parts), this.mat);
    const puff = new THREE.IcosahedronGeometry(0.5, 1);
    this.geos.push(this.jet.geometry, this.sock.geometry, this.pole.geometry, puff);
    this.trail = new THREE.InstancedMesh(puff, this.trailMat, TRAIL_CAP);
    this.trail.count = 0;
    this.trail.frustumCulled = false;
    this.trail.setColorAt(0, col.setHex(0xffffff));
    this.sock.visible = this.pole.visible = false;
    this.group.add(this.jet, this.sock, this.pole, this.trail);
  }

  /** (Re)build the fixed scenery for an estate, or clear it (null). */
  setEstate(t: EstateTiles | null, size: number) {
    const key = t ? JSON.stringify([t.roles, t.runway, t.hangar, size]) : '';
    if (key === this.key) return;
    this.key = key;
    if (this.statics) { this.group.remove(this.statics); this.statics.geometry.dispose(); this.statics = null; }
    this.frame = t ? runwayFrame(t, size) : null;
    const show = !!t;
    this.pole.visible = this.sock.visible = show;
    if (!t || !this.frame) { this.jet.visible = false; return; }
    const f = this.frame;
    const parts: Part[] = [];
    const tile = (m: { parts: Part[] }, at: { x: number; y: number } | undefined) => { if (at) parts.push(...placeOnTile(m.parts, centreOf(at, size))); };
    tile(ESTATE_DRIVE, t.roles.drive); tile(ESTATE_POOL, t.roles.pool); tile(ESTATE_GARDEN, t.roles.garden);
    parts.push(...placeInFrame(HANGAR.parts, f), ...placeInFrame(runwayModel(Math.max(1, Math.round(f.len))).parts, f));
    const g = mergeParts(parts);
    this.statics = new THREE.Mesh(g, this.mat);
    this.statics.receiveShadow = true;
    this.statics.castShadow = true;
    this.group.add(this.statics);
    // The windsock stands on the north side of the far end, a tile edge past the last runway tile's centre.
    const [px, pz] = frameToWorld(f, f.len + 0.38, LANE_V - 0.58);
    this.pole.position.set(px, 0, pz);
    this.sock.position.set(px, 0.36, pz);
    this.pole.rotation.y = this.sock.rotation.y = f.yaw;
  }

  /** Draw the jet and its trail for this frame. */
  update(d: JetDraw | null) {
    if (!this.frame) { this.drawn = 0; return; }
    const f = this.frame;
    // Windsock: a slow sway (still when the user asks for reduced motion).
    this.sock.rotation.y = f.yaw + Math.PI + (d?.still ? 0 : Math.sin((d?.sec ?? 0) * 1.3) * 0.25);
    this.sock.rotation.z = -0.35 + (d?.still ? 0 : Math.sin((d?.sec ?? 0) * 2.1) * 0.08);
    if (!d || !d.pose.visible) { this.jet.visible = false; this.trail.count = 0; this.drawn = 0; return; }
    const p = d.pose;
    const [x, z] = frameToWorld(f, p.u, p.v);
    // Heading in the runway frame (yaw 0 = +u) -> world: direction (cos, sin) in (u, v).
    const dir = frameToWorld({ ...f, origin: [0, 0] }, Math.cos(p.yaw), Math.sin(p.yaw));
    this.jet.visible = true;
    this.jet.position.set(x, 0.01 + p.y, z);
    this.jet.rotation.set(0, Math.atan2(-dir[1], dir[0]), 0, 'YXZ');
    this.jet.rotation.z = p.pitch;
    this.jet.rotation.x = p.bank;
    let n = 0;
    for (const t of d.trail) {
      if (n >= TRAIL_CAP) break;
      const [tx, tz] = frameToWorld(f, t.u, t.v);
      const grow = Math.min(1, t.age * 4);
      const fade = 1 - t.age;
      dummy.position.set(tx, 0.02 + t.y, tz);
      dummy.rotation.set(n, n * 2, 0);
      dummy.scale.setScalar(Math.max(0.01, (t.exhaust ? 0.1 : 0.07) * (0.5 + grow * 1.6) * fade));
      dummy.updateMatrix();
      this.trail.setMatrixAt(n, dummy.matrix);
      const dark = t.exhaust ? mix(SMOKE, SMOKE_DARK, d.weight) : 0xffffff;
      this.trail.setColorAt(n, col.setHex(d.lens ? mix(dark, 0xd55e00, 0.55 * d.weight) : dark));
      n++;
    }
    this.trail.count = n;
    this.trail.instanceMatrix.needsUpdate = true;
    if (this.trail.instanceColor) this.trail.instanceColor.needsUpdate = true;
    this.drawn = n + 1;
  }

  dispose() {
    this.trail.dispose();
    this.statics?.geometry.dispose();
    this.geos.forEach((g) => g.dispose());
    this.mat.dispose();
    this.trailMat.dispose();
  }
}
