<script>
  import '../lib/tokens.css';
  import { createFakeClient } from '@screen-ink/client/fake';
  import {
    AppIcon,
    Banner,
    CaptureOverlay,
    CheatSheet,
    FirstRunSheet,
    Group,
    ICON_NAMES,
    ICONS,
    Icon,
    Island,
    IslandHost,
    Keycap,
    Magnifier,
    MenuRow,
    MiniMap,
    PinFrame,
    Row,
    SectionLabel,
    Segmented,
    SelectionFrame,
    SettingsWindow,
    Sheet,
    Switch,
    TrayPopoverScreen,
    applyTheme,
    watchSystemTheme,
  } from '../lib/index.js';

  let { client } = $props();

  const params = new URLSearchParams(typeof location !== 'undefined' ? location.search : '');
  const initialScreen = params.get('screen') || 'settings';
  const initialTheme = params.get('theme') || (initialScreen === 'first' || initialScreen === 'cheat' ? 'dark' : 'light');
  const initialPlatform = params.get('platform') || 'windows';
  const initialTab = params.get('tab') || 'shortcuts';
  const initialFirstRun = params.get('firstRun') === '1';
  const hideBar = params.get('chrome') === '0';

  let screen = $state(initialScreen);
  let theme = $state(initialTheme);
  let tab = $state(initialTab);
  let systemDark = $state(false);
  let boardSeg = $state('copy');
  let boardSwitchOn = $state(true);
  let boardSwitchOff = $state(false);
  let demoSheetOpen = $state(false);
  let demoDangerSheetOpen = $state(false);
  let settingsFirstRun = $state(initialFirstRun);
  let firstOpen = $state(true);
  let firstConflict = $state(true);
  let cheatOpen = $state(true);
  let cheatMode = $state('draw');
  let overlayActive = $state(false);
  let overlayNativeFrame = $state(params.get('native') === '1');
  /** @type {{ loadMs?: number, paintMs?: number } | null} */
  let overlayLastTiming = $state(null);

  const PREVIEW_HIGHLIGHT = { x: 240, y: 140, w: 560, h: 340 };

  // 가짜 클라이언트는 nativeFrame·highlight를 보내지 않으므로 미리보기 전용 래퍼에서 주입한다.
  const overlayPreviewClient = {
    getState: () => client.getState(),
    updateSetting: (path, value) => client.updateSetting(path, value),
    setShortcut: (id, combo) => client.setShortcut(id, combo),
    previewShortcut: (id, combo) => client.previewShortcut(id, combo),
    resetShortcutGroup: (group) => client.resetShortcutGroup(group),
    undo: () => client.undo(),
    setHotkeysEnabled: (enabled) => client.setHotkeysEnabled(enabled),
    run: (action) => client.run(action),
    previewImport: () => client.previewImport(),
    applyImport: (path) => client.applyImport(path),
    resetAll: () => client.resetAll(),
    overlayReady: (session, ui) => {
      overlayLastTiming = ui ?? null;
      return client.overlayReady(session, ui);
    },
    captureInput: (input) => client.captureInput(input),
    subscribe(handler) {
      return client.subscribe((e) => {
        if (e.type === 'overlay') {
          handler({
            type: 'overlay',
            overlay: {
              ...e.overlay,
              nativeFrame: overlayNativeFrame,
              highlight: e.overlay.highlight ?? PREVIEW_HIGHLIGHT,
            },
          });
          return;
        }
        handler(e);
      });
    },
  };

  // 트레이 미리보기에서 준비됨(겹침 없음) 상태를 보여 주기 위한 보조 클라이언트
  const cleanTrayClient = createFakeClient({ platform: initialPlatform, snipasteRunning: false });

  $effect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    return watchSystemTheme((isDark) => {
      systemDark = isDark;
    });
  });

  $effect(() => {
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        overlayActive = e.state?.mode?.mode === 'capture';
        if (e.state?.theme) {
          if (e.state.theme === 'light' || e.state.theme === 'dark') {
            theme = e.state.theme;
          } else if (e.state.theme === 'system') {
            theme = systemDark ? 'dark' : 'light';
          }
        }
      }
    });
    return off;
  });

  $effect(() => {
    if (typeof document !== 'undefined') {
      applyTheme(document.documentElement, theme, systemDark);
    }
  });

  function updateQuery(nextScreen, nextTheme, nextTab) {
    screen = nextScreen;
    theme = nextTheme;
    tab = nextTab;
    if (nextScreen === 'first') firstOpen = true;
    if (nextScreen === 'cheat') cheatOpen = true;
    if (typeof history !== 'undefined') {
      const q = new URLSearchParams(location.search);
      q.set('screen', screen);
      q.set('theme', theme);
      q.set('platform', initialPlatform);
      if (screen === 'settings') q.set('tab', tab);
      else q.delete('tab');
      history.replaceState(null, '', `?${q.toString()}`);
    }
  }

  function triggerIslandDemo(kind) {
    if (kind === 'result') {
      client.setShortcut('active', 'Shift+F2');
    } else if (kind === 'off') {
      client.setHotkeysEnabled(false);
    } else if (kind === 'on') {
      client.setHotkeysEnabled(true);
    } else {
      client.run('capture.region');
    }
  }
</script>

<div class="pv-shell" data-theme={theme}>
  {#if !hideBar && !overlayActive}
    <header class="pv-bar" data-cheat-ignore="true">
      <div class="pv-bar-group">
        {#each [
          { id: 'tray', label: '트레이 (S01)' },
          { id: 'settings', label: '설정 (S02~S05)' },
          { id: 'first', label: '첫 실행 (S06)' },
          { id: 'overlay', label: '오버레이 (S07)' },
          { id: 'cheat', label: '단축키 한 장 (S16)' },
          { id: 'island', label: '섬 (S17)' },
          { id: 'board', label: '부품 판 (F01·F02)' },
        ] as s (s.id)}
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={screen === s.id ? 'true' : undefined}
            onclick={() => updateQuery(s.id, theme, tab)}
          >
            {s.label}
          </button>
        {/each}
      </div>

      {#if screen === 'settings'}
        <div class="pv-bar-group">
          {#each [
            { id: 'shortcuts', label: '단축키' },
            { id: 'capture', label: '캡처·녹화' },
            { id: 'draw', label: '판서·확대' },
            { id: 'general', label: '일반' },
          ] as t (t.id)}
            <button
              type="button"
              class="pv-pill ink-focus"
              data-active={tab === t.id ? 'true' : undefined}
              onclick={() => updateQuery('settings', theme, t.id)}
            >
              {t.label}
            </button>
          {/each}
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={settingsFirstRun ? 'true' : undefined}
            onclick={() => (settingsFirstRun = !settingsFirstRun)}
          >
            첫 실행 시트
          </button>
        </div>
      {:else if screen === 'first'}
        <div class="pv-bar-group">
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={firstConflict ? 'true' : undefined}
            onclick={() => { firstConflict = true; firstOpen = true; }}
          >
            겹침 있음
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={!firstConflict ? 'true' : undefined}
            onclick={() => { firstConflict = false; firstOpen = true; }}
          >
            겹침 없음(F키 안내)
          </button>
        </div>
      {:else if screen === 'cheat'}
        <div class="pv-bar-group">
          {#each [
            { id: 'draw', label: '판서' },
            { id: 'capture', label: '캡처' },
            { id: 'zoom', label: '실시간 확대' },
          ] as m (m.id)}
            <button
              type="button"
              class="pv-pill ink-focus"
              data-active={cheatMode === m.id ? 'true' : undefined}
              onclick={() => { cheatMode = m.id; cheatOpen = true; }}
            >
              {m.label}
            </button>
          {/each}
        </div>
      {/if}

      <div class="pv-bar-spacer"></div>

      <div class="pv-bar-group">
        <button
          type="button"
          class="pv-pill ink-focus"
          data-active={theme === 'light' ? 'true' : undefined}
          onclick={() => updateQuery(screen, 'light', tab)}
        >
          라이트
        </button>
        <button
          type="button"
          class="pv-pill ink-focus"
          data-active={theme === 'dark' ? 'true' : undefined}
          onclick={() => updateQuery(screen, 'dark', tab)}
        >
          다크
        </button>
        <span class="pv-meta ink-mono">{initialPlatform}</span>
      </div>
    </header>
  {/if}

  <IslandHost {client} />

  {#if screen === 'tray'}
    <div class="pv-tray-split">
      <section class="pv-tray-pane" data-theme="light">
        <p class="pv-pane-label">라이트 — 그래파이트</p>
        <TrayPopoverScreen client={cleanTrayClient} />
      </section>
      <section class="pv-tray-pane" data-theme="dark">
        <p class="pv-pane-label">다크 — 아일랜드</p>
        <TrayPopoverScreen client={cleanTrayClient} />
      </section>
    </div>
  {:else if screen === 'settings'}
    <div class="pv-stage">
      <SettingsWindow
        {client}
        initialTab={tab}
        firstRun={settingsFirstRun}
        onFirstRunClose={() => (settingsFirstRun = false)}
      />
    </div>
  {:else if screen === 'first'}
    <div class="pv-stage">
      {#if !firstOpen}
        <button type="button" class="pv-pill ink-focus" onclick={() => (firstOpen = true)}>
          첫 실행 시트 다시 열기
        </button>
      {/if}
      <FirstRunSheet
        client={firstConflict ? client : cleanTrayClient}
        bind:open={firstOpen}
        onChangeShortcuts={() => updateQuery('settings', theme, 'shortcuts')}
      />
    </div>
  {:else if screen === 'overlay'}
    <div class="pv-stage">
      <div class="pv-overlay-launcher">
        <h2>캡처 오버레이</h2>
        <p class="pv-overlay-sub">움직임(인식) · 휠(넓히기·좁히기) · 클릭(확정) · 드래그 · Enter(복사) · ?(안내) · 우클릭·Esc(다시/취소)</p>
        <div class="pv-bar-group" role="group" aria-label="오버레이 렌더 방식">
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={!overlayNativeFrame ? 'true' : undefined}
            onclick={() => (overlayNativeFrame = false)}
          >
            웹 이미지 (nativeFrame=false)
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={overlayNativeFrame ? 'true' : undefined}
            onclick={() => (overlayNativeFrame = true)}
          >
            네이티브 정지 화면 (nativeFrame=true)
          </button>
        </div>
        <div class="pv-bar-group">
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active="true"
            onclick={() => client.run('capture.region')}
          >
            범위 캡처 (F1)
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            onclick={() => client.run('capture.window')}
          >
            창 캡처 (F2)
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            onclick={() => client.run('record.toggle')}
          >
            녹화 범위 (F3)
          </button>
        </div>
        {#if overlayLastTiming}
          <p class="pv-overlay-sub ink-mono">
            직전 overlayReady · load {overlayLastTiming.loadMs ?? 0}ms · paint {overlayLastTiming.paintMs ?? 0}ms
          </p>
        {/if}
      </div>
      <CaptureOverlay client={overlayPreviewClient} monitorId={0} />
    </div>
  {:else if screen === 'cheat'}
    <div class="pv-stage">
      {#if !cheatOpen}
        <button type="button" class="pv-pill ink-focus" onclick={() => (cheatOpen = true)}>
          단축키 한 장 다시 열기
        </button>
      {/if}
      <CheatSheet {client} mode={cheatMode} bind:open={cheatOpen} />
    </div>
  {:else if screen === 'island'}
    <div class="pv-island-page" data-theme="dark">
      <div class="pv-island-head">
        <h2>섬 — 화면 위 알림은 이 알약 하나</h2>
        <div class="pv-bar-group">
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('result')}>결과 띄우기(1.2초)</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('off')}>단축키 꺼짐</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('on')}>단축키 켜기</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('info')}>안내</button>
        </div>
      </div>

      <div class="pv-island-grid">
        <div class="pv-isl-item">
          <p>범위 캡처<small class="ink-mono">F1</small></p>
          <Island variant="region" text="범위 캡처" path={['창', '표', '2행']} level={2} value="718×40" />
        </div>
        <div class="pv-isl-item">
          <p>결과<small class="ink-mono">Enter · 1.2초</small></p>
          <Island
            variant="result"
            text="복사했어요"
            value="718×40"
            detail="한글·PPT에 Ctrl+V"
            valueMuted={true}
            button={{ label: '저장', key: 'Ctrl+S' }}
            secondaryButton={{ label: '판서로', key: 'Alt+1' }}
            progressDurationMs={1200}
            paused={true}
          >
            {#snippet thumbnail()}
              <span class="pv-thumb-sample"></span>
            {/snippet}
          </Island>
        </div>
        <div class="pv-isl-item">
          <p>창 캡처<small class="ink-mono">F2</small></p>
          <Island variant="window" text="창 캡처" detail="PowerPoint" keys={['F2']} keyHint="한 번 더 = 활성 창" />
        </div>
        <div class="pv-isl-item">
          <p>지연 캡처<small class="ink-mono">Alt F1</small></p>
          <Island variant="delay" countdown={3} text="초 뒤 범위 캡처" detail="메뉴를 열어 두세요" keys={['Esc']} />
        </div>
        <div class="pv-isl-item">
          <p>녹화 중<small class="ink-mono">F3 · 계속 보임</small></p>
          <Island
            variant="record"
            text="00:42"
            detail="1920×1080"
            micOff={true}
            button={{ label: '중지', key: 'F3', stop: true }}
          />
        </div>
        <div class="pv-isl-item">
          <p>판서<small class="ink-mono">Alt 1 · 계속 보임</small></p>
          <Island variant="draw" penColor="R" text="펜" penWidth="4px" value="×1.75" keys={['?']} />
        </div>
        <div class="pv-isl-item">
          <p>실시간 확대<small class="ink-mono">Alt 2</small></p>
          <Island variant="zoom" text="실시간 확대" value="×2.0" keys={['Alt 1']} keyHint="멈추고 쓰기" />
        </div>
        <div class="pv-isl-item">
          <p>칠판<small class="ink-mono">Alt 3</small></p>
          <Island variant="board" text="칠판" detail="흰색" value="1 / 3" keys={['W', 'K']} />
        </div>
        <div class="pv-isl-item">
          <p>창 고정<small class="ink-mono">Alt Shift D</small></p>
          <Island variant="pin" lead="Chrome" text="을 맨 위에 고정했어요" keys={['Alt', 'Shift', 'D']} keyHint="풀기" />
        </div>
        <div class="pv-isl-item">
          <p>겹침 문제<small class="ink-mono">시작 · 등록 실패</small></p>
          <Island variant="problem" text="F1은 Snipaste가 쓰고 있어요" button={{ label: '끄는 법' }} />
        </div>
        <div class="pv-isl-item">
          <p>단축키 꺼짐<small class="ink-mono">Alt `</small></p>
          <Island variant="off" text="단축키를 잠시 껐어요" keys={['Alt `']} keyHint="다시 켜기" />
        </div>
        <div class="pv-isl-item">
          <p>보안 화면<small class="ink-mono">UAC · 잠금</small></p>
          <Island variant="security" text="보안 화면에서는 쓸 수 없어요" />
        </div>
      </div>

      <div class="pv-island-twoline">
        <p class="pv-pane-label">처음 3번 또는 ? 키 — 두 줄 안내</p>
        <Island
          variant="region"
          text="범위 캡처"
          path={['창', '표', '2행']}
          level={2}
          value="718×40"
          showHints={true}
          hints={[
            { key: '휠', label: '넓히기·좁히기' },
            { key: 'Tab', label: '창' },
            { key: 'Enter', label: '복사' },
            { key: 'Esc', label: '취소' },
          ]}
        />
      </div>
    </div>
  {:else}
    <div class="pv-board-split">
      {#each ['light', 'dark'] as mode (mode)}
        <section class="pv-board-half" data-theme={mode}>
          <div class="pv-board-title">
            <h2>{mode === 'light' ? '라이트 · 그래파이트' : '다크 · 아일랜드'}</h2>
            <span class="ink-mono">{mode === 'light' ? 'E' : 'B'}</span>
          </div>

          <div>
            <SectionLabel title="행 상태 5종 — 기본 · 올림 · 누름 · 선택 · 비활성" />
            <Group>
              <div class="pv-row-states">
                <MenuRow icon="region" label="기본" combo="F1" visualState="default" />
                <MenuRow icon="region" label="올림" combo="F1" visualState="hover" />
                <MenuRow icon="region" label="누름" combo="F1" visualState="press" />
                <MenuRow icon="region" label="선택 · 키보드 포커스" combo="F1" visualState="selected" />
                <MenuRow icon="region" label="비활성 · 단축키 꺼짐" combo="F1" visualState="disabled" />
              </div>
            </Group>
          </div>

          <div>
            <SectionLabel title="부품 — 스위치 · 나눔 단추 · 키캡 · 배너 · 시트" />
            <Group>
              <Row label="스위치">
                <div class="pv-inline">
                  <Switch bind:checked={boardSwitchOn} label="켬 예시" />
                  <Switch bind:checked={boardSwitchOff} label="끔 예시" />
                </div>
              </Row>
              <Row label="나눔 단추">
                <Segmented
                  bind:value={boardSeg}
                  label="결과 예시"
                  options={[
                    { value: 'copy', label: '복사' },
                    { value: 'save', label: '저장' },
                    { value: 'both', label: '둘 다' },
                  ]}
                />
              </Row>
              <Row label="키캡 상태">
                <div class="pv-inline">
                  <Keycap combo="Alt+Shift+D" variant="default" />
                  <Keycap label="F1" variant="waiting" />
                  <Keycap label="F2" variant="dim" />
                </div>
              </Row>
              <Row label="확인 시트">
                <div class="pv-inline">
                  <button type="button" class="pv-pill ink-focus" onclick={() => (demoSheetOpen = true)}>일반 시트</button>
                  <button type="button" class="pv-pill ink-focus" onclick={() => (demoDangerSheetOpen = true)}>위험 시트(3초)</button>
                </div>
              </Row>
            </Group>
          </div>

          <Banner
            kind="conflict"
            lead="Snipaste"
            message="가 F1·F3 키 4개를 먼저 쓰고 있어요"
            primaryLabel="끄는 법"
            onPrimary={() => client.run('snipaste.howto')}
            secondaryLabel="키 바꾸기"
            onSecondary={() => updateQuery('settings', mode, 'shortcuts')}
          />

          <div>
            <SectionLabel title="오버레이 부품 — 선택 틀 · 확대경 · 위치 지도 · 창 고정" />
            <div class="pv-overlay-row">
              <div class="pv-sel-demo">
                <SelectionFrame x={22} y={24} width={120} height={54} state="confirmed" scrim={false} />
              </div>
              <Magnifier rgb={[255, 255, 255]} coord={{ x: 549, y: 246 }} />
              <MiniMap size="md" label="모니터 1" />
              <PinFrame borderWidth={4} radius={8}>
                <div class="pv-pin-sample">고정 창</div>
              </PinFrame>
            </div>
          </div>

          <div>
            <SectionLabel title="선 아이콘 24종 + 앱 아이콘" />
            <Group>
              <div class="pv-icon-grid">
                {#each ICON_NAMES as name (name)}
                  <div class="pv-icon-cell">
                    <Icon {name} size={24} />
                    <span>{ICONS[name].label}</span>
                  </div>
                {/each}
              </div>
            </Group>
            <div class="pv-app-icons">
              <AppIcon size={64} />
              <AppIcon size={32} />
              <AppIcon size={24} />
              <AppIcon size={16} />
            </div>
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>

<Sheet
  bind:open={demoSheetOpen}
  title="설정 가져오기"
  description="가져온 파일에서 4개 항목이 바뀌어요."
  confirmLabel="가져오기"
/>

<Sheet
  bind:open={demoDangerSheetOpen}
  title="설정을 모두 처음으로 되돌릴까요?"
  description="바꾼 단축키와 설정이 기본값으로 돌아가요."
  confirmLabel="되돌리기"
  danger={true}
  dangerDelaySeconds={3}
/>

<style>
  .pv-shell {
    min-height: 100vh;
    background: var(--ink-preview-backdrop);
    color: var(--ink-text);
    display: flex;
    flex-direction: column;
  }

  .pv-bar {
    position: relative;
    z-index: 80;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: var(--ink-surface);
    border-bottom: 1px solid var(--ink-line);
    flex-wrap: wrap;
  }

  .pv-bar-group {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .pv-bar-spacer {
    flex: 1;
  }

  .pv-pill {
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--ink-r-btn);
    background: var(--ink-raised);
    color: var(--ink-text-2);
    font-family: inherit;
    font-size: 12px;
    white-space: nowrap;
    cursor: pointer;
  }

  .pv-pill[data-active='true'] {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    font-weight: 500;
  }

  .pv-meta {
    font-size: 11px;
    color: var(--ink-sub);
    padding-left: 6px;
  }

  .pv-stage {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .pv-overlay-launcher {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 24px 28px;
    border-radius: var(--ink-r-window);
    background: var(--ink-surface);
    box-shadow: var(--ink-group-ring);
  }

  .pv-overlay-launcher h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 600;
  }

  .pv-overlay-sub {
    margin: 0;
    font-size: 12px;
    color: var(--ink-sub);
  }

  .pv-tray-split {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .pv-tray-pane {
    position: relative;
    display: grid;
    place-items: center;
    padding: 48px 24px;
    background: var(--ink-preview-backdrop);
    color: var(--ink-text);
  }

  .pv-pane-label {
    position: absolute;
    left: 24px;
    top: 18px;
    margin: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--ink-text-2);
  }

  .pv-island-page {
    flex: 1;
    padding: 28px 36px;
    background: var(--ink-bg);
    color: var(--ink-text);
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .pv-island-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .pv-island-head h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 600;
  }

  .pv-island-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 18px 40px;
  }

  .pv-isl-item {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: 14px;
  }

  .pv-isl-item p {
    margin: 0;
    font-size: 12.5px;
    color: var(--ink-text);
    white-space: nowrap;
  }

  .pv-isl-item small {
    display: block;
    font-size: 10.5px;
    color: var(--ink-sub);
    margin-top: 2px;
  }

  .pv-thumb-sample {
    display: block;
    width: 100%;
    height: 100%;
    background: var(--ink-island-text);
  }

  .pv-island-twoline {
    position: relative;
    padding-top: 28px;
  }

  .pv-island-twoline .pv-pane-label {
    left: 0;
    top: 0;
  }

  .pv-board-split {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .pv-board-half {
    padding: 24px 28px;
    background: var(--ink-bg);
    color: var(--ink-text);
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .pv-board-title {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .pv-board-title h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 600;
  }

  .pv-board-title span {
    font-size: 11px;
    color: var(--ink-sub);
  }

  .pv-row-states {
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pv-inline {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .pv-overlay-row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }

  .pv-sel-demo {
    position: relative;
    width: 164px;
    height: 102px;
    border-radius: var(--ink-r-group);
    background: var(--ink-surface);
    box-shadow: var(--ink-group-ring);
  }

  .pv-pin-sample {
    width: 88px;
    height: 64px;
    background: var(--ink-surface);
    color: var(--ink-text-2);
    font-size: 11.5px;
    display: grid;
    place-items: center;
  }

  .pv-icon-grid {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
  }

  .pv-icon-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 12px 4px 10px;
    font-size: 10.5px;
    color: var(--ink-text-2);
    white-space: nowrap;
    border-right: 1px solid var(--ink-hair);
    border-bottom: 1px solid var(--ink-hair);
  }

  .pv-icon-cell:nth-child(6n) {
    border-right: 0;
  }

  .pv-icon-cell:nth-last-child(-n + 6) {
    border-bottom: 0;
  }

  .pv-app-icons {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    margin-top: 10px;
  }
</style>
