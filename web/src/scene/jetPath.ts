/**
 * Pure flight path of the estate's private jet, in the runway's own frame: `u` runs along the runway from the centre of the hangar tile
 * toward the take-off end, `v` is sideways (positive = away from the hangar), `y` is height. `t` is month time 0..1, so a paused clock
 * freezes the jet. No Three.js, no randomness. Visual constants only (the model owns every real number: the tonnes come from the wire).
 */
export const LANE_V = 0.24;
/** Where the jet parks (in front of the hangar door). */
export const PARK_U = 0.1;
export const FLIGHT = { depart: [0.06, 0.4], back: [0.6, 0.95] } as const;

export type JetPhase = 'parked' | 'roll' | 'climb' | 'approach' | 'rollout' | 'taxi' | 'static' | 'away';
export interface JetPose { visible: boolean; u: number; v: number; y: number; /** Heading in the runway frame: 0 = along +u, PI = back toward the hangar. */ yaw: number; pitch: number; bank: number; phase: JetPhase; /** 0..1 how hard the engines work (exhaust puff). */ thrust: number; airborne: boolean }

const clamp01 = (x: number) => Math.min(1, Math.max(0, x));
const smooth = (x: number) => { const c = clamp01(x); return c * c * (3 - 2 * c); };
const lerp = (a: number, b: number, k: number) => a + (b - a) * k;
const sub = (t: number, [a, b]: readonly [number, number]) => clamp01((t - a) / (b - a));

const parked = (): JetPose => ({ visible: true, u: PARK_U, v: LANE_V, y: 0, yaw: 0, pitch: 0, bank: 0, phase: 'parked', thrust: 0, airborne: false });
const away = (): JetPose => ({ visible: false, u: 0, v: LANE_V, y: 0, yaw: 0, pitch: 0, bank: 0, phase: 'away', thrust: 0, airborne: false });

/** Where the jet is: the departure leg only (take-off roll then a climbing turn out over the meadow). `f` 0..1 over the leg. `len` = tiles from hangar to the far end. */
function departure(f: number, len: number, heading = true): JetPose {
  const uRot = len + 0.15;
  if (f < 0.1) return { ...parked(), thrust: f / 0.1 * 0.6 };
  if (f < 0.55) {
    const k = (f - 0.1) / 0.45;
    return { visible: true, u: lerp(PARK_U, uRot, k * k), v: LANE_V, y: 0, yaw: 0, pitch: 0, bank: 0, phase: 'roll', thrust: 1, airborne: false };
  }
  const k = (f - 0.55) / 0.45;
  const e = smooth(k);
  const ahead = heading ? departure(Math.min(1, f + 0.01), len, false) : null;
  const cur = { u: lerp(uRot, len + 5.5, k), v: lerp(LANE_V, LANE_V + 3.2, e * e) };
  const yaw = ahead ? Math.atan2(ahead.v - cur.v, ahead.u - cur.u) : 0;
  return { visible: k < 1, ...cur, y: 3.0 * Math.pow(k, 1.3) + 0.02, yaw, pitch: 0.32, bank: 0.25 * Math.sin(k * Math.PI), phase: 'climb', thrust: 1, airborne: true };
}

/** The return leg: a descending approach from the east, touchdown at the far end, a roll-out toward the hangar and a turn onto the parking spot. */
function arrival(f: number, len: number, heading = true): JetPose {
  const uTouch = len + 0.2;
  if (f < 0.45) {
    const k = f / 0.45;
    const e = 1 - Math.pow(1 - k, 2);
    const cur = { u: lerp(len + 5.5, uTouch, e), v: lerp(LANE_V + 3.2, LANE_V, Math.pow(e, 0.8)) };
    const ahead = heading ? arrival(Math.min(0.449, f + 0.01), len, false) : null;
    const yaw = ahead && (ahead.u !== cur.u || ahead.v !== cur.v) ? Math.atan2(ahead.v - cur.v, ahead.u - cur.u) : Math.PI;
    return { visible: true, ...cur, y: lerp(2.6, 0.02, Math.pow(k, 1.1)), yaw, pitch: -0.14, bank: -0.2 * (1 - k), phase: 'approach', thrust: 0.2, airborne: true };
  }
  if (f < 0.78) {
    const k = (f - 0.45) / 0.33;
    return { visible: true, u: lerp(uTouch, 0.95, 1 - Math.pow(1 - k, 2)), v: LANE_V, y: 0, yaw: Math.PI, pitch: 0, bank: 0, phase: 'rollout', thrust: 0, airborne: false };
  }
  const k = smooth((f - 0.78) / 0.22);
  return { visible: true, u: lerp(0.95, PARK_U, k), v: LANE_V, y: 0, yaw: Math.PI * (1 - k), pitch: 0, bank: 0, phase: 'taxi', thrust: 0, airborne: false };
}

/**
 * The jet at month time `t` in a month with a flight. `returns` false = the jet is still away at the end of the month (the flight was
 * in the latest month: `estate.jet.flyingThisMonth`). `reduced` = no flight path: it stands on the runway for a moment with a puff.
 */
export function jetPoseAt(t: number, len: number, returns: boolean, reduced = false): JetPose {
  const [d0, d1] = FLIGHT.depart, [b0, b1] = FLIGHT.back;
  if (reduced) {
    if (t >= d0 && t < b0) return { visible: true, u: len * 0.5 + 0.2, v: LANE_V, y: 0, yaw: 0, pitch: 0, bank: 0, phase: 'static', thrust: t < d0 + 0.2 ? 1 : 0, airborne: false };
    return t < d0 || returns ? parked() : away();
  }
  if (t < d0) return parked();
  if (t < d1) return departure(sub(t, FLIGHT.depart), len);
  if (!returns) return away();
  if (t < b0) return away();
  if (t < b1) return arrival(sub(t, FLIGHT.back), len);
  return parked();
}

/** Resting state, with no month being played: parked, or away while the latest month's flight is out (`flyingThisMonth`). */
export const jetRestPose = (flyingThisMonth: boolean): JetPose => (flyingThisMonth ? away() : parked());

export interface TrailPuff { u: number; v: number; y: number; /** 0 = fresh, 1 = gone. */ age: number; /** Fresh exhaust (dark, emissions) versus vapour (white). */ exhaust: boolean }
export const TRAIL = { count: 16, lag: 0.011 } as const;

/** Stateless trail: samples the jet's own path at earlier times (so it is reproducible and freezes when the clock pauses). */
export function jetTrail(t: number, len: number, returns: boolean, reduced = false): TrailPuff[] {
  const out: TrailPuff[] = [];
  if (reduced) {
    const p = jetPoseAt(t, len, returns, true);
    if (p.phase === 'static' && p.thrust > 0) out.push({ u: p.u - 0.45, v: p.v, y: 0.1, age: 0.3, exhaust: true });
    return out;
  }
  for (let i = 1; i <= TRAIL.count; i++) {
    const tt = t - i * TRAIL.lag;
    if (tt < 0) break;
    const p = jetPoseAt(tt, len, returns);
    if (!p.visible || p.thrust < 0.5) continue;
    // Exhaust sits low behind the aircraft on the ground and in the first moments of the climb; vapour is the long white trail up there.
    const age = i / TRAIL.count;
    out.push({ u: p.u - 0.3, v: p.v, y: p.y + 0.06, age, exhaust: !p.airborne || age < 0.35 });
  }
  return out;
}
