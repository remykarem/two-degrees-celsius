/** Pure camera maths (no Three.js), unit tested. */
export const ZOOM_MIN = 0.7;
export const ZOOM_MAX = 3.2;
export const ROTATE_MS = 480;

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
export const clampZoom = (z: number) => clamp(z, ZOOM_MIN, ZOOM_MAX);

/** Smooth ease in/out for the 90 degree rotation tween. */
export const easeInOut = (t: number) => { const c = clamp(t, 0, 1); return c * c * (3 - 2 * c); };

/** Shortest signed number of quarter turns from view `from` to view `to` (0..3). A half turn goes clockwise (+2). */
export function rotateDelta(from: number, to: number): -1 | 0 | 1 | 2 {
  const d = (((to - from) % 4) + 4) % 4;
  return d === 3 ? -1 : (d as 0 | 1 | 2);
}

/** How far from the map centre the camera target may travel. Zoomed out: almost nothing; zoomed in: most of the map. */
export function panLimit(size: number, zoom: number): number {
  return (size / 2) * Math.max(0, 1 - 1 / Math.max(zoom, 1)) + 1.2;
}
export function clampPan(x: number, z: number, size: number, zoom: number): [number, number] {
  const l = panLimit(size, zoom);
  return [clamp(x, -l, l), clamp(z, -l, l)];
}

/** Exponential smoothing step: fraction of the remaining distance covered in dtMs with the given time constant. */
export const smoothFactor = (dtMs: number, tauMs = 90) => 1 - Math.exp(-Math.max(0, dtMs) / tauMs);

/** Wheel delta to a zoom multiplier (deltaMode 1 = lines). Positive delta zooms out. */
export function wheelFactor(deltaY: number, deltaMode = 0): number {
  const px = deltaMode === 1 ? deltaY * 16 : deltaMode === 2 ? deltaY * 400 : deltaY;
  return Math.exp(-clamp(px, -400, 400) * 0.0015);
}

/** Time-based tween of the orbit angle. */
export interface AngleTween { from: number; to: number; elapsed: number; duration: number }
export function stepTween(t: AngleTween, dtMs: number): { value: number; done: boolean; tween: AngleTween } {
  const elapsed = t.elapsed + dtMs;
  const done = t.duration <= 0 || elapsed >= t.duration;
  const value = done ? t.to : t.from + (t.to - t.from) * easeInOut(elapsed / t.duration);
  return { value, done, tween: { ...t, elapsed } };
}
