<script>
  import { Spring } from 'svelte/motion';
  import Island from '../components/Island.svelte';
  import {
    DRAW_IDLE_DIM_MS,
    DRAW_PALETTE_EXPAND_MS,
    ISLAND_AVOID_MARGIN_PX,
    isDrawIslandVariant,
    resolveDrawIdleDim,
    shouldAvoidIsland,
    shouldExpandDrawPalette,
  } from '../models/drawUi.js';
  import {
    ISLAND_SPRING,
    applyCaptureView,
    clearModeIsland,
    createIslandState,
    dismissIsland,
    getAutoDismissMs,
    pushIsland,
    resolveIslandChip,
    setHelpOpen,
    setIslandHover,
    shouldShowHintLine,
  } from '../models/islandQueue.js';

  /**
   * S17 섬 호스트 — 한 번에 하나, 1.2초 자동 닫힘(올리면 멈춤), 안내 줄은 처음 3번 또는 '?' 때만.
   * 한 줄 ↔ 두 줄 ↔ 결과 전환은 스프링(단단함 .22 · 감쇠 .78)으로 폭·높이를 잇는다(10 §6).
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   position?: 'fixed' | 'inline',
   *   class?: string,
   * }}
   */
  let { client, position = 'fixed', class: className = '' } = $props();

  let queue = $state(createIslandState());
  /** @type {number | null} */
  let captureSession = $state(null);
  let platform = $state('windows');
  /** @type {'R' | 'Y' | 'G' | 'O' | 'B' | 'L' | 'W' | 'K'} */
  let defaultPenColor = $state('R');
  let reducedMotion = $state(false);
  /** @type {HTMLDivElement | null} */
  let anchorEl = $state(null);
  /** @type {HTMLDivElement | null} */
  let measureEl = $state(null);
  let hasMeasured = $state(false);
  let islandHover = $state(false);
  let drawPaletteExpanded = $state(false);
  let autoIdleDim = $state(false);
  let localAvoid = $state(false);
  let drawActivityNonce = $state(0);

  /** @type {string | null} */
  let lastDrawColor = null;
  let lastDrawSig = '';
  /** @type {ReturnType<typeof setTimeout> | null} */
  let paletteTimer = null;

  function clearDrawTimers() {
    if (paletteTimer !== null) {
      clearTimeout(paletteTimer);
      paletteTimer = null;
    }
  }

  function resetDrawUi() {
    clearDrawTimers();
    lastDrawColor = null;
    lastDrawSig = '';
    drawPaletteExpanded = false;
    autoIdleDim = false;
    localAvoid = false;
  }

  function bumpDrawActivity() {
    autoIdleDim = false;
    drawActivityNonce += 1;
  }

  const springW = new Spring(0, {
    stiffness: ISLAND_SPRING.stiffness,
    damping: ISLAND_SPRING.damping,
  });
  const springH = new Spring(0, {
    stiffness: ISLAND_SPRING.stiffness,
    damping: ISLAND_SPRING.damping,
  });

  $effect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    reducedMotion = mq.matches;
    const onChange = (e) => {
      reducedMotion = e.matches;
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (!alive || !s) return;
      if (s.platform) platform = s.platform;
      if (s.settings?.['draw.color']) defaultPenColor = s.settings['draw.color'];
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        if (e.state?.platform) platform = e.state.platform;
        if (e.state?.settings?.['draw.color']) defaultPenColor = e.state.settings['draw.color'];
        if (e.state?.mode?.mode !== 'capture') captureSession = null;
      } else if (e.type === 'island') {
        if (e.island?.kind === 'end') {
          captureSession = null;
          resetDrawUi();
        } else if (e.island?.kind === 'mode') {
          const nextVariant = e.island.variant || e.island.mode || 'region';
          if (isDrawIslandVariant(nextVariant)) {
            const nextColor = e.island.penColor || defaultPenColor;
            if (shouldExpandDrawPalette(lastDrawColor, nextColor, nextVariant)) {
              drawPaletteExpanded = true;
              clearDrawTimers();
              paletteTimer = setTimeout(() => {
                paletteTimer = null;
                drawPaletteExpanded = false;
              }, DRAW_PALETTE_EXPAND_MS);
            }
            lastDrawColor = nextColor;
            const sig = `${nextVariant}|${e.island.text}|${e.island.detail ?? ''}|${e.island.value ?? ''}|${nextColor}|${e.island.penWidth ?? ''}|${e.island.button?.action ?? ''}`;
            if (sig !== lastDrawSig || e.island.idleDim === false) {
              lastDrawSig = sig;
              bumpDrawActivity();
            }
          } else {
            resetDrawUi();
          }
        }
        queue =
          e.island?.kind === 'end'
            ? clearModeIsland(queue)
            : pushIsland(queue, e.island, { now: Date.now(), reducedMotion });
      } else if (e.type === 'capture') {
        captureSession = e.view?.session ?? null;
        queue = applyCaptureView(queue, e.view);
      }
    });
    return () => {
      alive = false;
      clearDrawTimers();
      off();
    };
  });

  $effect(() => {
    const exp = queue.expiresAt;
    if (exp === null || queue.hovering) return;
    const delay = Math.max(0, exp - Date.now());
    const timer = setTimeout(() => {
      queue = dismissIsland(queue);
    }, delay);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const el = measureEl;
    if (!el) {
      hasMeasured = false;
      return;
    }

    const syncSize = () => {
      const rect = el.getBoundingClientRect();
      const w = Math.ceil(rect.width);
      const h = Math.ceil(rect.height);
      if (w <= 0 || h <= 0) return;

      if (!hasMeasured || reducedMotion) {
        springW.set(w, { instant: true });
        springH.set(h, { instant: true });
        hasMeasured = true;
      } else {
        springW.target = w;
        springH.target = h;
      }
    };

    syncSize();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(syncSize);
    ro.observe(el);
    return () => ro.disconnect();
  });

  function handleKeydown(e) {
    if (isDrawMode && current?.idleDim === undefined) {
      bumpDrawActivity();
    }
    if (e.defaultPrevented || captureSession !== null) return;
    const tag = e.target?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || e.target?.isContentEditable) return;
    if (e.key === '?' && queue.current?.hints?.length) {
      queue = setHelpOpen(queue, !queue.helpOpen);
    }
  }

  /** @param {PointerEvent} e */
  function handleWindowPointerMove(e) {
    if (!isDrawMode) {
      if (localAvoid) localAvoid = false;
      return;
    }
    if (current?.idleDim === undefined) {
      bumpDrawActivity();
    }
    if (hasAction || drawPaletteExpanded || !anchorEl) {
      if (localAvoid) localAvoid = false;
      return;
    }
    localAvoid = shouldAvoidIsland(
      { x: e.clientX, y: e.clientY },
      anchorEl.getBoundingClientRect(),
      ISLAND_AVOID_MARGIN_PX,
    );
  }

  function handleWindowPointerLeave() {
    if (localAvoid) localAvoid = false;
  }

  let current = $derived(queue.current);
  let variant = $derived.by(() => {
    if (!current) return 'info';
    if (current.variant) return current.variant;
    if (current.kind === 'result') return 'result';
    if (current.kind === 'problem') return 'problem';
    if (current.kind === 'off') return 'off';
    if (current.kind === 'mode') return current.mode || 'region';
    return 'info';
  });

  let mutedSurface = $derived(variant === 'off' || variant === 'security');
  let isDrawMode = $derived(Boolean(current && current.kind === 'mode' && isDrawIslandVariant(variant)));
  let resolvedPenColor = $derived(
    current?.penColor ?? (isDrawIslandVariant(variant) ? defaultPenColor : undefined),
  );
  let showHints = $derived(shouldShowHintLine(queue));
  let morphKey = $derived(
    current
      ? `${current.kind}:${variant}:${current.text}:${current.chip?.label ?? ''}:${showHints ? 1 : 0}`
      : '',
  );
  let contentResetKey = $derived(current?.kind === 'mode' ? 'mode' : queue.seq);
  let durationMs = $derived(
    current && current.kind !== 'mode'
      ? getAutoDismissMs(current, { hasActiveMode: Boolean(queue.activeMode), reducedMotion })
      : null,
  );

  let actionButton = $derived(
    current?.button
      ? {
          label: current.button.label,
          key: current.buttonKey || current.button.key,
          stop: current.button.stop,
          onClick: async () => {
            const seq = queue.seq;
            const act = current.button.action;
            if (act === 'undo') {
              await client.undo();
            } else if (act) {
              await client.run(act);
            }
            if (queue.seq === seq && current.kind !== 'mode') {
              queue = dismissIsland(queue);
            }
          },
        }
      : null,
  );

  let chipButton = $derived.by(() => {
    const parsed = resolveIslandChip(current?.chip);
    if (!parsed) return null;
    return {
      label: parsed.label,
      hex: parsed.hex ?? undefined,
      onClick: async () => {
        const seq = queue.seq;
        if (parsed.action) {
          await client.run(/** @type {import('@screen-ink/client').InkAction} */ (parsed.action));
        }
        if (queue.seq === seq && current?.kind !== 'mode') {
          queue = dismissIsland(queue);
        }
      },
    };
  });
  let hasAction = $derived(Boolean(actionButton || chipButton));
  let isIdleDim = $derived(
    isDrawMode &&
      resolveDrawIdleDim({
        explicitIdleDim: current?.idleDim,
        autoIdleDim,
        hovering: islandHover,
        hasAction,
      }),
  );
  let isAvoid = $derived(
    isDrawMode && !hasAction && (Boolean(current?.avoid) || (!drawPaletteExpanded && localAvoid)),
  );

  // 4초 무동작 40% 물러남(Rust가 idleDim을 명시하지 않을 때 섬 호스트가 자체 계시)
  $effect(() => {
    void drawActivityNonce;
    if (!isDrawMode || typeof current?.idleDim === 'boolean' || islandHover || hasAction) {
      autoIdleDim = false;
      return;
    }
    const t = setTimeout(() => {
      autoIdleDim = true;
    }, DRAW_IDLE_DIM_MS);
    return () => clearTimeout(t);
  });
</script>

<svelte:window
  onkeydown={handleKeydown}
  onpointermove={handleWindowPointerMove}
  onpointerleave={handleWindowPointerLeave}
/>

{#if current}
  <div class="ink-island-host {className}" data-position={position}>
    <div class="ink-island-anchor" bind:this={anchorEl}>
      <div
        class="ink-island-morph"
        data-muted={mutedSurface ? 'true' : undefined}
        data-idle-dim={isIdleDim ? 'true' : undefined}
        data-avoid={isAvoid ? 'true' : undefined}
        style:width={hasMeasured ? `${springW.current}px` : undefined}
        style:height={hasMeasured ? `${springH.current}px` : undefined}
      >
        <div class="ink-island-measure" bind:this={measureEl}>
          {#key morphKey}
            <div
              class="ink-island-content"
              data-reveal={hasMeasured && !reducedMotion ? 'true' : undefined}
            >
              {#key contentResetKey}
                <Island
                  {variant}
                  text={current.text}
                  lead={current.lead || ''}
                  detail={current.detail || ''}
                  value={current.value || ''}
                  valueMuted={Boolean(current.valueMuted ?? current.kind === 'result')}
                  path={current.path}
                  level={current.level ?? -1}
                  tags={current.tags}
                  thumbUrl={current.thumbUrl || ''}
                  combo={current.combo || ''}
                  keys={current.keys}
                  {platform}
                  keyHint={current.keyHint || ''}
                  countdown={current.countdown ?? null}
                  penColor={resolvedPenColor}
                  penWidth={current.penWidth || ''}
                  paletteExpanded={isDrawMode && drawPaletteExpanded}
                  idleDim={isIdleDim}
                  avoid={isAvoid}
                  micOff={Boolean(current.micOff)}
                  button={actionButton}
                  chip={chipButton}
                  hints={current.hints || []}
                  {showHints}
                  progressDurationMs={current.kind === 'result' ? durationMs : null}
                  paused={queue.hovering}
                  onmouseenter={() => {
                    islandHover = true;
                    queue = setIslandHover(queue, true, { now: Date.now(), reducedMotion });
                  }}
                  onmouseleave={() => {
                    islandHover = false;
                    queue = setIslandHover(queue, false, { now: Date.now(), reducedMotion });
                  }}
                />
              {/key}
            </div>
          {/key}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .ink-island-host {
    display: flex;
    justify-content: center;
    pointer-events: none;
    z-index: 50;
  }

  .ink-island-host[data-position='fixed'] {
    position: fixed;
    left: 0;
    right: 0;
    top: 14px;
  }

  .ink-island-anchor {
    display: inline-flex;
    justify-content: center;
    pointer-events: none;
  }

  .ink-island-morph {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    max-width: 520px;
    border-radius: 22px;
    background: var(--ink-island);
    box-shadow: var(--ink-island-shadow);
    overflow: hidden;
    pointer-events: auto;
    transition:
      opacity 300ms ease,
      transform 180ms cubic-bezier(0.2, 0, 0, 1);
  }

  .ink-island-morph[data-muted='true'] {
    background: var(--ink-island-muted);
  }

  .ink-island-morph[data-idle-dim='true'] {
    opacity: 0.4;
  }

  .ink-island-morph[data-avoid='true'] {
    transform: translateY(-120%);
    opacity: 0.15;
    pointer-events: none;
  }

  .ink-island-measure {
    flex: none;
    display: inline-flex;
  }

  .ink-island-content {
    display: inline-flex;
  }

  .ink-island-content[data-reveal='true'] {
    animation: ink-island-content-in 120ms ease-out 80ms both;
  }

  .ink-island-morph :global(.ink-island) {
    background: transparent;
    box-shadow: none;
    opacity: 1;
    transform: none;
  }

  @keyframes ink-island-content-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
