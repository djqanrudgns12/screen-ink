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
    setLang,
    setOs,
    syncLang,
    t,
  } from '../lib/index.js';
  import { keycaps } from '@screen-ink/client';
  import { formatComboReadable } from '../lib/models/shortcutDisplay.js';

  let { client } = $props();

  const params = new URLSearchParams(typeof location !== 'undefined' ? location.search : '');
  const initialScreen = params.get('screen') || 'settings';
  const initialTheme = params.get('theme') || (initialScreen === 'first' || initialScreen === 'cheat' ? 'dark' : 'light');
  const initialPlatform = /** @type {'windows' | 'macos'} */ (params.get('platform') === 'macos' ? 'macos' : 'windows');
  const initialLang = params.get('lang') === 'en' ? 'en' : 'ko';
  const initialTab = params.get('tab') || 'shortcuts';
  const initialFirstRun = params.get('firstRun') === '1';
  const initialGuide = params.get('guide') || null;
  const stepRaw = params.get('step');
  const initialStep = stepRaw !== null && stepRaw !== '' ? Number(stepRaw) : undefined;
  const hideBar = params.get('chrome') === '0';

  setLang(initialLang);
  setOs(initialPlatform);

  let screen = $state(initialScreen);
  let theme = $state(initialTheme);
  let tab = $state(initialTab);
  let lang = $state(initialLang);
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
    systemFonts: () => client.systemFonts(),
    overlayReady: (session, ui) => {
      overlayLastTiming = ui ?? null;
      return client.overlayReady(session, ui);
    },
    captureInput: (input) => client.captureInput(input),
    // 캡처 주석(4.9): 개수 알림·주석 층 답(가짜 클라이언트가 부탁 → 답 → 결과를 흉내)
    captureAnnotations: (session, monitorId, items) => client.captureAnnotations(session, monitorId, items),
    captureCommitAnnotated: (meta, rgba) => client.captureCommitAnnotated(meta, rgba),
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
  const cleanTrayClient = createFakeClient({ platform: initialPlatform, lang: initialLang, snipasteRunning: false });

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
        syncLang(e.state);
        overlayActive = e.state?.mode?.mode === 'capture';
        if (e.state?.settings?.['general.lang']) {
          lang = e.state.settings['general.lang'];
        }
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

  function switchLanguage(nextLang) {
    setLang(nextLang);
    client.updateSetting('general.lang', nextLang);
    cleanTrayClient.updateSetting('general.lang', nextLang);
  }

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
    } else if (kind === 'chip') {
      void client.run('capture.region').then(() => client.captureInput({ type: 'command', name: 'copy' }));
    } else if (kind === 'draw') {
      void client.run('draw');
    } else if (kind === 'board') {
      void client.run('board');
    } else if (kind === 'off') {
      client.setHotkeysEnabled(false);
    } else if (kind === 'on') {
      client.setHotkeysEnabled(true);
    } else {
      client.run('capture.region');
    }
  }

  let kSave = $derived(initialPlatform === 'macos' ? 'Cmd+S' : 'Ctrl+S');
  let kDraw = $derived(initialPlatform === 'macos' ? 'Option+1' : 'Alt+1');
  let kDelay = $derived(initialPlatform === 'macos' ? 'Option+F1' : 'Alt+F1');
  let kZoom = $derived(initialPlatform === 'macos' ? 'Option+2' : 'Alt+2');
  let kBoard = $derived(initialPlatform === 'macos' ? 'Option+3' : 'Alt+3');
  let kPin = $derived(initialPlatform === 'macos' ? 'Option+Shift+D' : 'Alt+Shift+D');
  let kPause = $derived(initialPlatform === 'macos' ? 'Option+`' : 'Alt+`');
</script>

<div class="pv-shell" data-theme={theme}>
  {#if !hideBar && !overlayActive}
    <header class="pv-bar" data-cheat-ignore="true">
      <div class="pv-bar-group">
        {#each [
          { id: 'tray', label: t('meta.preview.tray') },
          { id: 'settings', label: t('meta.preview.settings') },
          { id: 'first', label: t('meta.preview.first') },
          { id: 'overlay', label: t('meta.preview.overlay') },
          { id: 'cheat', label: t('meta.preview.cheat') },
          { id: 'island', label: t('meta.preview.island') },
          { id: 'board', label: t('meta.preview.board') },
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
            { id: 'shortcuts', label: t('settings.tab.shortcuts') },
            { id: 'capture', label: t('settings.tab.capture') },
            { id: 'draw', label: t('settings.tab.draw') },
            { id: 'general', label: t('settings.tab.general') },
            { id: 'help', label: t('settings.tab.help') },
            { id: 'info', label: t('settings.tab.info') },
          ] as item (item.id)}
            <button
              type="button"
              class="pv-pill ink-focus"
              data-active={tab === item.id ? 'true' : undefined}
              onclick={() => updateQuery('settings', theme, item.id)}
            >
              {item.label}
            </button>
          {/each}
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={settingsFirstRun ? 'true' : undefined}
            onclick={() => (settingsFirstRun = !settingsFirstRun)}
          >
            {t('meta.preview.firstSheet')}
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
            {t('meta.preview.firstConflict')}
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={!firstConflict ? 'true' : undefined}
            onclick={() => { firstConflict = false; firstOpen = true; }}
          >
            {t('meta.preview.firstClean')}
          </button>
          {#if initialPlatform === 'macos'}
            <button
              type="button"
              class="pv-pill ink-focus"
              onclick={() => {
                /** @type {any} */ (client).setPermissions?.({ screen: false, accessibility: false, screenRevoked: false, accessibilityRevoked: false });
                firstOpen = true;
              }}
            >
              {t('meta.preview.firstPermOff')}
            </button>
            <button
              type="button"
              class="pv-pill ink-focus"
              onclick={() => {
                /** @type {any} */ (client).setPermissions?.({ screen: true, accessibility: true, screenRevoked: false, accessibilityRevoked: false });
                firstOpen = true;
              }}
            >
              {t('meta.preview.firstPermOn')}
            </button>
          {/if}
        </div>
      {:else if screen === 'cheat'}
        <div class="pv-bar-group">
          {#each [
            { id: 'draw', label: t('meta.preview.cheatModeDraw') },
            { id: 'capture', label: t('meta.preview.cheatModeCapture') },
            { id: 'zoom', label: t('meta.preview.cheatModeZoom') },
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
          data-active={lang === 'ko' ? 'true' : undefined}
          onclick={() => switchLanguage('ko')}
        >
          {t('meta.preview.langKo')}
        </button>
        <button
          type="button"
          class="pv-pill ink-focus"
          data-active={lang === 'en' ? 'true' : undefined}
          onclick={() => switchLanguage('en')}
        >
          EN
        </button>
        <button
          type="button"
          class="pv-pill ink-focus"
          data-active={theme === 'light' ? 'true' : undefined}
          onclick={() => updateQuery(screen, 'light', tab)}
        >
          {t('meta.preview.themeLight')}
        </button>
        <button
          type="button"
          class="pv-pill ink-focus"
          data-active={theme === 'dark' ? 'true' : undefined}
          onclick={() => updateQuery(screen, 'dark', tab)}
        >
          {t('meta.preview.themeDark')}
        </button>
        <span class="pv-meta ink-mono">{initialPlatform}</span>
      </div>
    </header>
  {/if}

  <IslandHost {client} />

  {#if screen === 'tray'}
    <div class="pv-tray-split">
      <section class="pv-tray-pane" data-theme="light">
        <p class="pv-pane-label">{t('meta.preview.themeLightLabel')}</p>
        <TrayPopoverScreen client={cleanTrayClient} />
      </section>
      <section class="pv-tray-pane" data-theme="dark">
        <p class="pv-pane-label">{t('meta.preview.themeDarkLabel')}</p>
        <TrayPopoverScreen client={cleanTrayClient} />
      </section>
    </div>
  {:else if screen === 'settings'}
    <div class="pv-stage">
      <SettingsWindow
        {client}
        initialTab={/** @type {any} */ (tab)}
        firstRun={settingsFirstRun}
        initialHelpGuide={initialGuide}
        onFirstRunClose={() => (settingsFirstRun = false)}
      />
    </div>
  {:else if screen === 'first'}
    <div class="pv-stage">
      {#if !firstOpen}
        <button type="button" class="pv-pill ink-focus" onclick={() => (firstOpen = true)}>
          {t('meta.preview.firstReopen')}
        </button>
      {/if}
      <FirstRunSheet
        client={firstConflict ? client : cleanTrayClient}
        bind:open={firstOpen}
        {initialStep}
        onChangeShortcuts={() => updateQuery('settings', theme, 'shortcuts')}
      />
    </div>
  {:else if screen === 'overlay'}
    <div class="pv-stage">
      <div class="pv-overlay-launcher">
        <h2>{t('meta.preview.overlayHeading')}</h2>
        <p class="pv-overlay-sub">{t('meta.preview.overlaySub')}</p>
        <div class="pv-bar-group" role="group" aria-label={t('meta.preview.overlayRenderAria')}>
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={!overlayNativeFrame ? 'true' : undefined}
            onclick={() => (overlayNativeFrame = false)}
          >
            {t('meta.preview.overlayWeb')}
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active={overlayNativeFrame ? 'true' : undefined}
            onclick={() => (overlayNativeFrame = true)}
          >
            {t('meta.preview.overlayNative')}
          </button>
        </div>
        <div class="pv-bar-group">
          <button
            type="button"
            class="pv-pill ink-focus"
            data-active="true"
            onclick={() => client.run('capture.region')}
          >
            {t('shortcuts.slot.region')} (F1)
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            onclick={() => client.run('capture.window')}
          >
            {t('shortcuts.slot.window')} (F2)
          </button>
          <button
            type="button"
            class="pv-pill ink-focus"
            onclick={() => client.run('record.toggle')}
          >
            {t('shortcuts.slot.record')} (F3)
          </button>
        </div>
        {#if overlayLastTiming}
          <p class="pv-overlay-sub ink-mono">
            {t('meta.preview.overlayTiming', { load: overlayLastTiming.loadMs ?? 0, paint: overlayLastTiming.paintMs ?? 0 })}
          </p>
        {/if}
      </div>
      <CaptureOverlay client={overlayPreviewClient} monitorId={0} />
    </div>
  {:else if screen === 'cheat'}
    <div class="pv-stage">
      {#if !cheatOpen}
        <button type="button" class="pv-pill ink-focus" onclick={() => (cheatOpen = true)}>
          {t('meta.preview.cheatReopen')}
        </button>
      {/if}
      <CheatSheet {client} mode={cheatMode} bind:open={cheatOpen} />
    </div>
  {:else if screen === 'island'}
    <div class="pv-island-page" data-theme="dark">
      <div class="pv-island-head">
        <h2>{t('meta.preview.islandHeading')}</h2>
        <div class="pv-bar-group">
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('result')}>{t('meta.preview.islandBtnResult')}</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('chip')}>{t('meta.preview.islandBtnChip')}</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('draw')}>{t('meta.preview.islandBtnDraw')}</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('board')}>{t('meta.preview.islandBtnBoard')}</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('off')}>{t('meta.preview.islandBtnOff')}</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('on')}>{t('meta.preview.islandBtnOn')}</button>
          <button type="button" class="pv-pill ink-focus" onclick={() => triggerIslandDemo('info')}>{t('meta.preview.islandBtnInfo')}</button>
        </div>
      </div>

      <div class="pv-island-grid">
        <div class="pv-isl-item">
          <p>{t('island.capture.regionTitle')}<small class="ink-mono">{t('meta.preview.islandRegionSub')}</small></p>
          <Island
            variant="region"
            platform={initialPlatform}
            text={t('island.capture.regionTitle')}
            path={[t('meta.preview.islandPath1'), t('meta.preview.islandPath2'), t('meta.preview.islandPath3')]}
            level={2}
            value="718×40"
          />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandResultTitle')}<small class="ink-mono">Enter · 1.2s</small></p>
          <Island
            variant="result"
            platform={initialPlatform}
            text={t('island.capture.copied')}
            value="718×40"
            detail={t('island.capture.pasteHint')}
            valueMuted={true}
            button={{ label: t('common.save'), key: kSave }}
            secondaryButton={{ label: t('island.capture.toDraw'), key: kDraw }}
            progressDurationMs={1200}
            paused={true}
          >
            {#snippet thumbnail()}
              <span class="pv-thumb-sample"></span>
            {/snippet}
          </Island>
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandResultChipTitle')}<small class="ink-mono">HEX</small></p>
          <Island
            variant="result"
            platform={initialPlatform}
            text={t('island.capture.copied')}
            value="120×120"
            detail={t('island.capture.pasteHint')}
            valueMuted={true}
            button={{ label: t('common.save'), key: kSave, onClick: () => client.run('capture.saveLast') }}
            secondaryButton={{ label: t('island.capture.toDraw'), key: kDraw }}
            chip={{ label: t('island.capture.copyHexChip', { hex: '#FF9F0A' }), hex: '#FF9F0A', onClick: () => client.run('capture.copyHex') }}
            progressDurationMs={1200}
            paused={true}
          >
            {#snippet thumbnail()}
              <span class="pv-thumb-sample" style:background="#FF9F0A"></span>
            {/snippet}
          </Island>
        </div>
        <div class="pv-isl-item">
          <p>{t('island.capture.windowTitle')}<small class="ink-mono">F2</small></p>
          <Island variant="window" platform={initialPlatform} text={t('island.capture.windowTitle')} detail="PowerPoint" keys={['F2']} keyHint={t('meta.preview.islandWindowKeyHint')} />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandDelayTitle')}<small class="ink-mono">{formatComboReadable(kDelay, initialPlatform)}</small></p>
          <Island variant="delay" platform={initialPlatform} countdown={3} text={t('island.delay.title')} detail={t('meta.preview.islandDelayDetail')} keys={['Esc']} />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandRecordTitle')}<small class="ink-mono">{t('meta.preview.islandRecordSub')}</small></p>
          <Island
            variant="record"
            platform={initialPlatform}
            text="00:42"
            detail="1920×1080"
            micOff={true}
            button={{ label: t('island.record.stopBtn'), key: 'F3', stop: true }}
          />
        </div>
        <div class="pv-isl-item">
          <p>{t('shortcuts.slot.draw')}<small class="ink-mono">{formatComboReadable(kDraw, initialPlatform)}</small></p>
          <Island variant="draw" platform={initialPlatform} penColor="R" text={t('island.draw.title', { tool: t('island.draw.tool.pen') })} penWidth="4px" value="×1.75" keys={['?']} />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandDraw6Title')}<small class="ink-mono">{t('meta.preview.islandDraw6Sub')}</small></p>
          <Island variant="draw" platform={initialPlatform} penColor="O" paletteExpanded={true} text={t('island.draw.title', { tool: t('island.draw.tool.pen') })} penWidth="4px" />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandDrawDimTitle')}<small class="ink-mono">{t('meta.preview.islandDrawDimSub')}</small></p>
          <Island variant="draw" platform={initialPlatform} penColor="R" idleDim={true} text={t('island.draw.title', { tool: t('island.draw.tool.pen') })} penWidth="4px" value="×1.75" />
        </div>
        <div class="pv-isl-item">
          <p>{t('island.zoom.title')}<small class="ink-mono">{formatComboReadable(kZoom, initialPlatform)}</small></p>
          <Island variant="zoom" platform={initialPlatform} text={t('island.zoom.title')} value="×2.0" keys={keycaps(kDraw, initialPlatform)} keyHint={t('meta.preview.islandZoomKeyHint')} />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandBoardTitle')}<small class="ink-mono">{formatComboReadable(kBoard, initialPlatform)}</small></p>
          <Island variant="board" platform={initialPlatform} penColor="R" text={t('island.draw.boardTitle', { bg: t('island.draw.board.white') })} value="1 / 1" keys={['W', 'K']} />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandBoard8Title')}<small class="ink-mono">{t('meta.preview.islandBoard8Sub')}</small></p>
          <Island variant="board" platform={initialPlatform} penColor="W" paletteExpanded={true} text={t('island.draw.boardTitle', { bg: t('island.draw.board.black') })} value="2 / 3" />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandPinTitle')}<small class="ink-mono">{formatComboReadable(kPin, initialPlatform)}</small></p>
          <Island variant="pin" platform={initialPlatform} lead={t('meta.preview.islandPinLead')} text={t('meta.preview.islandPinText')} keys={keycaps(kPin, initialPlatform)} keyHint={t('meta.preview.islandPinKeyHint')} />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandConflictTitle')}<small class="ink-mono">{t('meta.preview.islandConflictSub')}</small></p>
          <Island variant="problem" platform={initialPlatform} text={t('meta.preview.islandConflictText')} button={{ label: t('shortcuts.conflict.howto') }} />
        </div>
        <div class="pv-isl-item">
          <p>{t('tray.status.paused')}<small class="ink-mono">{formatComboReadable(kPause, initialPlatform)}</small></p>
          <Island variant="off" platform={initialPlatform} text={t('island.hotkeys.off')} keys={keycaps(kPause, initialPlatform)} keyHint={t('meta.preview.islandOffKeyHint')} />
        </div>
        <div class="pv-isl-item">
          <p>{t('meta.preview.islandSecurityTitle')}<small class="ink-mono">{t('meta.preview.islandSecuritySub')}</small></p>
          <Island variant="security" platform={initialPlatform} text={t('island.security')} />
        </div>
      </div>

      <div class="pv-island-twoline">
        <p class="pv-pane-label">{t('meta.preview.islandTwoLineLabel')}</p>
        <Island
          variant="region"
          platform={initialPlatform}
          text={t('island.capture.regionTitle')}
          path={[t('meta.preview.islandPath1'), t('meta.preview.islandPath2'), t('meta.preview.islandPath3')]}
          level={2}
          value="718×40"
          showHints={true}
          hints={[
            { key: t('island.hint.key.wheel'), label: t('island.hint.resize') },
            { key: 'Tab', label: t('island.hint.window') },
            { key: 'Enter', label: t('island.hint.copy') },
            { key: 'Esc', label: t('island.hint.cancel') },
          ]}
        />
      </div>
    </div>
  {:else}
    <div class="pv-board-split">
      {#each ['light', 'dark'] as mode (mode)}
        <section class="pv-board-half" data-theme={mode}>
          <div class="pv-board-title">
            <h2>{mode === 'light' ? t('meta.preview.boardLightTitle') : t('meta.preview.boardDarkTitle')}</h2>
            <span class="ink-mono">{mode === 'light' ? 'E' : 'B'}</span>
          </div>

          <div>
            <SectionLabel title={t('meta.preview.boardRowStates')} />
            <Group>
              <div class="pv-row-states">
                <MenuRow icon="region" label={t('meta.preview.boardRowDefault')} combo="F1" platform={initialPlatform} visualState="default" />
                <MenuRow icon="region" label={t('meta.preview.boardRowHover')} combo="F1" platform={initialPlatform} visualState="hover" />
                <MenuRow icon="region" label={t('meta.preview.boardRowPress')} combo="F1" platform={initialPlatform} visualState="press" />
                <MenuRow icon="region" label={t('meta.preview.boardRowSelected')} combo="F1" platform={initialPlatform} visualState="selected" />
                <MenuRow icon="region" label={t('meta.preview.boardRowDisabled')} combo="F1" platform={initialPlatform} visualState="disabled" />
              </div>
            </Group>
          </div>

          <div>
            <SectionLabel title={t('meta.preview.boardPartsSection')} />
            <Group>
              <Row label={t('meta.preview.boardSwitch')}>
                <div class="pv-inline">
                  <Switch bind:checked={boardSwitchOn} label={t('meta.preview.boardSwitchOn')} />
                  <Switch bind:checked={boardSwitchOff} label={t('meta.preview.boardSwitchOff')} />
                </div>
              </Row>
              <Row label={t('meta.preview.boardSegmented')}>
                <Segmented
                  bind:value={boardSeg}
                  label={t('meta.preview.boardSegmentedAria')}
                  options={[
                    { value: 'copy', label: t('common.copy') },
                    { value: 'save', label: t('common.save') },
                    { value: 'both', label: t('page.capture.result.both') },
                  ]}
                />
              </Row>
              <Row label={t('meta.preview.boardKeycapState')}>
                <div class="pv-inline">
                  <Keycap combo={kPin} platform={initialPlatform} variant="default" />
                  <Keycap label="F1" variant="waiting" />
                  <Keycap label="F2" variant="dim" />
                </div>
              </Row>
              <Row label={t('meta.preview.boardSheet')}>
                <div class="pv-inline">
                  <button type="button" class="pv-pill ink-focus" onclick={() => (demoSheetOpen = true)}>{t('meta.preview.boardSheetNormal')}</button>
                  <button type="button" class="pv-pill ink-focus" onclick={() => (demoDangerSheetOpen = true)}>{t('meta.preview.boardSheetDanger')}</button>
                </div>
              </Row>
            </Group>
          </div>

          <Banner
            kind="conflict"
            lead="Snipaste"
            message={t('meta.preview.boardBannerMsg')}
            primaryLabel={t('shortcuts.conflict.howto')}
            onPrimary={() => client.run('snipaste.howto')}
            secondaryLabel={t('shortcuts.conflict.change')}
            onSecondary={() => updateQuery('settings', mode, 'shortcuts')}
          />

          <div>
            <SectionLabel title={t('meta.preview.boardOverlaySection')} />
            <div class="pv-overlay-row">
              <div class="pv-sel-demo">
                <SelectionFrame x={22} y={24} width={120} height={54} state="confirmed" scrim={false} />
              </div>
              <Magnifier rgb={[255, 255, 255]} coord={{ x: 549, y: 246 }} />
              <MiniMap size="md" />
              <PinFrame borderWidth={4} radius={8}>
                <div class="pv-pin-sample">{t('meta.preview.boardPinSample')}</div>
              </PinFrame>
            </div>
          </div>

          <div>
            <SectionLabel title={t('meta.preview.boardIconsSection')} />
            <Group>
              <div class="pv-icon-grid">
                {#each ICON_NAMES as name (name)}
                  <div class="pv-icon-cell">
                    <Icon {name} size={24} />
                    <span>{t(ICONS[name].labelKey)}</span>
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
  title={t('meta.preview.boardSheetDemoTitle')}
  description={t('meta.preview.boardSheetDemoDesc')}
  confirmLabel={t('general.data.import')}
/>

<Sheet
  bind:open={demoDangerSheetOpen}
  title={t('meta.preview.boardDangerTitle')}
  description={t('meta.preview.boardDangerDesc')}
  confirmLabel={t('general.resetSheet.confirm')}
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
    background: #141518;
    color: #f7f8fa;
    padding: 26px 30px 44px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .pv-island-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
  }

  .pv-island-head h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 600;
  }

  .pv-island-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(540px, 100%), 1fr));
    gap: 18px 24px;
  }

  .pv-isl-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
    max-width: 100%;
  }

  .pv-isl-item p {
    margin: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 12px;
    color: #c9ccd3;
  }

  .pv-isl-item small {
    font-size: 11px;
    color: #8e939e;
  }

  .pv-thumb-sample {
    width: 100%;
    height: 100%;
    display: block;
    background: linear-gradient(135deg, #2563eb 0%, #38bdf8 55%, #f59e0b 100%);
  }

  .pv-island-twoline {
    position: relative;
    padding: 42px 24px 24px;
    border-radius: var(--ink-r-window);
    background: #1a1b1f;
    display: flex;
    justify-content: center;
  }

  .pv-island-twoline .pv-pane-label {
    color: #8e939e;
  }

  .pv-board-split {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    min-height: calc(100vh - 45px);
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
