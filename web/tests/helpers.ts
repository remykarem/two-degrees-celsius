import type { AccountRef, LedgerPage, Result } from '../src/contract';

/** Unwraps a port Result in a test: a failed result fails the test with the sim's message. */
export function ok<T>(r: Result<T>): T {
  if (!r.ok) throw new Error(`${r.error.code}: ${r.error.message}`);
  return r.value;
}

/** How an entry moves [balance, debt] of `me`, written as a table by posting (independent of the viewer's `entryEffect`). */
function effectOn(e: LedgerPage['entries'][number], me: string): [number, number] {
  const k = (a: AccountRef) => JSON.stringify(a);
  const from = k(e.from) === me, to = k(e.to) === me, a = e.amount;
  const unpaid = e.detail === 'unpaid interest added to debt';
  switch (true) {
    case e.kind === 'loan_new': return to ? [a, a] : [0, a]; // credit creation: borrower balance and debt, bank loans only
    case e.kind === 'loan_repay': return from ? [-a, -a] : [0, -a];
    case e.kind === 'loan_writeoff': return from ? [-a, -a] : to ? [0, -a] : [0, 0];
    case e.kind === 'bailout' || unpaid: return from ? [0, a] : to ? [a, a] : [0, 0];
    default: return to ? [a, 0] : from ? [-a, 0] : [0, 0];
  }
}
/**
 * The reconciliation rule, written independently of the viewer and the mock, for balance AND debt: opening + roll-up `net`/`debtNet` + each entry's effect = closing,
 * and every entry's balanceAfter/debtAfter is the running position. The real sim's pages must pass the same rule.
 */
export function reconciles(p: LedgerPage): boolean {
  const me = JSON.stringify(p.account);
  let bal = p.openingBalance + p.summary.reduce((a, r) => a + r.net, 0);
  let debt = p.openingDebt + p.summary.reduce((a, r) => a + r.debtNet, 0);
  for (const e of p.entries) {
    const [db, dd] = effectOn(e, me);
    bal += db; debt += dd;
    if (Math.abs(bal - e.balanceAfter) > 0.011 || Math.abs(debt - e.debtAfter) > 0.011) return false;
  }
  return Math.abs(bal - p.closingBalance) < 0.011 && Math.abs(debt - p.closingDebt) < 0.011;
}
