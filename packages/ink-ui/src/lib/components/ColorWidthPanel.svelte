<script>
  import Icon from '../icons/Icon.svelte';
  import FontPicker from './FontPicker.svelte';
  import { t } from '../i18n/index.svelte.js';
  import {
    DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_WEIGHT,
    buildFontFamilyStack,
    compactFontLabel,
    normalizeFontWeight,
    parseFontSetting,
    pushRecentFont,
    serializeFontSetting,
  } from '../models/fontPicker.js';
  import {
    BASIC_COLORS,
    EXTENDED_COLORS,
    isLightColor,
    markerPreviewColor,
    normalizeHex,
    resolvePaletteHex,
  } from '../models/palette.js';
  import {
    SIZE_PRESET_KEYS,
    computePreviewMetrics,
    formatSizeValue,
    getToolSizeSpec,
    sliderToValue,
    stepSize,
    valueToSlider,
  } from '../models/inkSize.js';

  /**
   * 색·굵기 통합 패널(R2-ANN-1~6 · R2-FIX-11):
   * - 기본 8색(R·Y·G·O·B·L·K·W) + 넓힌 8색 + 내 색(색상 고르기 + 최근 5) + 스포이트
   * - 도구별 범위 로그 슬라이더 + 가늘게·보통·굵게 자석(±8%) + 손잡이 숫자 말풍선 + 상시 숫자
   * - 실제 배율·확대 반영 미리보기(칸을 넘으면 줄이고 "실제의 n%")
   * - 글자 도구에는 Tidy식 글꼴 단추 + 바로 위에 붙는 팝오버(FontPicker)를 띄운다(R2-TXT-1·2·6)
   * @type {{
   *   toolId: string | null,
   *   color?: string,
   *   size?: number,
   *   recentColors?: ReadonlyArray<string>,
   *   font?: string,
   *   fontWeight?: number,
   *   recentFonts?: ReadonlyArray<string>,
   *   systemFonts?: ReadonlyArray<string>,
   *   client?: import('@screen-ink/client').InkClient | null,
   *   eyedropperActive?: boolean,
   *   showEyedropper?: boolean,
   *   displayScale?: number,
   *   zoomScale?: number,
   *   x?: number,
   *   y?: number,
   *   side?: 'above' | 'below',
   *   arrowX?: number,
   *   avoidRects?: ReadonlyArray<import('../models/popoverPlacement.js').RectLike | null | undefined> | null,
   *   onColor?: (codeOrHex: string, opts?: { recordRecent?: boolean }) => void,
   *   onSize?: (val: number) => void,
   *   onFont?: (family: string, weight: number) => void,
   *   onFontPreview?: (preview: { family: string, weight: number } | null) => void,
   *   onFontOpenChange?: (open: boolean) => void,
   *   onEyedropper?: () => void,
   *   onHover?: (over: boolean) => void,
   *   boxWidth?: number,
   *   boxHeight?: number,
   * }}
   */
  let {
    toolId,
    color = 'R',
    size = 4,
    recentColors = [],
    font = undefined,
    fontWeight = undefined,
    recentFonts = undefined,
    systemFonts = undefined,
    client = null,
    eyedropperActive = false,
    showEyedropper = true,
    displayScale = 1,
    zoomScale = 1,
    x = 0,
    y = 0,
    side = 'above',
    arrowX = 140,
    avoidRects = undefined,
    onColor = () => {},
    onSize = () => {},
    onFont = undefined,
    onFontPreview = undefined,
    onFontOpenChange = undefined,
    onEyedropper = () => {},
    onHover = () => {},
    boxWidth = $bindable(0),
    boxHeight = $bindable(0),
  } = $props();

  /** @type {HTMLDivElement | null} */
  let trackEl = $state(null);
  /** @type {HTMLInputElement | null} */
  let colorInputEl = $state(null);
  /** @type {HTMLButtonElement | null} */
  let fontBtnEl = $state(null);
  let dragging = $state(false);
  let trackHover = $state(false);
  let trackFocus = $state(false);
  let fontOpen = $state(false);
  let localFont = $state(DEFAULT_FONT_FAMILY);
  let localWeight = $state(DEFAULT_FONT_WEIGHT);
  /** @type {string[]} */
  let localRecentFonts = $state([]);
  /** @type {string[]} */
  let fetchedSystemFonts = $state([]);
  let systemFontsFetched = false;
  /** @type {{ family: string, weight: number } | null} */
  let fontPreview = $state(null);
  let wheelAcc = 0;

  let spec = $derived(getToolSizeSpec(toolId));
  let showColor = $derived(toolId !== 'mosaic' && toolId !== 'eraser');
  let isMarker = $derived(toolId === 'marker');
  let isText = $derived(toolId === 'text');
  let activeFont = $derived(font || localFont);
  let activeFontWeight = $derived(normalizeFontWeight(fontWeight ?? localWeight));
  let activeRecentFonts = $derived(recentFonts ?? localRecentFonts);
  let activeSystemFonts = $derived(systemFonts ?? fetchedSystemFonts);
  let previewFontFamily = $derived(fontPreview?.family ?? activeFont);
  let previewFontWeight = $derived(fontPreview?.weight ?? activeFontWeight);
  let fontLabelText = $derived(compactFontLabel(activeFont, activeFontWeight));
  let sliderPos = $derived(valueToSlider(size, spec));
  let sliderPct = $derived(Math.round(sliderPos * 1000) / 10);
  let activeHex = $derived(resolvePaletteHex(color));
  let previewColor = $derived(isMarker ? markerPreviewColor(color) : activeHex);
  let preview = $derived(computePreviewMetrics({ value: size, displayScale, zoomScale, maxPx: 40 }));
  let showThumbBubble = $derived(dragging || trackHover || trackFocus);
  let sizeLabelText = $derived(formatSizeValue(size, spec));

  $effect(() => {
    if (!isText && fontOpen) {
      fontOpen = false;
      fontPreview = null;
      onFontOpenChange?.(false);
    }
  });

  // 글자 도구일 때 client가 있으면 설정 annotate.font + 이 컴퓨터 글꼴(systemFonts) 1회 로드(이름은 로그에 남기지 않음)
  $effect(() => {
    if (!isText || !client) return;
    let alive = true;
    if (font === undefined && typeof client.getState === 'function') {
      Promise.resolve(client.getState())
        .then((s) => {
          if (!alive) return;
          const parsed = parseFontSetting(s?.settings?.['annotate.font']);
          localFont = parsed.family;
          localWeight = parsed.weight;
          localRecentFonts = parsed.recent;
        })
        .catch(() => {});
    }
    if (systemFonts === undefined && !systemFontsFetched && typeof client.systemFonts === 'function') {
      systemFontsFetched = true;
      Promise.resolve(client.systemFonts())
        .then((list) => {
          if (alive && Array.isArray(list)) fetchedSystemFonts = list;
        })
        .catch(() => {});
    }
    return () => {
      alive = false;
    };
  });

  /** @param {boolean} next */
  function setFontOpen(next) {
    if (fontOpen === next) return;
    fontOpen = next;
    if (!next) {
      fontPreview = null;
      onFontPreview?.(null);
    }
    onFontOpenChange?.(next);
  }

  /** @param {string} family @param {number} weight */
  function handleFontSelect(family, weight) {
    const normW = normalizeFontWeight(weight);
    localFont = family;
    localWeight = normW;
    localRecentFonts = pushRecentFont(activeRecentFonts, family, normW);
    if (onFont) {
      onFont(family, normW);
    } else if (typeof client?.updateSetting === 'function') {
      const serialized = serializeFontSetting({ family, weight: normW, recent: localRecentFonts });
      Promise.resolve(client.updateSetting('annotate.font', serialized)).catch(() => {});
    }
    setFontOpen(false);
  }

  /** @param {{ family: string, weight: number } | null} pv */
  function handleFontPreview(pv) {
    fontPreview = pv;
    onFontPreview?.(pv);
  }

  /** @param {Event} e */
  function stop(e) {
    e.stopPropagation();
  }

  /** @param {string} val */
  function isSelectedColor(val) {
    const a = normalizeHex(color);
    const b = normalizeHex(val);
    if (a && b) return a === b;
    return color === val;
  }

  /** @param {number} clientX */
  function updateFromClientX(clientX) {
    if (!trackEl) return;
    const rect = trackEl.getBoundingClientRect();
    if (rect.width <= 0) return;
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    const res = sliderToValue(ratio, spec, { snap: true });
    onSize(res.value);
  }

  /** @param {PointerEvent} e */
  function handleTrackDown(e) {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    dragging = true;
    try {
      trackEl?.setPointerCapture(e.pointerId);
    } catch {
      // 이미 풀림
    }
    trackEl?.focus();
    updateFromClientX(e.clientX);
  }

  /** @param {PointerEvent} e */
  function handleTrackMove(e) {
    if (!dragging) return;
    e.stopPropagation();
    updateFromClientX(e.clientX);
  }

  /** @param {PointerEvent} e */
  function handleTrackUp(e) {
    if (!dragging) return;
    e.stopPropagation();
    dragging = false;
    try {
      trackEl?.releasePointerCapture(e.pointerId);
    } catch {
      // 이미 풀림
    }
  }

  /** @param {KeyboardEvent} e */
  function handleTrackKey(e) {
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
      e.preventDefault();
      e.stopPropagation();
      onSize(stepSize(size, 1, spec, { large: e.shiftKey }));
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      onSize(stepSize(size, -1, spec, { large: e.shiftKey }));
    }
  }

  /** @param {WheelEvent} e */
  function handlePanelWheel(e) {
    e.preventDefault();
    e.stopPropagation();
    if (wheelAcc !== 0 && Math.sign(wheelAcc) !== Math.sign(e.deltaY)) wheelAcc = 0;
    wheelAcc += e.deltaY;
    if (Math.abs(wheelAcc) < 30) return;
    const dir = wheelAcc < 0 ? 1 : -1;
    wheelAcc = 0;
    onSize(stepSize(size, dir, spec, { large: e.shiftKey }));
  }

  /** @param {Event} e */
  function handleCustomInput(e) {
    const input = /** @type {HTMLInputElement} */ (e.currentTarget);
    const hex = normalizeHex(input.value);
    if (hex) onColor(hex, { recordRecent: true });
  }
</script>

<div
  class="ink-cw-panel"
  data-side={side}
  role="dialog"
  tabindex="-1"
  aria-label={t('capture.panel.label')}
  style:transform={`translate3d(${x}px, ${y}px, 0)`}
  bind:offsetWidth={boxWidth}
  bind:offsetHeight={boxHeight}
  onpointerenter={() => onHover(true)}
  onpointerleave={() => onHover(false)}
  onpointerdown={stop}
  onpointermove={stop}
  onpointerup={stop}
  ondblclick={stop}
  onwheel={handlePanelWheel}
  oncontextmenu={(e) => {
    e.preventDefault();
    e.stopPropagation();
  }}
>
  <span class="ink-cw-arrow" data-side={side} style:left={`${arrowX}px`} aria-hidden="true"></span>

  {#if showColor}
    <section class="ink-cw-section" aria-label={t('capture.panel.colorSection')}>
      <div class="ink-cw-head">
        <span class="ink-cw-label">{t('capture.panel.colorSection')}</span>
        {#if isMarker}
          <span class="ink-cw-badge">{t('capture.preview.markerNote')}</span>
        {/if}
      </div>

      <div class="ink-cw-grid" role="group" aria-label={t('capture.panel.colorSection')}>
        {#each [...BASIC_COLORS, ...EXTENDED_COLORS] as sw (sw.code)}
          {@const swLabel = sw.key ? `${t(sw.labelKey)} · ${sw.key}` : t(sw.labelKey)}
          <button
            type="button"
            class="ink-cw-swatch"
            class:active={isSelectedColor(sw.code)}
            class:light={sw.needsBorder}
            data-color={sw.code}
            aria-pressed={isSelectedColor(sw.code)}
            aria-label={swLabel}
            title={swLabel}
            style:background={sw.hex}
            onclick={(e) => {
              e.stopPropagation();
              onColor(sw.code);
            }}
          ></button>
        {/each}
      </div>

      <div class="ink-cw-custom-row">
        <span class="ink-cw-sublabel">{t('capture.panel.myColor')}</span>
        <div class="ink-cw-custom-controls">
          <button
            type="button"
            class="ink-cw-picker-btn"
            aria-label={t('capture.color.custom')}
            title={t('capture.color.custom')}
            onclick={(e) => {
              e.stopPropagation();
              colorInputEl?.click();
            }}
          >
            <span class="ink-cw-rainbow" aria-hidden="true"></span>
            <input
              bind:this={colorInputEl}
              type="color"
              class="ink-cw-color-native"
              value={activeHex}
              tabindex="-1"
              aria-hidden="true"
              oninput={handleCustomInput}
              onchange={handleCustomInput}
            />
          </button>

          {#if showEyedropper}
            <button
              type="button"
              class="ink-cw-eye-btn"
              class:active={eyedropperActive}
              data-action="eyedropper"
              aria-pressed={eyedropperActive}
              aria-label={t('capture.color.eyedropper')}
              title={t('capture.color.eyedropper')}
              onclick={(e) => {
                e.stopPropagation();
                onEyedropper();
              }}
            >
              <Icon name="eyedropper" size={16} accent={false} />
            </button>
          {/if}

          <div class="ink-cw-recent" role="group" aria-label={t('capture.color.recent')}>
            {#each recentColors as hex (hex)}
              <button
                type="button"
                class="ink-cw-recent-dot"
                class:active={isSelectedColor(hex)}
                class:light={isLightColor(hex)}
                data-color={hex}
                aria-pressed={isSelectedColor(hex)}
                aria-label={hex}
                title={hex}
                style:background={hex}
                onclick={(e) => {
                  e.stopPropagation();
                  onColor(hex, { recordRecent: true });
                }}
              ></button>
            {/each}
          </div>
        </div>
      </div>
    </section>
  {/if}

  {#if isText}
    <div class="ink-cw-font-row">
      <button
        bind:this={fontBtnEl}
        type="button"
        class="ink-cw-font-btn"
        class:open={fontOpen}
        data-action="font-picker"
        role="combobox"
        aria-haspopup="listbox"
        aria-controls="ink-font-picker-listbox"
        aria-expanded={fontOpen}
        aria-label={t('font.trigger.label')}
        title={fontLabelText}
        onmousedown={(e) => e.preventDefault()}
        onclick={(e) => {
          e.stopPropagation();
          setFontOpen(!fontOpen);
        }}
      >
        <Icon name="font" size={15} accent={fontOpen} />
        <span
          class="ink-cw-font-name"
          style:font-family={buildFontFamilyStack(activeFont)}
          style:font-weight={activeFontWeight}
        >
          {fontLabelText}
        </span>
        <svg
          class="ink-cw-font-caret"
          class:open={fontOpen}
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          aria-hidden="true"
        >
          <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>

      <FontPicker
        open={fontOpen}
        value={activeFont}
        weight={activeFontWeight}
        recentFonts={activeRecentFonts}
        systemFonts={activeSystemFonts}
        anchorEl={fontBtnEl}
        preferredSide={side}
        {avoidRects}
        layoutKey={`${x}:${y}:${side}:${boxHeight}`}
        onSelect={handleFontSelect}
        onPreview={handleFontPreview}
        onClose={() => setFontOpen(false)}
      />
    </div>
  {/if}

  <section class="ink-cw-section" aria-label={t('capture.panel.sizeSection')}>
    <div class="ink-cw-head">
      <span class="ink-cw-label">{t('capture.panel.sizeSection')}</span>
      <span class="ink-cw-size-value ink-mono" data-testid="size-value">{sizeLabelText}</span>
    </div>

    <div class="ink-cw-size-body">
      <div class="ink-cw-slider-col">
        <div
          bind:this={trackEl}
          class="ink-cw-track"
          role="slider"
          tabindex="0"
          aria-label={t('capture.size.slider')}
          aria-valuemin={spec.min}
          aria-valuemax={spec.max}
          aria-valuenow={size}
          aria-valuetext={sizeLabelText}
          onpointerdown={handleTrackDown}
          onpointermove={handleTrackMove}
          onpointerup={handleTrackUp}
          onpointercancel={handleTrackUp}
          onpointerenter={() => (trackHover = true)}
          onpointerleave={() => (trackHover = false)}
          onfocus={() => (trackFocus = true)}
          onblur={() => (trackFocus = false)}
          onkeydown={handleTrackKey}
        >
          <span class="ink-cw-rail" aria-hidden="true"></span>
          <span class="ink-cw-fill" style:width={`${sliderPct}%`} aria-hidden="true"></span>

          {#each SIZE_PRESET_KEYS as key (key)}
            {@const pos = Math.round(valueToSlider(spec.presets[key], spec) * 1000) / 10}
            <span
              class="ink-cw-snap-dot"
              class:snapped={size === spec.presets[key]}
              style:left={`${pos}%`}
              aria-hidden="true"
            ></span>
          {/each}

          <span class="ink-cw-thumb" style:left={`${sliderPct}%`} aria-hidden="true">
            {#if showThumbBubble}
              <span class="ink-cw-thumb-tip ink-mono">{sizeLabelText}</span>
            {/if}
          </span>
        </div>

        <div class="ink-cw-ticks">
          {#each SIZE_PRESET_KEYS as key (key)}
            {@const pos = Math.round(valueToSlider(spec.presets[key], spec) * 1000) / 10}
            <button
              type="button"
              class="ink-cw-tick-btn"
              class:active={size === spec.presets[key]}
              style:left={`${pos}%`}
              onclick={(e) => {
                e.stopPropagation();
                onSize(spec.presets[key]);
              }}
            >
              {t(`capture.size.${key}`)}
            </button>
          {/each}
        </div>
      </div>

      <div class="ink-cw-preview-wrap" aria-label={t('capture.preview.label')}>
        <div class="ink-cw-preview-box">
          {#if toolId === 'text'}
            <span
              class="ink-cw-preview-text"
              style:font-size={`${Math.max(10, preview.drawPx)}px`}
              style:font-family={buildFontFamilyStack(previewFontFamily)}
              style:font-weight={previewFontWeight}
              style:color={previewColor}
            >
              {t('capture.preview.sample')}
            </span>
          {:else if toolId === 'marker'}
            <span
              class="ink-cw-preview-marker"
              style:height={`${Math.max(4, preview.drawPx)}px`}
              style:background={previewColor}
            ></span>
          {:else if toolId === 'mosaic'}
            {@const cell = Math.max(4, Math.min(18, Math.round(preview.drawPx * 0.45)))}
            <span
              class="ink-cw-preview-mosaic"
              style:grid-template-columns={`repeat(3, ${cell}px)`}
              style:grid-template-rows={`repeat(3, ${cell}px)`}
              aria-hidden="true"
            >
              {#each [0.9, 0.45, 0.75, 0.35, 0.85, 0.5, 0.7, 0.4, 0.9] as op, idx (idx)}
                <span style:opacity={op}></span>
              {/each}
            </span>
          {:else if toolId === 'eraser'}
            <span
              class="ink-cw-preview-eraser"
              style:width={`${Math.max(8, preview.drawPx)}px`}
              style:height={`${Math.max(8, preview.drawPx)}px`}
            ></span>
          {:else}
            <span
              class="ink-cw-preview-dot"
              class:light={isLightColor(color)}
              style:width={`${Math.max(2, preview.drawPx)}px`}
              style:height={`${Math.max(2, preview.drawPx)}px`}
              style:background={previewColor}
            ></span>
          {/if}
        </div>
        {#if preview.scaled}
          <span class="ink-cw-preview-scale ink-mono">
            {t('capture.preview.scale', { pct: preview.scalePercent })}
          </span>
        {/if}
      </div>
    </div>
  </section>
</div>

<style>
  .ink-cw-panel {
    --cw-bg: var(--ink-cw-bg);
    --cw-fg: var(--ink-cw-fg);
    --cw-sub: var(--ink-cw-sub);
    --cw-border: var(--ink-cw-border);
    --cw-soft: var(--ink-cw-soft);
    --cw-rail: var(--ink-cw-rail);
    --cw-preview-bg: var(--ink-cw-preview-bg);
    --cw-shadow: var(--ink-cw-shadow);

    position: absolute;
    left: 0;
    top: 0;
    width: 284px;
    padding: 12px;
    box-sizing: border-box;
    border-radius: 14px;
    background: var(--cw-bg);
    color: var(--cw-fg);
    border: 1px solid var(--cw-border);
    box-shadow: var(--cw-shadow);
    backdrop-filter: blur(22px) saturate(1.4);
    -webkit-backdrop-filter: blur(22px) saturate(1.4);
    display: flex;
    flex-direction: column;
    gap: 10px;
    cursor: default;
    pointer-events: auto;
    will-change: transform;
    z-index: 7;
  }

  @media (prefers-reduced-transparency: reduce) {
    .ink-cw-panel {
      --cw-bg: var(--ink-glass-solid);
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
  }

  .ink-cw-arrow {
    position: absolute;
    width: 10px;
    height: 10px;
    background: var(--cw-bg);
    border-right: 1px solid var(--cw-border);
    border-bottom: 1px solid var(--cw-border);
    transform: translateX(-50%) rotate(45deg);
    pointer-events: none;
  }

  .ink-cw-arrow[data-side='above'] {
    bottom: -6px;
  }

  .ink-cw-arrow[data-side='below'] {
    top: -6px;
    transform: translateX(-50%) rotate(225deg);
  }

  .ink-cw-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ink-cw-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .ink-cw-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--cw-sub);
    letter-spacing: 0.01em;
  }

  .ink-cw-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--cw-soft);
    color: var(--cw-sub);
  }

  .ink-cw-size-value {
    font-size: 12px;
    font-weight: 600;
    color: var(--cw-fg);
  }

  .ink-cw-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 6px;
  }

  .ink-cw-swatch {
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 999px;
    cursor: pointer;
    box-shadow: var(--ink-swatch-ring);
    transition: transform 100ms ease-out;
  }

  .ink-cw-swatch.light {
    box-shadow: var(--ink-swatch-ring-light);
  }

  .ink-cw-swatch:hover {
    transform: scale(1.1);
  }

  .ink-cw-swatch:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 2px;
  }

  .ink-cw-swatch.active {
    box-shadow:
      0 0 0 2px var(--cw-bg),
      0 0 0 3.5px var(--ink-accent);
  }

  .ink-cw-custom-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-top: 2px;
  }

  .ink-cw-sublabel {
    font-size: 11px;
    color: var(--cw-sub);
    white-space: nowrap;
  }

  .ink-cw-custom-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ink-cw-picker-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 999px;
    background: transparent;
    cursor: pointer;
  }

  .ink-cw-rainbow {
    width: 22px;
    height: 22px;
    border-radius: 999px;
    background: var(--ink-rainbow-wheel);
    box-shadow: var(--ink-rainbow-ring);
  }

  .ink-cw-color-native {
    position: absolute;
    inset: 0;
    opacity: 0;
    width: 0;
    height: 0;
    pointer-events: none;
  }

  .ink-cw-eye-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: var(--cw-soft);
    color: var(--cw-fg);
    cursor: pointer;
  }

  .ink-cw-picker-btn:focus-visible,
  .ink-cw-eye-btn:focus-visible,
  .ink-cw-recent-dot:focus-visible {
    outline: 2px solid var(--ink-focus-ring);
    outline-offset: 2px;
  }

  .ink-cw-eye-btn.active {
    background: var(--ink-cw-eye-active-bg);
    color: var(--ink-accent);
    box-shadow: inset 0 0 0 1.5px var(--ink-accent);
  }

  .ink-cw-recent {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 20px;
  }

  .ink-cw-recent-dot {
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 999px;
    cursor: pointer;
    box-shadow: var(--ink-recent-dot-ring);
  }

  .ink-cw-recent-dot.light {
    box-shadow: var(--ink-recent-dot-ring-light);
  }

  .ink-cw-recent-dot.active {
    box-shadow:
      0 0 0 2px var(--cw-bg),
      0 0 0 3.5px var(--ink-accent);
  }

  .ink-cw-font-row {
    position: relative;
    display: flex;
  }

  .ink-cw-font-btn {
    width: 100%;
    height: 30px;
    padding: 0 9px;
    border: 1px solid var(--cw-border);
    border-radius: 8px;
    background: var(--cw-soft);
    color: var(--cw-fg);
    font-size: 12.5px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
    transition:
      background-color 110ms ease,
      border-color 110ms ease;
  }

  .ink-cw-font-btn:hover {
    background: var(--ink-cw-soft-hover);
  }

  .ink-cw-font-btn.open {
    border-color: var(--ink-accent);
    box-shadow: inset 0 0 0 1px var(--ink-accent);
  }

  .ink-cw-font-btn:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 1px;
  }

  .ink-cw-font-name {
    flex: 1;
    min-width: 0;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ink-cw-font-caret {
    flex: none;
    color: var(--cw-sub);
    transition: transform 140ms ease;
  }

  .ink-cw-font-caret.open {
    transform: rotate(180deg);
    color: var(--ink-accent);
  }

  .ink-cw-size-body {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .ink-cw-slider-col {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ink-cw-track {
    position: relative;
    height: 24px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
  }

  .ink-cw-track:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 2px;
    border-radius: 6px;
  }

  .ink-cw-rail {
    position: relative;
    width: 100%;
    height: 4px;
    border-radius: 999px;
    background: var(--cw-rail);
  }

  .ink-cw-fill {
    position: absolute;
    left: 0;
    height: 4px;
    border-radius: 999px;
    background: var(--ink-accent);
    pointer-events: none;
  }

  .ink-cw-snap-dot {
    position: absolute;
    top: 50%;
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--cw-fg);
    opacity: 0.45;
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .ink-cw-snap-dot.snapped {
    opacity: 1;
    background: var(--ink-accent);
  }

  .ink-cw-thumb {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    border-radius: 999px;
    background: var(--ink-cw-thumb-bg);
    border: 2px solid var(--ink-accent);
    box-shadow: var(--ink-cw-thumb-shadow);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .ink-cw-thumb-tip {
    position: absolute;
    bottom: calc(100% + 7px);
    left: 50%;
    transform: translateX(-50%);
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--ink-accent);
    color: var(--ink-on-accent);
    font-size: 11px;
    font-weight: 700;
    white-space: nowrap;
    box-shadow: var(--ink-cw-thumb-tip-shadow);
  }

  .ink-cw-ticks {
    position: relative;
    height: 16px;
  }

  .ink-cw-tick-btn {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--cw-sub);
    font-size: 11px;
    line-height: 1.2;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-cw-tick-btn:hover,
  .ink-cw-tick-btn.active {
    color: var(--cw-fg);
    font-weight: 600;
  }

  .ink-cw-tick-btn:focus-visible {
    outline: 2px solid var(--ink-focus-ring);
    outline-offset: 2px;
    border-radius: 3px;
  }

  .ink-cw-preview-wrap {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
  }

  .ink-cw-preview-box {
    width: 52px;
    height: 52px;
    border-radius: 10px;
    background: var(--cw-preview-bg);
    border: 1px solid var(--cw-border);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .ink-cw-preview-dot {
    border-radius: 999px;
  }

  .ink-cw-preview-dot.light {
    box-shadow: var(--ink-swatch-ring-dark-on-light);
  }

  .ink-cw-preview-marker {
    width: 34px;
    border-radius: 4px;
    transform: rotate(-18deg);
  }

  .ink-cw-preview-text {
    font-weight: 600;
    line-height: 1;
    user-select: none;
  }

  .ink-cw-preview-mosaic {
    display: grid;
    gap: 1px;
    overflow: hidden;
    border-radius: 3px;
  }

  .ink-cw-preview-mosaic > span {
    background: var(--cw-fg);
  }

  .ink-cw-preview-eraser {
    border-radius: 999px;
    border: 1.5px dashed var(--cw-fg);
    background: var(--cw-soft);
  }

  .ink-cw-preview-scale {
    font-size: 10px;
    color: var(--cw-sub);
    white-space: nowrap;
  }
</style>
