import { writable } from './readable';

/**
 * Pressure mode: where the clock is inside the month that is playing, for the scene (which reads it every frame; it is cheap plain data).
 * Written by store/live.ts, read by the scene. `monthT` is 0..1 through the current month and stops moving while the clock is paused.
 */
export interface LiveClockFeed { active: boolean; monthT: number; paused: boolean }
export const liveClock = writable<LiveClockFeed>({ active: false, monthT: 0, paused: false });
