import type { InfraKind } from '../contract';
import type { SceneryKind } from '../store';

/** Placeholder colours and heights. Shape/height (not colour alone) differ per kind. */
export const SCENERY: Record<SceneryKind, { color: number; height: number }> = {
  home: { color: 0xd9a066, height: 0.7 },
  firm: { color: 0x8a8f98, height: 1.1 },
  bank: { color: 0x4f7fb8, height: 1.4 },
  government: { color: 0x9b6bb5, height: 1.6 },
  forest: { color: 0x3f8f4f, height: 0.5 },
  commons: { color: 0x9ac36a, height: 0.15 },
};

export const INFRA: Record<InfraKind, { color: number; height: number }> = {
  public_housing: { color: 0xe8c07a, height: 0.9 },
  fossil_plant: { color: 0x4a4a4a, height: 1.3 },
  renewable_plant: { color: 0x58b8a0, height: 1.0 },
};

export const GROUND = [0xe4e8da, 0xd9dfcd];
export const HOVER_OK = 0x2f9e5b;
export const HOVER_BAD = 0xc0392b;
export const SELECT = 0x1d4ed8;
