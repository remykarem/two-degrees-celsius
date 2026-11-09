export interface CoachStep { title: string; body: string; /** data-coach attribute of the element to highlight. */ target: string | null; openDrawer?: boolean }

export const COACH_STEPS: CoachStep[] = [
  { title: 'The carbon budget', target: 'budget',
    body: 'This is how much carbon the town can still emit. Every quarter uses some. If it reaches zero, the game ends.' },
  { title: 'Political capital (PC)', target: 'pc',
    body: 'Changing policy or building costs PC. It regenerates each turn, faster when the government is trusted. Spend it where it matters.' },
  { title: 'Draft a policy', target: 'policies', openDrawer: true,
    body: 'Open Policies and move a slider. You get a draft with its PC cost, how many turns until it takes effect, and what it should push up or down. Lobbying can weaken some policies, and the draft tells you so.' },
  { title: 'Build something', target: 'build',
    body: 'Pick a building, then click a free tile on the map. Buildings cost PC and money and take a few turns to finish.' },
  { title: 'End the turn', target: 'endturn',
    body: 'Review your plan below, then End turn. The quarter plays out, and you get a results screen explaining what changed and why. An empty plan is fine.' },
];
