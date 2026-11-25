<script>
  /**
   * 확대 중 위치 지도(96×60 또는 112×70, 주황 사각 = 보이는 영역, F13·F14 · 10 §5).
   * @type {{
   *   size?: 'sm' | 'md',
   *   viewport?: { x: number, y: number, w: number, h: number },
   *   label?: string,
   *   class?: string,
   * }}
   */
  let {
    size = 'md',
    viewport = { x: 0.3, y: 0.28, w: 0.45, h: 0.46 },
    label = '모니터 1',
    class: className = '',
  } = $props();

  let clamp01 = (v) => Math.max(0, Math.min(1, v));
  let vx = $derived(clamp01(viewport.x) * 100);
  let vy = $derived(clamp01(viewport.y) * 100);
  let vw = $derived(clamp01(viewport.w) * 100);
  let vh = $derived(clamp01(viewport.h) * 100);
</script>

<div aria-label="위치 지도" class="ink-minimap {className}" data-size={size}>
  <span
    class="ink-minimap-view"
    style:left={`${vx}%`}
    style:top={`${vy}%`}
    style:width={`${vw}%`}
    style:height={`${vh}%`}
  ></span>
  {#if label}
    <span class="ink-minimap-label ink-mono">{label}</span>
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
</style>
