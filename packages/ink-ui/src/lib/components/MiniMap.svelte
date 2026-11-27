<script>
  import { normalizeMinimapViewport } from '../models/drawUi.js';
  import { t } from '../i18n/index.svelte.js';

  /**
   * 확대 중 위치 지도(96×60 또는 112×70, 주황 사각 = 보이는 영역, F13·F14 · 10 §5).
   * @type {{
   *   size?: 'sm' | 'md',
   *   viewport?: { x: number, y: number, w: number, h: number },
   *   label?: string,
   *   zoomText?: string,
   *   ariaLabel?: string,
   *   class?: string,
   * }}
   */
  let {
    size = 'md',
    viewport = { x: 0.3, y: 0.28, w: 0.45, h: 0.46 },
    label = undefined,
    zoomText = '',
    ariaLabel = undefined,
    class: className = '',
  } = $props();

  let resolvedLabel = $derived(label === undefined ? t('zoomUi.minimap.defaultLabel') : label);
  let resolvedAriaLabel = $derived(ariaLabel === undefined ? t('zoomUi.minimap.aria') : ariaLabel);
  let norm = $derived(normalizeMinimapViewport(viewport));
  let vx = $derived(norm.x * 100);
  let vy = $derived(norm.y * 100);
  let vw = $derived(norm.w * 100);
  let vh = $derived(norm.h * 100);
</script>

<div aria-label={resolvedAriaLabel} class="ink-minimap {className}" data-size={size}>
  <span
    class="ink-minimap-view"
    style:left={`${vx}%`}
    style:top={`${vy}%`}
    style:width={`${vw}%`}
    style:height={`${vh}%`}
  ></span>
  {#if resolvedLabel}
    <span class="ink-minimap-label ink-mono">{resolvedLabel}</span>
  {/if}
  {#if zoomText}
    <span class="ink-minimap-zoom ink-mono">{zoomText}</span>
  {/if}
</div>

<style>
  .ink-minimap {
    position: relative;
    width: 112px;
    height: 70px;
    border-radius: 9px;
    background: var(--ink-minimap-bg);
    box-shadow: var(--ink-minimap-ring);
    overflow: hidden;
  }

  .ink-minimap[data-size='sm'] {
    width: 96px;
    height: 60px;
  }

  .ink-minimap-view {
    position: absolute;
    border-radius: 3px;
    background: var(--ink-minimap-view-bg);
    box-shadow: 0 0 0 1.5px var(--ink-accent);
    box-sizing: border-box;
  }

  .ink-minimap-label {
    position: absolute;
    left: 8px;
    bottom: 5px;
    font-size: 9.5px;
    color: var(--ink-island-sub);
    white-space: nowrap;
  }

  .ink-minimap-zoom {
    position: absolute;
    right: 6px;
    bottom: 4px;
    font-size: 9.5px;
    color: var(--ink-island-accent-text);
    white-space: nowrap;
  }
</style>
