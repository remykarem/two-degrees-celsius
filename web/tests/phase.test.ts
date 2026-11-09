import { describe, expect, it } from 'vitest';
import { canTransition, nextPhase, type Phase, type PhaseEvent } from '../src/store';

describe('phase machine', () => {
  it('runs review -> plan -> playback -> results -> review', () => {
    let p: Phase = 'review';
    for (const [e, want] of [['startPlan', 'plan'], ['endTurn', 'playback'], ['playbackDone', 'results'], ['continue', 'review']] as [PhaseEvent, Phase][]) {
      p = nextPhase(p, e);
      expect(p).toBe(want);
    }
  });
  it('allows ending an empty turn straight from review, and cancelling a plan', () => {
    expect(nextPhase('review', 'endTurn')).toBe('playback');
    expect(nextPhase('plan', 'cancelPlan')).toBe('review');
  });
  it('ends the game from results', () => {
    expect(nextPhase('results', 'gameOver')).toBe('ended');
    expect(nextPhase('ended', 'continue')).toBe('ended');
  });
  it('ignores illegal events', () => {
    expect(nextPhase('playback', 'startPlan')).toBe('playback');
    expect(nextPhase('playback', 'endTurn')).toBe('playback');
    expect(nextPhase('results', 'endTurn')).toBe('results');
    expect(canTransition('playback', 'endTurn')).toBe(false);
    expect(canTransition('review', 'startPlan')).toBe(true);
  });
});
