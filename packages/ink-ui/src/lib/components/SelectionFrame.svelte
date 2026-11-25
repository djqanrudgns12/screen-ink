<script>
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
    label = '고른 범위',
    class: className = '',
  } = $props();

  let pad = 8;
  let svgW = $derived(width + pad * 2);
  let svgH = $derived(height + pad * 2);
  let arm = $derived(Math.min(14, Math.floor(Math.min(width, height) / 3)));
</script>

<div
  aria-label={label}
  class="ink-sel-frame {className}"
  data-state={frameState}
  data-scrim={scrim && frameState !== 'afterimage' ? 'true' : undefined}
  style:left={`${x}px`}
  style:top={`${y}px`}
  style:width={`${width}px`}
  style:height={`${height}px`}
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
      <path
        d={`M1 ${1 + arm}V1h${arm}M${svgW - 1 - arm} 1h${arm}v${arm}M${svgW - 1} ${svgH - 1 - arm}v${arm}h${-arm}M${1 + arm} ${svgH - 1}H1v${-arm}`}
        fill="none"
        stroke={frameState === 'recording' ? 'var(--ink-danger)' : 'var(--ink-accent)'}
        stroke-width="2.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  {/if}

  {#if frameState === 'confirmed'}
    <span class="ink-sel-handle ink-h-tl" aria-hidden="true"></span>
    <span class="ink-sel-handle ink-h-tr" aria-hidden="true"></span>
    <span class="ink-sel-handle ink-h-bl" aria-hidden="true"></span>
    <span class="ink-sel-handle ink-h-br" aria-hidden="true"></span>
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

  .ink-sel-frame[data-state='picking'],
  .ink-sel-frame[data-state='confirmed'] {
    outline: 1.5px solid var(--ink-accent);
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
</style>
