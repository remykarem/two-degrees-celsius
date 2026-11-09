export type Phase = 'review' | 'plan' | 'playback' | 'results' | 'ended';
export type PhaseEvent = 'startPlan' | 'cancelPlan' | 'endTurn' | 'playbackDone' | 'continue' | 'gameOver';

const TABLE: Record<Phase, Partial<Record<PhaseEvent, Phase>>> = {
  review: { startPlan: 'plan', endTurn: 'playback' },
  plan: { cancelPlan: 'review', endTurn: 'playback' },
  playback: { playbackDone: 'results' },
  results: { continue: 'review', gameOver: 'ended' },
  ended: {},
};

/** Pure transition. Returns the same phase when the event is not allowed. */
export function nextPhase(phase: Phase, event: PhaseEvent): Phase {
  return TABLE[phase][event] ?? phase;
}

export const canTransition = (phase: Phase, event: PhaseEvent) => event in TABLE[phase];
