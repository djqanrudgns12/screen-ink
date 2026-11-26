<script>
  import { Slider } from 'bits-ui';
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Switch from '../components/Switch.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { formatShapeOnHoldSub } from '../models/settingsMeta.js';
  import { buildBoardPreviewSamples } from '../models/drawUi.js';

  /**
   * S04 설정 · 판서·확대 화면(F06).
   * @type {{ client: import('@screen-ink/client').InkClient }}
   */
  let { client } = $props();

  const PEN_COLORS = [
    { key: 'R', label: '빨강', lightFg: false },
    { key: 'Y', label: '노랑', lightFg: false },
    { key: 'G', label: '초록', lightFg: false },
    { key: 'O', label: '주황', lightFg: false },
    { key: 'B', label: '파랑', lightFg: true },
    { key: 'L', label: '연두', lightFg: false },
  ];

  let state = $state(null);

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (alive) state = s;
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') state = e.state;
    });
    return () => {
      alive = false;
      off();
    };
  });

  let settings = $derived(state?.settings ?? {});
  let penColor = $derived(settings['draw.color'] ?? 'R');
  let penWidth = $derived(Number(settings['draw.width'] ?? 4));
  let shapeApply = $derived(settings['draw.shapeApply'] ?? 'suggest');
  let shapeOnHoldSub = $derived(formatShapeOnHoldSub(shapeApply));
  // "멈추면 도형으로"가 꺼져 있으면 도형 보정 방식은 쓸 일이 없다 — 흐리게
  let shapeOff = $derived(!settings['draw.shapeOnHold']);
  let boardShortcut = $derived(state?.platform === 'macos' ? '⌥3' : 'Alt+3');
  let boardPreviews = $derived(buildBoardPreviewSamples({ penColor, penWidth }));

  function setSetting(path, value) {
    client.updateSetting(path, value);
  }
</script>

<div class="ink-page">
  <header class="ink-page-head" data-tauri-drag-region>
    <h1 class="ink-page-title" data-tauri-drag-region>판서·확대</h1>
  </header>

  <div class="ink-page-body ink-scroll">
  <section class="ink-section">
    <SectionLabel title="펜" />
    <Group>
      <Row label="처음 색">
        <div role="radiogroup" aria-label="처음 펜 색" class="ink-pen-colors">
          {#each PEN_COLORS as c (c.key)}
            <button
              type="button"
              role="radio"
              aria-checked={penColor === c.key}
              aria-label={`펜 색 ${c.label} (${c.key})`}
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
      <Row label="굵기" sub="Ctrl+휠">
        <div class="ink-slider-wrap">
          <Slider.Root
            type="single"
            min={1}
            max={30}
            step={1}
            value={penWidth}
            onValueChange={(v) => setSetting('draw.width', v)}
            aria-label="펜 굵기"
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
    <SectionLabel title="보정" />
    <Group>
      <Row label="떨림 보정">
        <Segmented
          label="떨림 보정"
          value={settings['draw.smoothing'] ?? 'light'}
          options={[
            { value: 'off', label: '끔' },
            { value: 'light', label: '약' },
            { value: 'strong', label: '강' },
          ]}
          onChange={(v) => setSetting('draw.smoothing', v)}
        />
      </Row>
      <Row label="거의 곧은 선은 곧게" sub="±4°" subMono={true}>
        <Switch
          checked={Boolean(settings['draw.straighten'])}
          label="거의 곧은 선은 곧게"
          onCheckedChange={(v) => setSetting('draw.straighten', v)}
        />
      </Row>
      <Row label="멈추면 도형으로" sub={shapeOnHoldSub}>
        <Switch
          checked={Boolean(settings['draw.shapeOnHold'])}
          label="멈추면 도형으로"
          onCheckedChange={(v) => setSetting('draw.shapeOnHold', v)}
        />
      </Row>
      <Row label="도형 보정" class={shapeOff ? 'ink-row-off' : ''}>
        <Segmented
          label="도형 보정"
          value={settings['draw.shapeApply'] ?? 'suggest'}
          disabled={shapeOff}
          options={[
            { value: 'suggest', label: '물어보고 바꾸기' },
            { value: 'auto', label: '바로 바꾸기' },
          ]}
          onChange={(v) => setSetting('draw.shapeApply', v)}
        />
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title="판서" />
    <Group>
      <Row label="첫 휠 배율">
        <Segmented
          label="첫 휠 배율"
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
      <Row label="나갈 때 보관" sub="Alt+Shift+1로 다시 열기">
        <Switch
          checked={Boolean(settings['draw.keep'])}
          label="나갈 때 보관"
          onCheckedChange={(v) => setSetting('draw.keep', v)}
        />
      </Row>
      <Row label="Esc 키">
        <Segmented
          label="Esc 키 동작"
          value={settings['draw.esc'] ?? 'step'}
          options={[
            { value: 'step', label: '한 단계씩' },
            { value: 'exit', label: '바로 나가기' },
          ]}
          onChange={(v) => setSetting('draw.esc', v)}
        />
      </Row>
      <Row label="확대 중 오른쪽 클릭" sub="손 모드 · 아니면 끝내기">
        <Switch
          checked={Boolean(settings['draw.rightClickPan'])}
          label="확대 중 오른쪽 클릭"
          onCheckedChange={(v) => setSetting('draw.rightClickPan', v)}
        />
      </Row>
      <Row label="칠판 미리 보기" sub={`${boardShortcut} · W 흰색 · K 검은색`}>
        <div class="ink-board-previews" role="group" aria-label="흰 칠판과 검은 칠판 미리 보기">
          {#each boardPreviews as board (board.id)}
            <div
              class="ink-board-mini"
              data-board={board.id}
              style:background={board.bg}
              title={board.substituted ? `${board.label} (검정 펜은 흰색으로)` : board.label}
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
                <span>{board.id === 'white' ? '흰색' : '검은색'}</span>
              </span>
            </div>
          {/each}
        </div>
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title="실시간 확대" />
    <Group>
      <Row label="커서 따라가기">
        <Segmented
          label="커서 따라가기"
          value={settings['zoom.follow'] ?? 'smooth'}
          options={[
            { value: 'smooth', label: '부드럽게' },
            { value: 'instant', label: '바로' },
          ]}
          onChange={(v) => setSetting('zoom.follow', v)}
        />
      </Row>
      <Row label="위치 지도 보이기" sub="모서리에 작게">
        <Switch
          checked={Boolean(settings['zoom.minimap'])}
          label="위치 지도 보이기"
          onCheckedChange={(v) => setSetting('zoom.minimap', v)}
        />
      </Row>
    </Group>
  </section>
  </div>
</div>

<style>
  :global(.ink-row-wrap.ink-row-off) {
    opacity: 0.45;
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

  .ink-board-previews {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .ink-board-mini {
    position: relative;
    width: 92px;
    height: 44px;
    border-radius: var(--ink-r-row);
    box-sizing: border-box;
    overflow: hidden;
    flex: none;
  }

  .ink-board-mini[data-board='white'] {
    box-shadow: inset 0 0 0 1px var(--ink-line-strong);
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
