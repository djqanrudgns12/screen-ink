<script>
  import { keycaps } from '@screen-ink/client';

  /**
   * @type {{
   *   label?: string,
   *   keys?: string[],
   *   combo?: string,
   *   platform?: 'windows' | 'macos',
   *   variant?: 'default' | 'focus' | 'waiting' | 'dim' | 'island' | 'hover',
   *   class?: string,
   * }}
   */
  let {
    label = '',
    keys = undefined,
    combo = '',
    platform = 'windows',
    variant = 'default',
    class: className = '',
  } = $props();

  let items = $derived(
    Array.isArray(keys) && keys.length
      ? keys
      : combo
        ? keycaps(combo, platform)
        : label
          ? [label]
          : [],
  );
</script>

{#if items.length > 0}
  <span class="ink-keycaps {className}" data-variant={variant}>
    {#each items as k, i (i)}
      <kbd class="ink-kbd ink-mono" data-variant={variant}>{k}</kbd>
    {/each}
  </span>
{/if}

<style>
  .ink-keycaps {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex: none;
  }

  .ink-kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 19px;
    padding: 0 5px;
    box-sizing: border-box;
    border-radius: var(--ink-r-key);
    font-size: 10.5px;
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    color: var(--ink-text-2);
    background: var(--ink-key-bg);
    border: 1px solid var(--ink-key-border);
  }

  .ink-kbd[data-variant='hover'] {
    background: var(--ink-key-hover-bg);
  }

  .ink-kbd[data-variant='focus'] {
    color: var(--ink-key-on-invert-text);
    background: var(--ink-key-on-invert-bg);
    border-color: var(--ink-key-on-invert-border);
  }

  .ink-kbd[data-variant='waiting'] {
    color: var(--ink-accent-text);
    background: var(--ink-surface);
    border-color: var(--ink-attn);
    animation: ink-breathe 1.4s ease-in-out infinite;
  }

  .ink-kbd[data-variant='dim'] {
    color: var(--ink-muted);
    background: transparent;
    border-color: var(--ink-line-soft);
  }

  .ink-kbd[data-variant='island'] {
    color: var(--ink-island-text-2);
    background: var(--ink-island-key-bg);
    border-color: transparent;
    border-radius: 5px;
    height: 18px;
    padding: 0 6px;
  }
</style>
