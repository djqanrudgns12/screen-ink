<script>
  import { ICONS } from './icons.js';

  /**
   * @type {{
   *   name: string,
   *   size?: 16 | 20 | 24,
   *   accent?: boolean,
   *   inverted?: boolean,
   *   label?: string,
   *   class?: string,
   * }}
   */
  let {
    name,
    size = 16,
    accent = true,
    inverted = false,
    label = '',
    class: className = '',
  } = $props();

  let def = $derived(ICONS[name] ?? null);
  let strokeWidth = $derived(size <= 16 ? 1.85 : 1.5);
  let accentFill = $derived(inverted ? 'var(--ink-accent)' : 'var(--ink-accent-icon)');
</script>

{#if def}
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width={strokeWidth}
    stroke-linecap="round"
    stroke-linejoin="round"
    role={label ? 'img' : undefined}
    aria-label={label || undefined}
    aria-hidden={label ? undefined : 'true'}
    class="ink-icon {className}"
  >
    {#each def.shapes as shape, i (i)}
      {#if shape.role === 'accent'}
        {#if accent}
          {#if shape.tag === 'path'}
            <path {...shape.attrs} fill={accentFill} stroke="none" />
          {:else if shape.tag === 'circle'}
            <circle {...shape.attrs} fill={accentFill} stroke="none" />
          {/if}
        {/if}
      {:else if shape.role === 'danger'}
        {#if accent}
          {#if shape.tag === 'circle'}
            <circle {...shape.attrs} fill="var(--ink-danger)" stroke="none" />
          {:else if shape.tag === 'path'}
            <path {...shape.attrs} fill="var(--ink-danger)" stroke="none" />
          {/if}
        {/if}
      {:else if shape.role === 'dot'}
        <circle {...shape.attrs} fill="currentColor" stroke="none" />
      {:else if shape.tag === 'path'}
        <path {...shape.attrs} />
      {:else if shape.tag === 'rect'}
        <rect {...shape.attrs} />
      {:else if shape.tag === 'circle'}
        <circle {...shape.attrs} />
      {/if}
    {/each}
  </svg>
{/if}

<style>
  .ink-icon {
    flex: none;
    display: inline-block;
    vertical-align: middle;
  }
</style>
