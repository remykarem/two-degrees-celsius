import type { Part } from './models';
import type { EstateTiles } from '../store/scene-estate';
import { centreOf, type Pt } from './coords';

/** Pure layout of the estate in world space: where the runway frame sits and how its parts are moved into the world. No Three.js. */
export interface RunwayFrame {
  /** World position of the hangar tile centre. */
  origin: Pt;
  /** Unit vector along the runway (hangar toward the far end), in world x/z. */
  axis: Pt;
  /** Yaw (radians, three.js rotation.y) that turns local +x onto the axis. */
  yaw: number;
  /** Tiles from the hangar tile centre to the far end's centre. */
  len: number;
}

export function runwayFrame(t: EstateTiles, size: number): RunwayFrame {
  const o = centreOf(t.hangar, size);
  const far = t.runway.length ? t.runway[t.runway.length - 1]! : t.hangar;
  const f = centreOf(far, size);
  const dx = f[0] - o[0], dz = f[1] - o[1];
  const len = Math.hypot(dx, dz);
  const axis: Pt = len > 1e-9 ? [dx / len, dz / len] : [1, 0];
  return { origin: o, axis, yaw: Math.atan2(-axis[1], axis[0]), len };
}

/** Runway frame (u along, v sideways) -> world x/z. */
export const frameToWorld = (f: RunwayFrame, u: number, v: number): Pt => [f.origin[0] + u * f.axis[0] - v * f.axis[1], f.origin[1] + u * f.axis[1] + v * f.axis[0]];

/** Move runway-frame parts into the world (positions rotated, each part turned by the frame's yaw). */
export function placeInFrame(parts: readonly Part[], f: RunwayFrame): Part[] {
  return parts.map((p) => { const [x, z] = frameToWorld(f, p.p[0], p.p[2]); return { ...p, p: [x, p.p[1], z] as [number, number, number], ry: (p.ry ?? 0) + f.yaw }; });
}

/** Move tile-local parts onto a tile. */
export function placeOnTile(parts: readonly Part[], c: Pt): Part[] {
  return parts.map((p) => ({ ...p, p: [p.p[0] + c[0], p.p[1], p.p[2] + c[1]] as [number, number, number] }));
}
