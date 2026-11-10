import type { AccountRef, LedgerPage, Result } from '../src/contract';

/** Unwraps a port Result in a test: a failed result fails the test with the sim's message. */
export function ok<T>(r: Result<T>): T {
  if (!r.ok) throw new Error(`${r.error.code}: ${r.error.message}`);
  return r.value;
}

/** The reconciliation rule, written independently of the viewer and the mock: opening + detail entries + roll-ups = closing. The real sim's pages must pass the same rule. */
export function reconciles(p: LedgerPage): boolean {
  const k = (a: AccountRef) => JSON.stringify(a);
  const me = k(p.account);
  const direct = p.entries.reduce((a, e) => a + (k(e.to) === me ? e.amount : k(e.from) === me ? -e.amount : 0), 0);
  const rolled = p.summary.reduce((a, r) => a + r.inflow - r.outflow, 0);
  return Math.abs(p.openingBalance + direct + rolled - p.closingBalance) < 0.01;
}
