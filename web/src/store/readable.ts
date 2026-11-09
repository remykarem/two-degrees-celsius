/** Minimal observable. `subscribe` follows the Svelte store contract (calls back immediately). */
export interface Readable<T> { get(): T; subscribe(fn: (v: T) => void): () => void }
export interface Writable<T> extends Readable<T> { set(v: T): void; update(fn: (v: T) => T): void }

export function writable<T>(initial: T): Writable<T> {
  let value = initial;
  const subs = new Set<(v: T) => void>();
  const set = (v: T) => {
    if (Object.is(v, value)) return;
    value = v;
    for (const fn of [...subs]) fn(value);
  };
  return {
    get: () => value,
    set,
    update: (fn) => set(fn(value)),
    subscribe(fn) {
      subs.add(fn);
      fn(value);
      return () => subs.delete(fn);
    },
  };
}
