<script>
  import { t } from '../i18n/index.svelte.js';

  /**
   * 캡처·녹화 선택 틀(F09 · F10 · F12 · 10 §5).
   * 상태: picking(고르는 중) | confirmed(확정·손잡이) | recording(녹화·빨강) | afterimage(잔상)
   * @type {{
   *   x?: number,
   *   y?: number,
   *   width: number,
   *   height: number,
   *   state?: 'picking' | 'confirmed' | 'recording' | 'afterimage',
   *   scrim?: boolean,
   *   instant?: boolean,
   *   aligned?: boolean,
   *   label?: string,
   *   class?: string,
   * }}
   */
  let {
    x = 0,
    y = 0,
    width = 240,
    height = 100,
    state: frameState = 'picking',
    scrim = true,
    instant = false,
    aligned = false,
    label = undefined,
    class: className = '',
  } = $props();

  let resolvedLabel = $derived(label === undefined ? t('islandUi.selection.aria') : label);

  let pad = 8;
  let safeW = $derived(Math.max(0, width));
  let safeH = $derived(Math.max(0, height));
  let svgW = $derived(safeW + pad * 2);
  let svgH = $derived(safeH + pad * 2);
  let arm = $derived(Math.min(14, Math.floor(Math.min(safeW, safeH) / 3)));
  // 창 추적 녹화에서는 꺾쇠 선의 중심을 창 경계에 놓는다.
  let corner = $derived(aligned ? pad : 1);
  let borderInset = $derived(Math.min(0.75, safeW / 2, safeH / 2));
</script>

<div
  aria-label={resolvedLabel}
  class="ink-sel-frame {className}"
  data-state={frameState}
  data-scrim={scrim && frameState !== 'afterimage' ? 'true' : undefined}
  data-instant={instant ? 'true' : undefined}
  style:left={`${x}px`}
  style:top={`${y}px`}
  style:width={`${safeW}px`}
  style:height={`${safeH}px`}
>
  {#if frameState !== 'afterimage'}
    <svg
      aria-hidden="true"
      class="ink-sel-brackets"
      width={svgW}
      height={svgH}
      viewBox={`0 0 ${svgW} ${svgH}`}
      style:left={`${-pad}px`}
      style:top={`${-pad}px`}
    >
      {#if (frameState === 'picking' || frameState === 'confirmed') && safeW > 0 && safeH > 0}
        <rect
          x={pad + borderInset}
          y={pad + borderInset}
          width={Math.max(0, safeW - borderInset * 2)}
          height={Math.max(0, safeH - borderInset * 2)}
          fill="none"
          stroke="var(--ink-accent)"
          stroke-width="1.5"
        />
      {/if}
      {#if arm > 0}
        <path
          d={`M${corner} ${corner + arm}V${corner}h${arm}M${svgW - corner - arm} ${corner}h${arm}v${arm}M${svgW - corner} ${svgH - corner - arm}v${arm}h${-arm}M${corner + arm} ${svgH - corner}H${corner}v${-arm}`}
          fill="none"
          stroke={frameState === 'recording' ? 'var(--ink-danger)' : 'var(--ink-accent)'}
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      {/if}
    </svg>
  {/if}

  {#if frameState === 'confirmed'}
    <span class="ink-sel-handle ink-h-tl" aria-hidden="true"></span>
    <span class="ink-sel-handle ink-h-tr" aria-hidden="true"></span>
    <span class="ink-sel-handle ink-h-bl" aria-hidden="true"></span>
    <span class="ink-sel-handle ink-h-br" aria-hidden="true"></span>
    <!-- 변 가운데 손잡이: 변을 잡아 늘리기·줄이기(2026-11-28) — 범위가 작으면 귀 손잡이와 겹치지 않게 숨김 -->
    {#if width >= 48}
      <span class="ink-sel-handle ink-h-t" aria-hidden="true"></span>
      <span class="ink-sel-handle ink-h-b" aria-hidden="true"></span>
    {/if}
    {#if height >= 48}
      <span class="ink-sel-handle ink-h-l" aria-hidden="true"></span>
      <span class="ink-sel-handle ink-h-r" aria-hidden="true"></span>
    {/if}
  {/if}
</div>

<style>
  .ink-sel-frame {
    position: absolute;
    box-sizing: border-box;
    pointer-events: none;
    transition:
      left 70ms ease-out,
      top 70ms ease-out,
      width 70ms ease-out,
      height 70ms ease-out;
  }

  .ink-sel-frame[data-instant='true'] {
    transition: none;
  }

  .ink-sel-frame[data-state='picking'][data-scrim='true'],
  .ink-sel-frame[data-state='confirmed'][data-scrim='true'] {
    box-shadow: 0 0 0 9999px var(--ink-scrim);
  }

  .ink-sel-frame[data-state='recording'] {
    border-radius: 6px;
  }

  .ink-sel-frame[data-state='recording'][data-scrim='true'] {
    box-shadow: 0 0 0 9999px var(--ink-scrim-rec);
  }

  .ink-sel-frame[data-state='afterimage'] {
    outline: 2px solid var(--ink-afterimage);
    border-radius: 2px;
    animation: ink-afterimage 400ms linear forwards;
  }

  .ink-sel-brackets {
    position: absolute;
    pointer-events: none;
  }

  .ink-sel-handle {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--ink-surface);
    border: 1.5px solid var(--ink-accent);
    box-sizing: border-box;
  }

  .ink-h-tl {
    left: -4px;
    top: -4px;
  }
  .ink-h-tr {
    right: -4px;
    top: -4px;
  }
  .ink-h-bl {
    left: -4px;
    bottom: -4px;
  }
  .ink-h-br {
    right: -4px;
    bottom: -4px;
  }
  .ink-h-t {
    left: calc(50% - 4px);
    top: -4px;
  }
  .ink-h-b {
    left: calc(50% - 4px);
    bottom: -4px;
  }
  .ink-h-l {
    left: -4px;
    top: calc(50% - 4px);
  }
  .ink-h-r {
    right: -4px;
    top: calc(50% - 4px);
  }
</style>
