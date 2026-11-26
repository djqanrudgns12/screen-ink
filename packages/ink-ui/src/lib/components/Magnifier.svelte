<script>
  import { MAGNIFIER_LENS, formatColor, formatHex } from '../models/captureOverlay.js';
  import { t } from '../i18n/index.svelte.js';

  /**
   * 108×84 확대경 + 색·좌표 알약(F09 · 10 §5).
   * 화면 가장자리에 닿으면 `flipX`/`flipY`로 반대쪽으로 넘긴다.
   * @type {{
   *   rgb?: [number, number, number],
   *   coord?: { x: number, y: number } | null,
   *   format?: 'hex' | 'rgb',
   *   flipX?: boolean,
   *   flipY?: boolean,
   *   patch?: { data: Uint8ClampedArray, width: number, height: number } | null,
   *   class?: string,
   * }}
   */
  let {
    rgb = [255, 255, 255],
    coord = null,
    format = 'hex',
    flipX = false,
    flipY = false,
    patch = null,
    class: className = '',
  } = $props();

  let lensCanvas = $state(null);
  /** @type {HTMLCanvasElement | null} */
  let patchCanvas = null;
  /** @type {CanvasRenderingContext2D | null} */
  let patchCtx = null;

  let hexLabel = $derived(formatHex(rgb));
  let colorText = $derived(formatColor(rgb, format));

  $effect(() => {
    const canvas = lensCanvas;
    const currentPatch = patch;
    if (!canvas || !currentPatch || typeof document === 'undefined') return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    if (!patchCanvas || patchCanvas.width !== currentPatch.width || patchCanvas.height !== currentPatch.height) {
      patchCanvas = document.createElement('canvas');
      patchCanvas.width = currentPatch.width;
      patchCanvas.height = currentPatch.height;
      patchCtx = patchCanvas.getContext('2d');
    }
    if (!patchCtx) return;

    const imgData = patchCtx.createImageData(currentPatch.width, currentPatch.height);
    imgData.data.set(currentPatch.data);
    patchCtx.putImageData(imgData, 0, 0);

    ctx.clearRect(0, 0, MAGNIFIER_LENS.width, MAGNIFIER_LENS.height);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(
      patchCanvas,
      MAGNIFIER_LENS.drawOffsetX,
      MAGNIFIER_LENS.drawOffsetY,
      currentPatch.width * MAGNIFIER_LENS.zoom,
      currentPatch.height * MAGNIFIER_LENS.zoom,
    );
  });
</script>

<div
  aria-label={t('islandUi.magnifier.aria')}
  class="ink-mag {className}"
  data-flip-x={flipX ? 'true' : undefined}
  data-flip-y={flipY ? 'true' : undefined}
>
  <div class="ink-mag-lens">
    {#if patch}
      <canvas
        bind:this={lensCanvas}
        class="ink-mag-canvas"
        width={MAGNIFIER_LENS.width}
        height={MAGNIFIER_LENS.height}
        aria-hidden="true"
      ></canvas>
    {/if}
    <div class="ink-mag-grid" aria-hidden="true"></div>
    <div class="ink-mag-cross-v" aria-hidden="true"></div>
    <div class="ink-mag-cross-h" aria-hidden="true"></div>
  </div>
  <div class="ink-mag-pill">
    <span class="ink-mag-swatch" style:background-color={hexLabel} aria-hidden="true"></span>
    <span class="ink-mono">{colorText}</span>
    {#if coord}
      <span class="ink-mag-sub ink-mono">{coord.x},{coord.y}</span>
    {/if}
    <span class="ink-mag-sub">{t('islandUi.magnifier.copyHex')}</span>
  </div>
</div>

<style>
  .ink-mag {
    display: inline-flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 5px;
    font-family: var(--ink-font-sans);
  }

  .ink-mag[data-flip-x='true'] {
    align-items: flex-end;
  }

  .ink-mag[data-flip-y='true'] {
    flex-direction: column-reverse;
  }

  .ink-mag-lens {
    position: relative;
    width: 108px;
    height: 84px;
    border-radius: 10px;
    overflow: hidden;
    background: var(--ink-mag-bg);
    box-shadow: var(--ink-mag-shadow);
  }

  .ink-mag-canvas {
    position: absolute;
    inset: 0;
    width: 108px;
    height: 84px;
    display: block;
    image-rendering: pixelated;
  }

  .ink-mag-grid {
    position: absolute;
    inset: 0;
    background-image:
      linear-gradient(to right, var(--ink-mag-grid) 1px, transparent 1px),
      linear-gradient(to bottom, var(--ink-mag-grid) 1px, transparent 1px);
    background-size: 9px 9px;
    background-position: 50px 37px;
  }

  .ink-mag-cross-v {
    position: absolute;
    left: 50px;
    top: 0;
    bottom: 0;
    width: 9px;
    border-left: 1px solid var(--ink-accent);
    border-right: 1px solid var(--ink-accent);
    box-sizing: border-box;
  }

  .ink-mag-cross-h {
    position: absolute;
    top: 37px;
    left: 0;
    right: 0;
    height: 9px;
    border-top: 1px solid var(--ink-accent);
    border-bottom: 1px solid var(--ink-accent);
    box-sizing: border-box;
  }

  .ink-mag-pill {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    background: var(--ink-island);
    color: var(--ink-island-text);
    font-size: 11px;
    white-space: nowrap;
  }

  .ink-mag-swatch {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--ink-line);
    flex: none;
  }

  .ink-mag-sub {
    color: var(--ink-island-sub);
  }
</style>
