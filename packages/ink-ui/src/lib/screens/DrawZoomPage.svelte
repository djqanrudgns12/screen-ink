<script>
  import { Slider } from 'bits-ui';
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Switch from '../components/Switch.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { locale, syncLang, t } from '../i18n/index.svelte.js';
  import { defaultSlotCombo, formatComboReadable } from '../models/shortcutDisplay.js';
  import { formatShapeOnHoldSub } from '../models/settingsMeta.js';
  import { buildBoardPreviewSamples } from '../models/drawUi.js';

  /**
   * S04 설정 · 그리기·확대 화면(F06 · R2-FIX-12).
   * @type {{ client: import('@screen-ink/client').InkClient }}
   */
  let { client } = $props();

  const PEN_COLOR_META = [
    { key: 'R', lightFg: false },
    { key: 'Y', lightFg: false },
    { key: 'G', lightFg: false },
    { key: 'O', lightFg: false },
    { key: 'B', lightFg: true },
    { key: 'L', lightFg: false },
  ];

  let state = $state(null);

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (!alive) return;
      state = s;
      syncLang(s);
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        state = e.state;
        syncLang(e.state);
      }
    });
    return () => {
      alive = false;
      off();
    };
  });

  let settings = $derived(state?.settings ?? {});
  let platform = $derived(state?.platform ?? 'windows');
  let penColor = $derived(settings['draw.color'] ?? 'R');
  let penWidth = $derived(Number(settings['draw.width'] ?? 4));
  let shapeApply = $derived(settings['draw.shapeApply'] ?? 'suggest');
  let dUndoCombo = $derived(state?.shortcuts?.find((s) => s.id === 'd-undo')?.combo ?? '');
  let restoreCombo = $derived(
    state?.shortcuts?.find((s) => s.id === 'restore')?.combo ?? defaultSlotCombo('restore', platform),
  );
  let restoreKeyLabel = $derived(
    formatComboReadable(restoreCombo || defaultSlotCombo('restore', platform), platform),
  );
  let boardCombo = $derived(
    state?.shortcuts?.find((s) => s.id === 'board')?.combo ?? defaultSlotCombo('board', platform),
  );
  let boardShortcut = $derived(
    formatComboReadable(boardCombo || defaultSlotCombo('board', platform), platform),
  );
  let shapeOnHoldSub = $derived(formatShapeOnHoldSub(shapeApply, platform, locale.lang, dUndoCombo));
  // "멈추면 도형으로"가 꺼져 있으면 도형 바꾸는 방식은 쓸 일이 없다 — 들여쓰기+흐리게
  let shapeOff = $derived(!settings['draw.shapeOnHold']);
  let boardPreviews = $derived(buildBoardPreviewSamples({ penColor, penWidth, lang: locale.lang }));

  function setSetting(path, value) {
    client.updateSetting(path, value);
  }
</script>

<div class="ink-page">
  <header class="ink-page-head" data-tauri-drag-region>
    <h1 class="ink-page-title" data-tauri-drag-region>{t('page.draw.title')}</h1>
  </header>

  <div class="ink-page-body ink-scroll">
  <section class="ink-section">
    <SectionLabel title={t('page.draw.section.pen')} />
    <Group>
      <Row label={t('page.draw.color')}>
        <div role="radiogroup" aria-label={t('page.draw.colorAria')} class="ink-pen-colors">
          {#each PEN_COLOR_META as c (c.key)}
            {@const colorName = t(`capture.color.${c.key}`)}
            <button
              type="button"
              role="radio"
              aria-checked={penColor === c.key}
              aria-label={t('page.draw.colorItemAria', { color: colorName, key: c.key })}
              class="ink-pen-swatch ink-mono ink-focus"
              data-selected={penColor === c.key ? 'true' : undefined}
              style:background={`var(--ink-pen-${c.key})`}
              style:color={c.lightFg ? 'var(--ink-pen-fg-light)' : 'var(--ink-pen-fg-dark)'}
              onclick={() => setSetting('draw.color', c.key)}
            >
              {c.key}
            </button>
          {/each}
        </div>
      </Row>
      <Row label={t('page.draw.width')} sub={t('page.draw.widthSub')}>
        <div class="ink-slider-wrap">
          <Slider.Root
            type="single"
            min={1}
            max={30}
            step={1}
            value={penWidth}
            onValueChange={(v) => setSetting('draw.width', v)}
            aria-label={t('page.draw.widthAria')}
            class="ink-slider"
          >
            <span class="ink-slider-track">
              <Slider.Range class="ink-slider-range" />
            </span>
            <Slider.Thumb index={0} class="ink-slider-thumb ink-focus" />
          </Slider.Root>
          <span class="ink-slider-val ink-mono">{penWidth}px</span>
        </div>
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title={t('page.draw.section.assist')} />
    <Group>
      <Row label={t('page.draw.smoothing')}>
        <Segmented
          label={t('page.draw.smoothing')}
          value={settings['draw.smoothing'] ?? 'light'}
          options={[
            { value: 'off', label: t('page.draw.smoothing.off') },
            { value: 'light', label: t('page.draw.smoothing.light') },
            { value: 'strong', label: t('page.draw.smoothing.strong') },
          ]}
          onChange={(v) => setSetting('draw.smoothing', v)}
        />
      </Row>
      <Row label={t('page.draw.straighten')} sub="±4°" subMono={true}>
        <Switch
          checked={Boolean(settings['draw.straighten'])}
          label={t('page.draw.straighten')}
          onCheckedChange={(v) => setSetting('draw.straighten', v)}
        />
      </Row>
      <Row label={t('page.draw.shapeOnHold')} sub={shapeOnHoldSub}>
        <Switch
          checked={Boolean(settings['draw.shapeOnHold'])}
          label={t('page.draw.shapeOnHold')}
          onCheckedChange={(v) => setSetting('draw.shapeOnHold', v)}
        />
      </Row>
      <Row
        label={t('page.draw.shapeApply')}
        class={shapeOff ? 'ink-row-sub-item ink-row-off' : 'ink-row-sub-item'}
      >
        <Segmented
          label={t('page.draw.shapeApply')}
          value={settings['draw.shapeApply'] ?? 'suggest'}
          disabled={shapeOff}
          options={[
            { value: 'suggest', label: t('page.draw.shapeApply.suggest') },
            { value: 'auto', label: t('page.draw.shapeApply.auto') },
          ]}
          onChange={(v) => setSetting('draw.shapeApply', v)}
        />
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title={t('page.draw.section.draw')} />
    <Group>
      <Row label={t('page.draw.startZoom')}>
        <Segmented
          label={t('page.draw.startZoom')}
          mono={true}
          value={Number(settings['draw.startZoom'] ?? 1.75)}
          options={[
            { value: 1.25, label: '1.25' },
            { value: 1.5, label: '1.5' },
            { value: 1.75, label: '1.75' },
            { value: 2, label: '2' },
            { value: 3, label: '3' },
          ]}
          onChange={(v) => setSetting('draw.startZoom', Number(v))}
        />
      </Row>
      <Row label={t('page.draw.keep')} sub={t('page.draw.keepSub', { key: restoreKeyLabel })}>
        <Switch
          checked={Boolean(settings['draw.keep'])}
          label={t('page.draw.keep')}
          onCheckedChange={(v) => setSetting('draw.keep', v)}
        />
      </Row>
      <Row label={t('page.draw.esc')}>
        <Segmented
          label={t('page.draw.escAria')}
          value={settings['draw.esc'] ?? 'step'}
          options={[
            { value: 'step', label: t('page.draw.esc.step') },
            { value: 'exit', label: t('page.draw.esc.exit') },
          ]}
          onChange={(v) => setSetting('draw.esc', v)}
        />
      </Row>
      <Row label={t('page.draw.rightClickPan')} sub={t('page.draw.rightClickPanSub')}>
        <Switch
          checked={Boolean(settings['draw.rightClickPan'])}
          label={t('page.draw.rightClickPan')}
          onCheckedChange={(v) => setSetting('draw.rightClickPan', v)}
        />
      </Row>
      <Row
        label={t('page.draw.boardPreview')}
        sub={t('page.draw.boardPreviewSub', { shortcut: boardShortcut })}
      >
        <div class="ink-board-previews" role="img" aria-label={t('page.draw.boardPreviewAria')}>
          {#each boardPreviews as board (board.id)}
            <div
              class="ink-board-mini"
              data-board={board.id}
              style:background={board.bg}
              title={board.substituted ? t('page.draw.boardPreviewSubstituted', { label: board.label }) : board.label}
            >
              <svg viewBox="0 0 84 40" class="ink-board-mini-svg" aria-hidden="true">
                <path
                  d="M 10 24 C 22 10, 34 30, 48 16 S 66 24, 74 14"
                  fill="none"
                  stroke={board.strokeColor}
                  stroke-width={board.strokeWidth}
                  stroke-linecap="round"
                />
              </svg>
              <span class="ink-board-mini-tag">
                <kbd class="ink-mono">{board.key}</kbd>
                <span>{board.id === 'white' ? t('page.draw.boardBgWhite') : t('page.draw.boardBgBlack')}</span>
              </span>
            </div>
          {/each}
        </div>
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title={t('page.draw.section.zoom')} />
    <Group>
      <Row label={t('page.draw.zoomFollow')}>
        <Segmented
          label={t('page.draw.zoomFollow')}
          value={settings['zoom.follow'] ?? 'pointer'}
          options={[
            { value: 'pointer', label: t('page.draw.zoomFollow.pointer') },
            { value: 'smooth', label: t('page.draw.zoomFollow.smooth') },
            { value: 'instant', label: t('page.draw.zoomFollow.instant') },
          ]}
          onChange={(v) => setSetting('zoom.follow', v)}
        />
      </Row>
      <Row label={t('page.draw.zoomMinimap')} sub={t('page.draw.zoomMinimapSub')}>
        <Switch
          checked={Boolean(settings['zoom.minimap'])}
          label={t('page.draw.zoomMinimap')}
          onCheckedChange={(v) => setSetting('zoom.minimap', v)}
        />
      </Row>
    </Group>
  </section>
  </div>
</div>

<style>
  /* R2-FIX-12: "도형 바꾸는 방식"을 "멈추면 도형으로" 하위 행으로 들여쓰기·연결 표시 */
  :global(.ink-row-wrap.ink-row-sub-item .ink-row) {
    padding-left: 28px;
    position: relative;
  }

  :global(.ink-row-wrap.ink-row-sub-item .ink-row::before) {
    content: '';
    position: absolute;
    left: 14px;
    top: 8px;
    bottom: 14px;
    width: 7px;
    border-left: 1.5px solid var(--ink-line-strong);
    border-bottom: 1.5px solid var(--ink-line-strong);
    border-bottom-left-radius: 3px;
  }

  :global(.ink-row-wrap.ink-row-off) {
    opacity: 0.45;
    pointer-events: none;
  }

  .ink-page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ink-page-head {
    flex: none;
    padding: 18px 60px 10px 26px;
  }

  .ink-page-title {
    margin: 0;
    font-size: 19px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
    user-select: none;
  }

  .ink-page-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    padding: 2px 20px 22px 26px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .ink-section {
    display: flex;
    flex-direction: column;
  }

  .ink-pen-colors {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .ink-pen-swatch {
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 50%;
    padding: 0;
    display: grid;
    place-items: center;
    font-size: 9.5px;
    font-weight: 500;
    cursor: pointer;
  }

  .ink-pen-swatch[data-selected='true'] {
    box-shadow: 0 0 0 2px var(--ink-group-bg), 0 0 0 3.5px var(--ink-text);
  }

  .ink-slider-wrap {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }

  :global(.ink-slider) {
    position: relative;
    display: flex;
    align-items: center;
    width: 150px;
    height: 18px;
    touch-action: none;
    user-select: none;
    cursor: pointer;
  }

  .ink-slider-track {
    position: relative;
    display: block;
    width: 100%;
    height: 4px;
    border-radius: 2px;
    background: var(--ink-line);
    overflow: hidden;
  }

  :global(.ink-slider-range) {
    position: absolute;
    height: 100%;
    background: var(--ink-invert);
  }

  :global(.ink-slider-thumb) {
    display: block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--ink-surface);
    box-shadow: var(--ink-shadow-slider-thumb);
    cursor: grab;
  }

  .ink-slider-val {
    font-size: 11.5px;
    min-width: 28px;
    text-align: right;
    color: var(--ink-text);
  }

  /* R2-FIX-12: 단추가 아닌 비대화형 미리보기 프레임으로 구분 */
  .ink-board-previews {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px;
    border-radius: 8px;
    border: 1px dashed var(--ink-line);
    background: var(--ink-raised);
    cursor: default;
    user-select: none;
    pointer-events: none;
  }

  .ink-board-mini {
    position: relative;
    width: 90px;
    height: 42px;
    border-radius: calc(var(--ink-r-row) - 1px);
    box-sizing: border-box;
    overflow: hidden;
    flex: none;
  }

  .ink-board-mini[data-board='white'] {
    box-shadow: inset 0 0 0 1px var(--ink-line);
    color: #5e5e5b;
  }

  .ink-board-mini[data-board='black'] {
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.14);
    color: #c9ccd3;
  }

  .ink-board-mini-svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .ink-board-mini-tag {
    position: absolute;
    left: 6px;
    bottom: 4px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 9.5px;
    line-height: 1;
    pointer-events: none;
  }

  .ink-board-mini-tag kbd {
    font-size: 9px;
    font-weight: 600;
    opacity: 0.9;
  }
</style>
