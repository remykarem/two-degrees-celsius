import type { Result } from '../src/contract';

/** Unwraps a port Result in a test: a failed result fails the test with the sim's message. */
export function ok<T>(r: Result<T>): T {
  if (!r.ok) throw new Error(`${r.error.code}: ${r.error.message}`);
  return r.value;
}
