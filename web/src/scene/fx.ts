import * as THREE from 'three';
import { CLASS_COLOR, COIN, FOOD, GOODS, mix, SINK, SMOKE, SMOKE_DARK } from './palette';
import { CAPS, CLASS_ORDER, commuteFraction, pointAt, smokeCounts, smokeParticle, trackProgress, type Emitter, type FramePlan, type FxType, type Track } from './flows';

const FX_COLOR: Record<FxType, number> = {
  closed: 0x5b5b66, founded: 0x2f9e5b, hired: 0x0072b2, fired: 0xd55e00, forest: 0x8a6a3a, commons: 0xe69f00, enclosed: 0xa4472f,
  bank: 0xd1432f, class: 0xcc79a7, shortfall: 0xd55e00, default: 0xd1432f,
};
const TOKEN_COLOR = [COIN, 0x56b4e9, 0xe69f00];
const FX_S = 1.4;

const dummy = new THREE.Object3D();

function inst(geo: THREE.BufferGeometry, mat: THREE.Material, cap: number, shadow = false) {
  const m = new THREE.InstancedMesh(geo, mat, cap);
  m.count = 0;
  m.frustumCulled = false;
  m.castShadow = shadow;
  m.setColorAt(0, new THREE.Color(1, 1, 1));
  return m;
}

/** Instanced particles and figures for playback and ambient smoke. Capped pools; nothing is allocated per frame. */
export class FxLayer {
  readonly group = new THREE.Group();
  private figures: THREE.InstancedMesh;
  private goods: THREE.InstancedMesh;
  private tokens: THREE.InstancedMesh;
  private smoke: THREE.InstancedMesh;
  private rings: THREE.InstancedMesh;
  private geos: THREE.BufferGeometry[] = [];
  private mats: THREE.Material[] = [];
  private c = new THREE.Color();
  /** Counts drawn last frame, for the stats readout and tests. */
  drawn = { figures: 0, goods: 0, tokens: 0, smoke: 0, rings: 0 };

  constructor() {
    const body = new THREE.CylinderGeometry(0.05, 0.07, 0.17, 7).translate(0, 0.1, 0);
    const head = new THREE.SphereGeometry(0.055, 7, 5).translate(0, 0.25, 0);
    const fig = mergeTwo(body, head);
    const box = new THREE.BoxGeometry(0.13, 0.11, 0.13).translate(0, 0.07, 0);
    const coin = new THREE.CylinderGeometry(0.07, 0.07, 0.025, 10).rotateX(Math.PI / 2);
    const puff = new THREE.IcosahedronGeometry(0.5, 1);
    const ring = new THREE.RingGeometry(0.28, 0.34, 20).rotateX(-Math.PI / 2);
    this.geos.push(body, head, fig, box, coin, puff, ring);
    const lam = () => { const m = new THREE.MeshLambertMaterial(); this.mats.push(m); return m; };
    const soft = new THREE.MeshLambertMaterial({ transparent: true, opacity: 0.6, depthWrite: false });
    const ringMat = new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.85, depthTest: false, side: THREE.DoubleSide });
    this.mats.push(soft, ringMat);
    this.figures = inst(fig, lam(), CAPS.figures, true);
    this.goods = inst(box, lam(), CAPS.goods, true);
    this.tokens = inst(coin, lam(), CAPS.tokens);
    this.smoke = inst(puff, soft, CAPS.smoke);
    this.rings = inst(ring, ringMat, CAPS.fx);
    this.rings.renderOrder = 15;
    this.group.add(this.figures, this.goods, this.tokens, this.smoke, this.rings);
  }

  /** Total live instances (for the stats readout). */
  get instances() { const d = this.drawn; return d.figures + d.goods + d.tokens + d.smoke + d.rings; }

  /**
   * Draw one instant. `plan` is the current month's plan (or null), `monthT` its local time 0..1, `sec` wall seconds (smoke),
   * `ambient` emitters used when there is no plan. `loop` are looping tracks (money-flow lens), with `loopT` 0..1.
   */
  update(plan: FramePlan | null, monthT: number, sec: number, ambient: Emitter[], loop: Track[], loopT: number, boostSmoke: boolean) {
    const tracks = plan ? plan.tracks : [];
    let nf = 0, ng = 0, nt = 0;
    const put = (tr: Track, t: number) => {
      const u = trackProgress(tr, t);
      if (u === null) return;
      if (tr.kind === 'figure') {
        if (nf >= CAPS.figures) return;
        const f = commuteFraction(u), f2 = commuteFraction(Math.min(1, u + 0.01));
        const [x, z] = pointAt(tr.path, f), [x2, z2] = pointAt(tr.path, f2);
        const sc = [1.25, 1.1, 1.0, 0.9, 0.72][tr.style] ?? 1;
        dummy.position.set(x, 0.07 + Math.abs(Math.sin(u * 60)) * 0.025, z);
        dummy.rotation.set(0, Math.atan2(x2 - x, z2 - z), 0);
        dummy.scale.setScalar(sc);
        dummy.updateMatrix();
        this.figures.setMatrixAt(nf, dummy.matrix);
        this.figures.setColorAt(nf, this.c.setHex(CLASS_COLOR[CLASS_ORDER[tr.style] ?? 'worker']));
        nf++;
      } else if (tr.kind === 'good') {
        if (ng >= CAPS.goods) return;
        const e = u * u * (3 - 2 * u);
        const [x, z] = pointAt(tr.path, e);
        dummy.position.set(x, 0.07 + Math.sin(u * Math.PI) * 0.12, z);
        dummy.rotation.set(0, u * 6, 0);
        dummy.scale.setScalar(tr.style === 1 ? 0.9 : 1);
        dummy.updateMatrix();
        this.goods.setMatrixAt(ng, dummy.matrix);
        this.goods.setColorAt(ng, this.c.setHex(tr.style === 1 ? FOOD : GOODS));
        ng++;
      } else {
        if (nt >= CAPS.tokens) return;
        const [x, z] = pointAt(tr.path, u);
        dummy.position.set(x, 0.55 + Math.sin(u * Math.PI) * 0.35, z);
        dummy.rotation.set(0, u * 12, 0);
        dummy.scale.setScalar(1.5);
        dummy.updateMatrix();
        this.tokens.setMatrixAt(nt, dummy.matrix);
        this.tokens.setColorAt(nt, this.c.setHex(TOKEN_COLOR[tr.style] ?? COIN));
        nt++;
      }
    };
    for (const tr of tracks) put(tr, monthT);
    for (const tr of loop) put(tr, loopT);
    this.figures.count = nf; this.goods.count = ng; this.tokens.count = nt;

    const em = plan ? plan.emitters : ambient;
    const counts = smokeCounts(em, boostSmoke ? CAPS.smoke : Math.floor(CAPS.smoke * 0.6));
    let ns = 0;
    em.forEach((e, ei) => {
      for (let i = 0; i < counts[ei]!; i++) {
        if (ns >= CAPS.smoke) return;
        const p = smokeParticle(e, i, sec);
        dummy.position.set(p.x, p.y, p.z);
        dummy.rotation.set(i, i * 2, 0);
        dummy.scale.setScalar(p.scale * (boostSmoke ? 1.25 : 1));
        dummy.updateMatrix();
        this.smoke.setMatrixAt(ns, dummy.matrix);
        const col = e.kind === 'sink' ? SINK : e.kind === 'house' ? SMOKE : mix(SMOKE, SMOKE_DARK, Math.min(1, e.rate * 0.9 + p.life * 0.2));
        this.smoke.setColorAt(ns, this.c.setHex(col));
        ns++;
      }
    });
    this.smoke.count = ns;

    let nr = 0;
    if (plan) {
      for (const fx of plan.fx) {
        const lt = (monthT - fx.t) / (FX_S / 2.2);
        if (lt < 0 || lt > 1) continue;
        const s = 0.8 + lt * 2.2;
        dummy.position.set(fx.at[0], 0.12 + lt * 0.15, fx.at[1]);
        dummy.rotation.set(0, 0, 0);
        dummy.scale.set(s, s, s);
        dummy.updateMatrix();
        this.rings.setMatrixAt(nr, dummy.matrix);
        this.rings.setColorAt(nr, this.c.setHex(FX_COLOR[fx.type]));
        nr++;
      }
    }
    this.rings.count = nr;
    for (const m of [this.figures, this.goods, this.tokens, this.smoke, this.rings]) {
      m.instanceMatrix.needsUpdate = true;
      if (m.instanceColor) m.instanceColor.needsUpdate = true;
    }
    this.drawn = { figures: nf, goods: ng, tokens: nt, smoke: ns, rings: nr };
  }

  dispose() {
    for (const m of [this.figures, this.goods, this.tokens, this.smoke, this.rings]) m.dispose();
    this.geos.forEach((g) => g.dispose());
    this.mats.forEach((m) => m.dispose());
  }
}

function mergeTwo(a: THREE.BufferGeometry, b: THREE.BufferGeometry): THREE.BufferGeometry {
  const pa = a.toNonIndexed(), pb = b.toNonIndexed();
  const g = new THREE.BufferGeometry();
  for (const name of ['position', 'normal']) {
    const x = pa.getAttribute(name)!.array as Float32Array, y = pb.getAttribute(name)!.array as Float32Array;
    const arr = new Float32Array(x.length + y.length);
    arr.set(x); arr.set(y, x.length);
    g.setAttribute(name, new THREE.BufferAttribute(arr, 3));
  }
  pa.dispose(); pb.dispose();
  return g;
}
