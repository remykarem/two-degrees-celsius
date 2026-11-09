import * as THREE from 'three';
import { clampPan, clampZoom, rotateDelta, smoothFactor, stepTween, ROTATE_MS, ZOOM_MAX, ZOOM_MIN, type AngleTween } from './cameraMath';

const OFFSET = new THREE.Vector3(14, 12, 14);
const HALF_H = 7.4;
const UP = new THREE.Vector3(0, 1, 0);
const GROUND = new THREE.Plane(UP, 0);

/**
 * Orthographic camera: 90 degree tweened orbit, wheel/pinch zoom about the cursor, drag pan with clamped bounds.
 * `reducedMotion` makes every change instant.
 */
export function createRig(size: number, reducedMotion: () => boolean) {
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 100);
  let view = 0; // 0..3, as in the store
  let angleIdx = 0; // unwrapped quarter turns
  let angle = 0;
  let tween: AngleTween | null = null;
  let zoom = 1, zoomGoal = 1;
  let tx = 0, tz = 0, gx = 0, gz = 0;
  let anchor: { x: number; y: number } | null = null;
  const ray = new THREE.Raycaster();
  const tmp = new THREE.Vector3();

  function place() {
    camera.position.copy(OFFSET).applyAxisAngle(UP, angle).add(tmp.set(tx, 0, tz));
    camera.zoom = zoom;
    camera.lookAt(tx, 0, tz);
    camera.updateProjectionMatrix();
    camera.updateMatrixWorld();
  }
  function groundAt(nx: number, ny: number, out = new THREE.Vector3()): THREE.Vector3 | null {
    ray.setFromCamera(new THREE.Vector2(nx, ny), camera);
    return ray.ray.intersectPlane(GROUND, out);
  }

  const rig = {
    camera,
    get view() { return view; },
    get zoom() { return zoom; },
    get zoomGoal() { return zoomGoal; },
    get panned() { return Math.abs(gx) > 0.05 || Math.abs(gz) > 0.05; },
    resize(w: number, h: number) {
      const aspect = w / Math.max(1, h);
      camera.left = -HALF_H * aspect; camera.right = HALF_H * aspect; camera.top = HALF_H; camera.bottom = -HALF_H;
      place();
    },
    /** Select one of the four views (0..3) by the shortest way round. */
    setView(v: number) {
      const d = rotateDelta(view, v);
      view = ((v % 4) + 4) % 4;
      if (d === 0) return;
      angleIdx += d;
      const to = angleIdx * (Math.PI / 2);
      if (reducedMotion()) { angle = to; tween = null; place(); }
      else tween = { from: angle, to, elapsed: 0, duration: ROTATE_MS };
    },
    /** Zoom by a factor (wheel, pinch, buttons), keeping the point under (nx, ny) fixed when given. */
    zoomBy(factor: number, nx?: number, ny?: number) {
      zoomGoal = clampZoom(zoomGoal * factor);
      anchor = nx === undefined || ny === undefined ? null : { x: nx, y: ny };
      if (reducedMotion()) rig.tick(1000);
    },
    /** Pan by dragging from screen point a to b (NDC). The ground point under the finger stays under the finger. */
    pan(ax: number, ay: number, bx: number, by: number) {
      const p0 = groundAt(ax, ay), p1 = groundAt(bx, by);
      if (!p0 || !p1) return;
      tx += p0.x - p1.x; tz += p0.z - p1.z;
      [tx, tz] = clampPan(tx, tz, size, zoom);
      gx = tx; gz = tz;
      place();
    },
    reset() {
      zoomGoal = 1; gx = 0; gz = 0; anchor = null;
      if (reducedMotion()) { zoom = 1; tx = 0; tz = 0; place(); }
    },
    groundAt,
    /** Advance by dtMs. Returns true while anything is still moving. */
    tick(dtMs: number): boolean {
      let moving = false;
      if (tween) {
        const r = stepTween(tween, dtMs);
        angle = r.value;
        tween = r.done ? null : r.tween;
        moving = !r.done;
      }
      const k = smoothFactor(dtMs);
      const before = anchor ? groundAt(anchor.x, anchor.y) : null;
      if (Math.abs(zoomGoal - zoom) > 0.0005) { zoom = Math.abs(zoomGoal - zoom) < 0.003 ? zoomGoal : zoom + (zoomGoal - zoom) * k; moving = true; }
      else zoom = zoomGoal;
      place();
      if (before && anchor) {
        const after = groundAt(anchor.x, anchor.y);
        if (after) { tx += before.x - after.x; tz += before.z - after.z; gx = tx; gz = tz; }
      }
      // Recentre toward the pan goal (reset) and always keep inside bounds.
      if (Math.abs(gx - tx) > 0.002 || Math.abs(gz - tz) > 0.002) { tx += (gx - tx) * k; tz += (gz - tz) * k; moving = true; }
      [tx, tz] = clampPan(tx, tz, size, zoom);
      place();
      return moving;
    },
    snapNow() { angle = angleIdx * (Math.PI / 2); tween = null; zoom = zoomGoal; tx = gx; tz = gz; place(); },
    limits: { min: ZOOM_MIN, max: ZOOM_MAX },
  };
  return rig;
}
export type Rig = ReturnType<typeof createRig>;
