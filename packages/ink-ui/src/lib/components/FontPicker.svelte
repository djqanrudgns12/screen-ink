<script>
  import { onDestroy, tick } from 'svelte';
  import Icon from '../icons/Icon.svelte';
  import { t } from '../i18n/index.svelte.js';
  import {
    FONT_POPOVER_HEIGHT,
    FONT_POPOVER_WIDTH,
    buildFontCatalog,
    buildFontFamilyStack,
    filterFontCatalog,
    normalizeFontWeight,
    placeFontPopover,
    shouldStartTypeSearch,
    stepFontRowIndex,
    stepWeightChipIndex,
  } from '../models/fontPicker.js';

  /**
   * Tidy식 글꼴 고르기 팝오버(R2-TXT-1·2·6 · 안 가):
   * - 별도 OS 창 없이 캡처/그리기 화면 안 트리거 바로 위에 밀착(약 264×320, 모자라면 아래 + 높이 압축)
   * - 묶음 탭(전체·고딕·명조·손글씨·둥근·제목·고정폭·내 글꼴) + 최근 3개
   * - 각 줄 이름을 그 글꼴과 굵기로 렌더링, 있는 굵기 알약만 표시, 호버/포커스 시 그 줄만 미리 보기
   * - 글자가 많을 때만 맨 위 검색(글자 치면 바로 검색), ↑↓·←→·Home/End·Enter·Esc, 선택 보존
   * @type {{
   *   open?: boolean,
   *   value?: string,
   *   weight?: number,
   *   recentFonts?: ReadonlyArray<string>,
   *   systemFonts?: ReadonlyArray<string>,
   *   anchorEl?: HTMLElement | null,
   *   onSelect?: (family: string, weight: number) => void,
   *   onPreview?: (preview: { family: string, weight: number } | null) => void,
   *   onClose?: () => void,
   * }}
   */
  let {
    open = false,
    value = 'Freesentation',
    weight = 400,
    recentFonts = [],
    systemFonts = [],
    anchorEl = null,
    onSelect = () => {},
    onPreview = () => {},
    onClose = () => {},
  } = $props();

  /** @type {HTMLDivElement | null} */
  let rootEl = $state(null);
  /** @type {HTMLDivElement | null} */
  let scrollEl = $state(null);
  /** @type {HTMLInputElement | null} */
  let searchInputEl = $state(null);

  /** @type {import('../models/fontPicker.js').FontTabId} */
  let activeTab = $state('all');
  let query = $state('');
  let searchRevealed = $state(false);
  /** @type {{ rowKey: string, family: string, weight: number } | null} */
  let hoveredChip = $state(null);
  let canScrollUp = $state(false);
  let canScrollDown = $state(false);
  let activeRowIdx = $state(-1);
  let activeChipIdx = $state(-1);

  let pos = $state({
    offsetLeft: 0,
    width: FONT_POPOVER_WIDTH,
    maxHeight: FONT_POPOVER_HEIGHT,
    side: /** @type {'above' | 'below'} */ ('above'),
    ready: false,
  });

  let catalog = $derived(buildFontCatalog(systemFonts));
  let filtered = $derived(
    filterFontCatalog(catalog, {
      category: activeTab,
      query,
      recent: recentFonts,
    }),
  );
  let showSearchBar = $derived(filtered.showSearch || searchRevealed);
  let normalizedWeight = $derived(normalizeFontWeight(weight));

  function updatePlacement() {
    if (!open || typeof window === 'undefined' || !anchorEl) return;
    const rect = anchorEl.getBoundingClientRect();
    const placed = placeFontPopover(rect, { w: window.innerWidth, h: window.innerHeight });
    pos = {
      offsetLeft: Math.round(placed.left - rect.left),
      width: placed.width,
      maxHeight: placed.maxHeight,
      side: placed.side,
      ready: true,
    };
  }

  function updateScrollFade() {
    if (!scrollEl) {
      canScrollUp = false;
      canScrollDown = false;
      return;
    }
    const { scrollTop, scrollHeight, clientHeight } = scrollEl;
    canScrollUp = scrollTop > 4;
    canScrollDown = scrollTop + clientHeight < scrollHeight - 4;
  }

  /**
   * 외부 스크롤을 건드리지 않고 내부 리스트만 맞춘다(Tidy scrollRowIntoList).
   * @param {HTMLElement} el
   * @param {'center' | 'nearest'} mode
   */
  function scrollRowIntoList(el, mode = 'nearest') {
    if (!scrollEl) return;
    const sRect = scrollEl.getBoundingClientRect();
    const rRect = el.getBoundingClientRect();
    if (mode === 'center') {
      const delta = rRect.top - sRect.top - (scrollEl.clientHeight - rRect.height) / 2;
      scrollEl.scrollTop = Math.max(0, scrollEl.scrollTop + delta);
    } else {
      const topGap = rRect.top - sRect.top;
      const bottomGap = rRect.bottom - sRect.bottom;
      if (topGap < 4) scrollEl.scrollTop = Math.max(0, scrollEl.scrollTop + topGap - 4);
      else if (bottomGap > -4) scrollEl.scrollTop = Math.max(0, scrollEl.scrollTop + bottomGap + 4);
    }
  }

  function clearPreview() {
    hoveredChip = null;
    onPreview(null);
  }

  /** @param {import('../models/fontPicker.js').FontFamilyItem} item @param {number} [explicitWeight] */
  function pickFont(item, explicitWeight) {
    const w = explicitWeight ?? item.defaultWeight;
    clearPreview();
    onSelect(item.name, w);
  }

  /** @param {string} rowKey @param {import('../models/fontPicker.js').FontFamilyItem} item @param {number} [recentWeight] */
  function displayVariant(rowKey, item, recentWeight) {
    if (hoveredChip && hoveredChip.rowKey === rowKey) {
      return item.variants.find((v) => v.weight === hoveredChip?.weight) ?? item.variants[0];
    }
    if (item.name.toLowerCase() === value.toLowerCase()) {
      const match = item.variants.find((v) => v.weight === normalizedWeight);
      if (match) return match;
    }
    if (recentWeight) {
      const match = item.variants.find((v) => v.weight === recentWeight);
      if (match) return match;
    }
    return item.variants.find((v) => v.weight === item.defaultWeight) ?? item.variants[0];
  }

  /** @param {number} idx */
  function focusRow(idx, chipIdx = -1) {
    activeRowIdx = idx;
    activeChipIdx = chipIdx;
    const row = filtered.flatRows[idx];
    if (!row || !scrollEl) return;
    const rowEl = scrollEl.querySelectorAll('[data-fp-row="true"]')[idx];
    if (rowEl instanceof HTMLElement) {
      scrollRowIntoList(rowEl, 'nearest');
      if (chipIdx >= 0) {
        const chipEl = rowEl.querySelectorAll('[data-fp-chip="true"]')[chipIdx];
        if (chipEl instanceof HTMLElement) {
          chipEl.focus({ preventScroll: true });
          const v = row.item.variants[chipIdx];
          if (v) {
            hoveredChip = { rowKey: row.key, family: v.family, weight: v.weight };
            onPreview({ family: row.item.name, weight: v.weight });
          }
          return;
        }
      }
      clearPreview();
      rowEl.focus({ preventScroll: true });
      const defVar = displayVariant(row.key, row.item, row.variantWeight);
      onPreview({ family: row.item.name, weight: defVar.weight });
    }
  }

  /** @param {KeyboardEvent} e */
  function handleKeydownCapture(e) {
    if (!open) return;
    const rows = filtered.flatRows;

    if (e.key === 'Escape' || e.code === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (query && document.activeElement === searchInputEl) {
        query = '';
        return;
      }
      clearPreview();
      onClose();
      return;
    }

    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      e.stopPropagation();
      const next = stepFontRowIndex(activeRowIdx, e.key, rows.length);
      if (next >= 0) focusRow(next, -1);
      return;
    }

    if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && document.activeElement !== searchInputEl) {
      const idx = activeRowIdx >= 0 ? activeRowIdx : 0;
      const row = rows[idx];
      if (row && row.item.variants.length > 1) {
        e.preventDefault();
        e.stopPropagation();
        const nextChip = stepWeightChipIndex(activeChipIdx, e.key, row.item.variants.length);
        focusRow(idx, nextChip);
      }
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      const idx = activeRowIdx >= 0 ? activeRowIdx : 0;
      const row = rows[idx];
      if (row) {
        const chipVar = activeChipIdx >= 0 ? row.item.variants[activeChipIdx] : null;
        const chosenWeight = chipVar?.weight ?? row.variantWeight ?? row.item.defaultWeight;
        pickFont(row.item, chosenWeight);
      }
      return;
    }

    // 검색창 밖에서 글자를 치면 즉시 검색창으로(R2-TXT-6)
    if (document.activeElement !== searchInputEl && shouldStartTypeSearch(e)) {
      e.stopPropagation();
      searchRevealed = true;
      void tick().then(() => {
        if (!searchInputEl) return;
        searchInputEl.focus({ preventScroll: true });
        query = `${query}${e.key}`;
      });
      e.preventDefault();
    }
  }

  /** @param {PointerEvent} e */
  function handlePointerDownCapture(e) {
    if (!open) return;
    const target = /** @type {Node | null} */ (e.target);
    if (!target) return;
    if (rootEl?.contains(target) || anchorEl?.contains(target)) return;
    clearPreview();
    onClose();
  }

  $effect(() => {
    if (!open) {
      clearPreview();
      return;
    }
    activeTab = 'all';
    query = '';
    searchRevealed = false;
    activeChipIdx = -1;
    updatePlacement();
    void tick().then(() => {
      updatePlacement();
      const selectedIdx = filtered.flatRows.findIndex(
        (r) => !r.isRecent && r.item.name.toLowerCase() === value.toLowerCase(),
      );
      activeRowIdx = selectedIdx >= 0 ? selectedIdx : filtered.flatRows.length > 0 ? 0 : -1;
      if (selectedIdx >= 0 && scrollEl) {
        const rowEl = scrollEl.querySelectorAll('[data-fp-row="true"]')[selectedIdx];
        if (rowEl instanceof HTMLElement) scrollRowIntoList(rowEl, 'center');
      }
      updateScrollFade();
    });
  });

  $effect(() => {
    void filtered;
    void tick().then(updateScrollFade);
  });

  $effect(() => {
    if (!open || typeof window === 'undefined') return;
    window.addEventListener('keydown', handleKeydownCapture, true);
    window.addEventListener('pointerdown', handlePointerDownCapture, true);
    window.addEventListener('resize', updatePlacement);
    return () => {
      window.removeEventListener('keydown', handleKeydownCapture, true);
      window.removeEventListener('pointerdown', handlePointerDownCapture, true);
      window.removeEventListener('resize', updatePlacement);
    };
  });

  onDestroy(clearPreview);
</script>

{#if open}
  <div
    bind:this={rootEl}
    class="ink-fp"
    data-side={pos.side}
    data-ready={pos.ready ? 'true' : undefined}
    role="dialog"
    tabindex="-1"
    aria-label={t('font.picker.label')}
    style:position="absolute"
    style:left={`${pos.offsetLeft}px`}
    style:top={pos.side === 'below' ? 'calc(100% + 6px)' : 'auto'}
    style:bottom={pos.side === 'above' ? 'calc(100% + 6px)' : 'auto'}
    style:width={`${pos.width}px`}
    style:max-height={`${pos.maxHeight}px`}
    onpointerdown={(e) => e.stopPropagation()}
    onpointermove={(e) => e.stopPropagation()}
    onpointerup={(e) => e.stopPropagation()}
    onwheel={(e) => e.stopPropagation()}
    ondblclick={(e) => e.stopPropagation()}
  >
    {#if showSearchBar}
      <div class="ink-fp-search">
        <span class="ink-fp-search-icon" aria-hidden="true">
          <Icon name="zoom" size={13} accent={false} />
        </span>
        <input
          bind:this={searchInputEl}
          type="text"
          class="ink-fp-search-input"
          value={query}
          placeholder={t('font.picker.searchPlaceholder')}
          aria-label={t('font.picker.searchLabel')}
          oninput={(e) => {
            query = /** @type {HTMLInputElement} */ (e.currentTarget).value;
            activeRowIdx = 0;
            activeChipIdx = -1;
            if (scrollEl) scrollEl.scrollTop = 0;
          }}
          onkeydown={(e) => e.stopPropagation()}
        />
        {#if query}
          <button
            type="button"
            class="ink-fp-search-clear"
            aria-label={t('font.picker.clearSearch')}
            onmousedown={(e) => e.preventDefault()}
            onclick={(e) => {
              e.stopPropagation();
              query = '';
              searchInputEl?.focus();
            }}
          >
            <Icon name="close" size={11} accent={false} />
          </button>
        {/if}
      </div>
    {/if}

    <div class="ink-fp-tabs" role="tablist" aria-label={t('font.picker.tabsLabel')}>
      {#each filtered.tabs as tabId (tabId)}
        {@const active = filtered.activeTab === tabId}
        <button
          type="button"
          role="tab"
          aria-selected={active}
          class="ink-fp-tab"
          class:is-active={active}
          data-font-tab={tabId}
          onmousedown={(e) => e.preventDefault()}
          onclick={(e) => {
            e.stopPropagation();
            activeTab = activeTab === tabId && tabId !== 'all' ? 'all' : tabId;
            activeRowIdx = 0;
            activeChipIdx = -1;
            clearPreview();
            if (scrollEl) scrollEl.scrollTop = 0;
          }}
        >
          {t(`font.tab.${tabId}`)}
        </button>
      {/each}
    </div>

    <div class="ink-fp-list-wrap">
      <div class="ink-fp-fade ink-fp-fade-top" class:is-visible={canScrollUp} aria-hidden="true"></div>

      <div
        id="ink-font-picker-listbox"
        bind:this={scrollEl}
        class="ink-fp-scroll"
        role="listbox"
        aria-label={t('font.picker.label')}
        tabindex="-1"
        onscroll={updateScrollFade}
      >
        {#if filtered.sections.length === 0}
          <div class="ink-fp-empty">{t('font.picker.empty')}</div>
        {:else}
          {#each filtered.sections as sec (sec.id)}
            <section class="ink-fp-sec" aria-label={t(sec.titleKey)}>
              {#if filtered.sections.length > 1 || sec.id === 'recent' || sec.id === 'mine'}
                <div class="ink-fp-sec-head" aria-hidden="true">
                  <span class="ink-fp-sec-dot"></span>
                  <span class="ink-fp-sec-title">{t(sec.titleKey)}</span>
                  <span class="ink-fp-sec-rule"></span>
                </div>
              {/if}

              {#each sec.rows as row (row.key)}
                {@const isSelected = row.item.name.toLowerCase() === value.toLowerCase()}
                {@const isMulti = row.item.variants.length > 1}
                {@const disp = displayVariant(row.key, row.item, row.variantWeight)}
                {@const rowIndex = filtered.flatRows.indexOf(row)}

                <div
                  role="option"
                  tabindex="0"
                  aria-selected={isSelected}
                  data-fp-row="true"
                  data-font-family={row.item.name}
                  data-selected={isSelected ? 'true' : undefined}
                  class="ink-fp-row"
                  class:is-selected={isSelected}
                  onmousedown={(e) => e.preventDefault()}
                  onpointerenter={() => {
                    activeRowIdx = rowIndex;
                    activeChipIdx = -1;
                    onPreview({ family: row.item.name, weight: disp.weight });
                  }}
                  onpointerleave={() => {
                    if (!hoveredChip) onPreview(null);
                  }}
                  onclick={(e) => {
                    e.stopPropagation();
                    pickFont(row.item, row.variantWeight ?? row.item.defaultWeight);
                  }}
                  onkeydown={(e) => {
                    if (e.target !== e.currentTarget) return;
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      e.stopPropagation();
                      pickFont(row.item, row.variantWeight ?? row.item.defaultWeight);
                    }
                  }}
                >
                  <span
                    class="ink-fp-name"
                    style:font-family={buildFontFamilyStack(disp.family)}
                    style:font-weight={disp.weight}
                  >
                    {row.item.name}
                  </span>

                  {#if isMulti}
                    <span class="ink-fp-chips" role="group" aria-label={t('font.picker.weightsLabel', { name: row.item.name })}>
                      {#each row.item.variants as v (v.weight + ':' + v.family)}
                        {@const chipActive = isSelected && normalizedWeight === v.weight}
                        <button
                          type="button"
                          data-fp-chip="true"
                          data-weight={v.weight}
                          class="ink-fp-chip"
                          class:is-active={chipActive}
                          aria-label={`${row.item.name} ${v.chip}`}
                          aria-pressed={chipActive}
                          onmousedown={(e) => {
                            e.stopPropagation();
                            e.preventDefault();
                          }}
                          onpointerenter={() => {
                            hoveredChip = { rowKey: row.key, family: v.family, weight: v.weight };
                            onPreview({ family: row.item.name, weight: v.weight });
                          }}
                          onpointerleave={() => {
                            if (hoveredChip?.rowKey === row.key) clearPreview();
                          }}
                          onfocus={() => {
                            hoveredChip = { rowKey: row.key, family: v.family, weight: v.weight };
                            onPreview({ family: row.item.name, weight: v.weight });
                          }}
                          onblur={() => {
                            if (hoveredChip?.rowKey === row.key) clearPreview();
                          }}
                          onclick={(e) => {
                            e.stopPropagation();
                            pickFont(row.item, v.weight);
                          }}
                        >
                          {v.chip}
                        </button>
                      {/each}
                    </span>
                  {:else if isSelected}
                    <span class="ink-fp-check" aria-hidden="true">
                      <Icon name="check" size={12} accent={true} />
                    </span>
                  {/if}
                </div>
              {/each}
            </section>
          {/each}
        {/if}
      </div>

      <div class="ink-fp-fade ink-fp-fade-bottom" class:is-visible={canScrollDown} aria-hidden="true"></div>
    </div>
  </div>
{/if}

<style>
  .ink-fp {
    --fp-bg: rgba(20, 21, 25, 0.94);
    --fp-fg: #f7f8fa;
    --fp-muted: #9ca3af;
    --fp-border: rgba(255, 255, 255, 0.14);
    --fp-line: rgba(255, 255, 255, 0.08);
    --fp-bar: rgba(255, 255, 255, 0.04);
    --fp-soft: rgba(255, 255, 255, 0.08);
    --fp-hover: rgba(255, 159, 10, 0.14);
    --fp-selected: rgba(255, 159, 10, 0.22);
    --fp-chip-bg: rgba(255, 255, 255, 0.08);
    --fp-fade: rgba(20, 21, 25, 0.96);
    --fp-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1), 0 18px 46px rgba(0, 0, 0, 0.52);

    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: var(--fp-bg);
    color: var(--fp-fg);
    border: 1px solid var(--fp-border);
    box-shadow: var(--fp-shadow);
    backdrop-filter: blur(22px) saturate(1.4);
    -webkit-backdrop-filter: blur(22px) saturate(1.4);
    overflow: hidden;
    user-select: none;
    z-index: 30;
    pointer-events: auto;
    font-family: var(--ink-font-body);
  }

  :global([data-theme='light']) .ink-fp {
    --fp-bg: rgba(250, 250, 248, 0.96);
    --fp-fg: #141414;
    --fp-muted: #5e5e5b;
    --fp-border: rgba(0, 0, 0, 0.13);
    --fp-line: rgba(0, 0, 0, 0.08);
    --fp-bar: rgba(0, 0, 0, 0.03);
    --fp-soft: rgba(0, 0, 0, 0.06);
    --fp-hover: rgba(217, 119, 6, 0.12);
    --fp-selected: rgba(217, 119, 6, 0.2);
    --fp-chip-bg: rgba(0, 0, 0, 0.055);
    --fp-fade: rgba(250, 250, 248, 0.98);
    --fp-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.85), 0 18px 42px rgba(0, 0, 0, 0.2);
  }

  @media (prefers-color-scheme: light) {
    :global(:root:not([data-theme='dark'])) .ink-fp {
      --fp-bg: rgba(250, 250, 248, 0.96);
      --fp-fg: #141414;
      --fp-muted: #5e5e5b;
      --fp-border: rgba(0, 0, 0, 0.13);
      --fp-line: rgba(0, 0, 0, 0.08);
      --fp-bar: rgba(0, 0, 0, 0.03);
      --fp-soft: rgba(0, 0, 0, 0.06);
      --fp-hover: rgba(217, 119, 6, 0.12);
      --fp-selected: rgba(217, 119, 6, 0.2);
      --fp-chip-bg: rgba(0, 0, 0, 0.055);
      --fp-fade: rgba(250, 250, 248, 0.98);
      --fp-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.85), 0 18px 42px rgba(0, 0, 0, 0.2);
    }
  }

  @media (prefers-reduced-transparency: reduce) {
    .ink-fp {
      --fp-bg: #18191d;
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
    :global([data-theme='light']) .ink-fp {
      --fp-bg: #fafaf8;
    }
  }

  .ink-fp-search {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 8px;
    background: var(--fp-bar);
    border-bottom: 1px solid var(--fp-line);
  }

  .ink-fp-search-icon {
    display: inline-flex;
    color: var(--fp-muted);
    flex: none;
  }

  .ink-fp-search-input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--fp-fg);
    font-family: inherit;
    font-size: 12px;
    outline: none;
  }

  .ink-fp-search-input::placeholder {
    color: var(--fp-muted);
  }

  .ink-fp-search-clear {
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 999px;
    background: var(--fp-soft);
    color: var(--fp-muted);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .ink-fp-tabs {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px;
    padding: 5px 6px;
    background: var(--fp-bar);
    border-bottom: 1px solid var(--fp-line);
  }

  .ink-fp-tab {
    height: 21px;
    padding: 0 6px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--fp-muted);
    font-family: inherit;
    font-size: 10.5px;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background-color 100ms ease,
      color 100ms ease;
  }

  .ink-fp-tab:hover {
    background: var(--fp-hover);
    color: var(--ink-accent);
  }

  .ink-fp-tab.is-active {
    background: var(--ink-accent);
    color: #111315;
    font-weight: 700;
  }

  .ink-fp-list-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ink-fp-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 5px 5px 7px;
    scrollbar-gutter: stable;
    outline: none;
  }

  .ink-fp-scroll::-webkit-scrollbar {
    width: 5px;
  }

  .ink-fp-scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .ink-fp-scroll::-webkit-scrollbar-thumb {
    background: rgba(255, 159, 10, 0.3);
    border-radius: 999px;
  }

  .ink-fp-fade {
    position: absolute;
    left: 0;
    right: 6px;
    height: 14px;
    pointer-events: none;
    opacity: 0;
    z-index: 4;
    transition: opacity 120ms ease;
  }

  .ink-fp-fade.is-visible {
    opacity: 1;
  }

  .ink-fp-fade-top {
    top: 0;
    background: linear-gradient(to bottom, var(--fp-fade), transparent);
  }

  .ink-fp-fade-bottom {
    bottom: 0;
    background: linear-gradient(to top, var(--fp-fade), transparent);
  }

  .ink-fp-sec-head {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 6px 3px 7px;
  }

  .ink-fp-sec:first-child .ink-fp-sec-head {
    padding-top: 2px;
  }

  .ink-fp-sec-dot {
    width: 5px;
    height: 5px;
    border-radius: 999px;
    background: var(--ink-accent);
    opacity: 0.85;
    flex: none;
  }

  .ink-fp-sec-title {
    font-size: 10px;
    font-weight: 700;
    color: var(--fp-muted);
    white-space: nowrap;
  }

  .ink-fp-sec-rule {
    flex: 1;
    height: 1px;
    background: var(--fp-line);
  }

  .ink-fp-row {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    min-height: 30px;
    padding: 3px 6px 3px 10px;
    margin-bottom: 1px;
    border-radius: 8px;
    cursor: pointer;
    box-sizing: border-box;
    outline: none;
    transition: background-color 100ms ease;
  }

  .ink-fp-row::before {
    content: '';
    position: absolute;
    left: 2px;
    top: 5px;
    bottom: 5px;
    width: 2.5px;
    border-radius: 999px;
    background: var(--ink-accent);
    opacity: 0;
    transform: scaleY(0.45);
    transition:
      opacity 100ms ease,
      transform 120ms ease-out;
  }

  .ink-fp-row:hover,
  .ink-fp-row:focus-visible {
    background: var(--fp-hover);
  }

  .ink-fp-row:hover::before,
  .ink-fp-row:focus-visible::before,
  .ink-fp-row.is-selected::before {
    opacity: 1;
    transform: scaleY(1);
  }

  .ink-fp-row.is-selected {
    background: var(--fp-selected);
  }

  .ink-fp-name {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fp-fg);
  }

  .ink-fp-row.is-selected .ink-fp-name {
    color: var(--ink-accent);
  }

  .ink-fp-check {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }

  .ink-fp-chips {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }

  .ink-fp-chip {
    height: 19px;
    min-width: 19px;
    padding: 0 4px;
    border-radius: 5px;
    border: 1px solid var(--fp-line);
    background: var(--fp-chip-bg);
    color: var(--fp-muted);
    font-family: var(--ink-font-body);
    font-size: 9.5px;
    font-weight: 700;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .ink-fp-chip:hover,
  .ink-fp-chip:focus-visible,
  .ink-fp-chip.is-active {
    background: var(--ink-accent);
    color: #111315;
    border-color: var(--ink-accent);
    outline: none;
  }

  .ink-fp-empty {
    padding: 28px 12px;
    text-align: center;
    font-size: 12px;
    color: var(--fp-muted);
  }
</style>
