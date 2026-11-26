<script>
  import { ICONS } from './icons.js';

  /**
   * 슉쇽 맞춤 아이콘 렌더러(24×24 격자, 20px→1.60 / 16px→1.75 광학 획 보정).
   * @type {{
   *   name: string,
   *   size?: 16 | 18 | 20 | 24 | number,
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
  let strokeWidth = $derived(size <= 16 ? 1.75 : size < 20 ? 1.65 : 1.6);
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
          {:else if shape.tag === 'rect'}
            <rect {...shape.attrs} fill={accentFill} stroke="none" />
          {:else if shape.tag === 'ellipse'}
            <ellipse {...shape.attrs} fill={accentFill} stroke="none" />
          {/if}
        {/if}
      {:else if shape.role === 'danger'}
        {#if shape.tag === 'circle'}
          <circle {...shape.attrs} fill={accent ? 'var(--ink-danger)' : 'currentColor'} stroke="none" />
        {:else if shape.tag === 'path'}
          <path {...shape.attrs} fill={accent ? 'var(--ink-danger)' : 'currentColor'} stroke="none" />
        {/if}
      {:else if shape.role === 'dot'}
        <circle {...shape.attrs} fill="currentColor" stroke="none" />
      {:else if shape.role === 'fill'}
        {#if shape.tag === 'path'}
          <path {...shape.attrs} fill="currentColor" stroke="none" />
        {:else if shape.tag === 'rect'}
          <rect {...shape.attrs} fill="currentColor" stroke="none" />
        {:else if shape.tag === 'circle'}
          <circle {...shape.attrs} fill="currentColor" stroke="none" />
        {:else if shape.tag === 'ellipse'}
          <ellipse {...shape.attrs} fill="currentColor" stroke="none" />
        {/if}
      {:else if shape.tag === 'path'}
        <path {...shape.attrs} />
      {:else if shape.tag === 'rect'}
        <rect {...shape.attrs} />
      {:else if shape.tag === 'circle'}
        <circle {...shape.attrs} />
      {:else if shape.tag === 'ellipse'}
        <ellipse {...shape.attrs} />
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
