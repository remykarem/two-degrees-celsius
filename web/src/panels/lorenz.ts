export interface LorenzPoint { populationShare: number; wealthShare: number }

/** SVG path for the Lorenz curve in a w x h box (origin bottom-left), plus the line of equality. Points are the wire's. */
export function lorenzPath(points: LorenzPoint[], w: number, h: number, pad = 0): { curve: string; equality: string; area: string } {
  const x = (v: number) => (pad + v * (w - 2 * pad)).toFixed(1);
  const y = (v: number) => (h - pad - v * (h - 2 * pad)).toFixed(1);
  const pts = points.length ? points : [{ populationShare: 0, wealthShare: 0 }, { populationShare: 1, wealthShare: 1 }];
  const curve = pts.map((p, i) => `${i ? 'L' : 'M'}${x(p.populationShare)} ${y(p.wealthShare)}`).join(' ');
  return { curve, equality: `M${x(0)} ${y(0)} L${x(1)} ${y(1)}`, area: `${curve} L${x(1)} ${y(1)} L${x(0)} ${y(0)} Z` };
}
