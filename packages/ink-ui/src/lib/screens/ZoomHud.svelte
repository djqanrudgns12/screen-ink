<script>
  /**
   * 실시간 확대 HUD(04 S13 · F14 · R-ZOOM-6): 클릭이 통과하는 투명 창 `ink-zoom-hud` 하나에
   * 커서 주황 후광(44px)과 오른쪽 아래 위치 지도(MiniMap 112×70 + 모니터 이름)만 그린다.
   * 이 창은 돋보기 원본·캡처에서 빠진다(Rust가 필터 목록·WDA_EXCLUDEFROMCAPTURE로). 섬은 섬 창이 그린다.
   * 좌표는 Rust가 정한다(`screen-ink://zoom` → 'zoom' 이벤트, 끝 = null).
   */
  import MiniMap from '../components/MiniMap.svelte';
  import { MINIMAP_MARGIN, applyZoomEvent, haloBox, showMinimap } from '../models/zoomHud.js';

  /** @type {{ client: import('@screen-ink/client').InkClient, class?: string }} */
  let { client, class: className = '' } = $props();

  /** @type {import('@screen-ink/client').LiveZoomState | null} */
  let zoom = $state(null);

  $effect(() => {
    const off = client.subscribe((e) => {
      zoom = applyZoomEvent(zoom, e);
    });
    return off;
  });

  let halo = $derived(zoom ? haloBox(zoom.cursorCss) : null);
  let mapOn = $derived(showMinimap(zoom));
</script>

{#if zoom}
  <div class="ink-zoom-hud {className}" aria-hidden="true" data-session={zoom.session}>
    {#if halo}
      <span
        class="ink-zoom-halo"
        style:width={`${halo.size}px`}
        style:height={`${halo.size}px`}
        style:transform={`translate3d(${halo.x}px, ${halo.y}px, 0)`}
      ></span>
    {/if}
    {#if mapOn}
      <div class="ink-zoom-map" style:right={`${MINIMAP_MARGIN}px`} style:bottom={`${MINIMAP_MARGIN}px`}>
        <MiniMap size="md" viewport={zoom.viewport} label={zoom.monitorLabel} />
      </div>
    {/if}
  </div>
{/if}

<style>
  /* HUD 창은 바탕이 투명해야 한다(tokens.css가 html·body에 바탕색을 깐다) */
  :global(html[data-view='zoom-hud']),
  :global(html[data-view='zoom-hud'] body) {
    background: transparent;
    overflow: hidden;
  }

  .ink-zoom-hud {
    position: fixed;
    inset: 0;
    pointer-events: none;
    user-select: none;
  }

  /* 커서를 잃지 않게: 얇은 주황 테두리 + 옅은 어둠(F14). 커서를 바로 따라가므로 전환 효과 없음 */
  .ink-zoom-halo {
    position: absolute;
    left: 0;
    top: 0;
    border-radius: 50%;
    box-sizing: border-box;
    background: rgba(0, 0, 0, 0.28);
    box-shadow:
      inset 0 0 0 1.5px var(--ink-accent),
      0 0 10px rgba(255, 159, 10, 0.35);
    will-change: transform;
  }

  .ink-zoom-map {
    position: absolute;
  }
</style>
