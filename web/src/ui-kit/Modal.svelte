<script lang="ts" module>
  /** Open modals, topmost last: only the topmost one reacts to the keyboard. */
  const stack: symbol[] = [];
</script>

<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  let { title, onclose, size = 'md', dismissible = true, children, footer }: {
    title: string; onclose?: () => void; size?: 'sm' | 'md' | 'lg'; dismissible?: boolean;
    children: Snippet; footer?: Snippet;
  } = $props();

  let dialog: HTMLDivElement;
  const uid = Math.random().toString(36).slice(2, 8);
  const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

  const me = Symbol('modal');
  onMount(() => {
    stack.push(me);
    const previous = document.activeElement as HTMLElement | null;
    const first = dialog.querySelector<HTMLElement>('[data-autofocus]') ?? dialog.querySelector<HTMLElement>(FOCUSABLE) ?? dialog;
    first.focus();
    return () => { stack.splice(stack.indexOf(me), 1); previous?.focus?.(); };
  });

  function onkeydown(e: KeyboardEvent) {
    if (stack[stack.length - 1] !== me) return;
    if (e.key === 'Escape' && dismissible && onclose) {
      e.stopPropagation();
      onclose();
    } else if (e.key === 'Tab') {
      const items = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.offsetParent !== null || el === document.activeElement);
      if (!items.length) { e.preventDefault(); return; }
      const a = items[0]!, z = items[items.length - 1]!;
      if (e.shiftKey && (document.activeElement === a || !dialog.contains(document.activeElement))) { e.preventDefault(); z.focus(); }
      else if (!e.shiftKey && (document.activeElement === z || !dialog.contains(document.activeElement))) { e.preventDefault(); a.focus(); }
    }
  }
</script>

<svelte:window {onkeydown} />
<div class="scrim" role="presentation">
  <div class="dialog {size}" role="dialog" aria-modal="true" aria-labelledby="modal-t-{uid}" tabindex="-1" bind:this={dialog}>
    <header>
      <h2 id="modal-t-{uid}">{title}</h2>
      {#if dismissible && onclose}<button type="button" class="close" aria-label="Close" onclick={onclose}>✕</button>{/if}
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 100; background: var(--scrim); display: grid; place-items: center; padding: 12px; }
  .dialog { background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); box-shadow: var(--shadow);
    display: flex; flex-direction: column; max-height: calc(100vh - 24px); width: 100%; }
  .sm { max-width: 420px; } .md { max-width: 640px; } .lg { max-width: 980px; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 10px 16px; border-bottom: 1px solid var(--border); }
  header h2 { font-size: 15px; text-transform: none; letter-spacing: 0; color: var(--text); }
  .close { border: 0; background: transparent; color: var(--text); cursor: pointer; font-size: 16px; padding: 2px 8px; border-radius: var(--radius); }
  .body { padding: 12px 16px; overflow: auto; }
  footer { padding: 10px 16px; border-top: 1px solid var(--border); display: flex; gap: 8px; justify-content: flex-end; }
</style>
