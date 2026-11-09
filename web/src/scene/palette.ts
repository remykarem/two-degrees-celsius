import type { ClassKey } from '../contract';

/**
 * Light, warm palette. Class colours are the Okabe-Ito colour-blind-safe set; every class also has its own
 * building silhouette and rooftop marker (models.ts), so colour is never the only cue.
 */
export const CLASS_COLOR: Record<ClassKey, number> = {
  capitalist: 0xcc79a7, // reddish purple
  rentier: 0xe69f00, // orange
  manager: 0x56b4e9, // sky blue
  worker: 0x009e73, // bluish green
  dependent: 0xf0e442, // yellow
};
export const CLASS_LABEL: Record<ClassKey, string> = {
  capitalist: 'Owners (villa, diamond marker)', rentier: 'Landlords (terrace, cube marker)', manager: 'Managers (chimney house, ball marker)',
  worker: 'Workers (gable house, cone marker)', dependent: 'Dependents (cottage, no marker)',
};
export const CLASS_SHORT: Record<ClassKey, string> = {
  capitalist: 'Owners', rentier: 'Landlords', manager: 'Managers', worker: 'Workers', dependent: 'Dependents',
};

export const WALL = 0xf3e8d4;
export const ROOF = 0xc9654a;
export const ROOF_DARK = 0x8c5a4a;
export const WOOD = 0xa9774f;
export const STONE = 0xd8d2c6;
export const METAL = 0x8b929c;
export const GLASS = 0x7fb0c9;
export const DARK = 0x4b4f58;
export const BRICK = 0xb5573f;
export const GOLD = 0xf2c14e;
export const LEAF = 0x5fa45b;
export const LEAF_DRY = 0xb89a52;
export const GRASS = 0xa8cf86;
export const PANEL = 0x2d4f7a;

export const GROUND = [0xa8d37c, 0x9fcb72];
/** Dirt footpaths where there is no road. */
export const PATH = 0xd9c593;
/** Real (built) roads: darker than the footpaths. */
export const ASPHALT = 0x6d7078;
export const SOIL = 0x9b7653;
export const SOIL_DARK = 0x7f6040;
export const BACKGROUND = 0xf6f0e3;
export const FAR_GRASS = 0x8bbf68;
export const SKY_TOP = '#8ec9ee';
export const SKY_MID = '#cfe8f3';
export const SKY_LOW = '#fbf3dc';
export const ROAD_LINE = 0xf2efe2;
export const HEDGE = 0x4f9250;
export const AWNING_A = 0xd1432f;
export const EDGE = 0x5a4636;
export const HOVER_OK = 0x2f9e5b;
export const HOVER_BAD = 0xd1432f;
export const SELECT = 0x1d4ed8;
export const SMOKE = 0xcfcfd2;
export const SMOKE_DARK = 0x7a7a82;
export const SINK = 0x6dbb6a;
export const COIN = 0xf2c14e;
export const GOODS = 0xc58a52;
export const FOOD = 0xd6603f;
export const SERVICE = 0x9b6fd0;
export const CRANE = 0xf0b429;

/** Base wall tints (lens "none"): class colour washed toward cream so the town stays calm and warm. */
export const mix = (a: number, b: number, t: number): number => {
  const ch = (s: number) => Math.round(((a >> s) & 255) * (1 - t) + ((b >> s) & 255) * t);
  return (ch(16) << 16) | (ch(8) << 8) | ch(0);
};
export const homeTint = (c: ClassKey) => mix(CLASS_COLOR[c], WALL, 0.62);
export const hex = (n: number) => `#${n.toString(16).padStart(6, '0')}`;
