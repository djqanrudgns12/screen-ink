<script>
  import { Tabs } from 'bits-ui';
  import { SHORTCUT_GROUPS } from '@screen-ink/client';
  import Banner from '../components/Banner.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Keycap from '../components/Keycap.svelte';
  import Icon from '../icons/Icon.svelte';
  import { eventToCombo, isClearOnly, isEscapeOnly } from '../models/keyCombo.js';
  import { formatConflictBanner, getShortcutRowDisplay } from '../models/shortcutDisplay.js';
  import { locale, syncLang, t } from '../i18n/index.svelte.js';
  import { exportShortcutSheetPdf } from '../pdf/download.js';

  const GROUP_I18N_KEYS = {
    capture: 'shortcuts.group.capture',
    tools: 'shortcuts.group.tools',
    quick: 'shortcuts.group.quick',
    'capture-mode': 'shortcuts.group.captureMode',
    'draw-mode': 'shortcuts.group.drawMode',
    'zoom-mode': 'shortcuts.group.zoomMode',
  };

  const SLOT_I18N_KEYS = {
    region: 'shortcuts.slot.region',
    window: 'shortcuts.slot.window',
    record: 'shortcuts.slot.record',
    draw: 'shortcuts.slot.draw',
    zoom: 'shortcuts.slot.zoom',
    board: 'shortcuts.slot.board',
    last: 'shortcuts.slot.last',
    full: 'shortcuts.slot.full',
    active: 'shortcuts.slot.active',
    delay: 'shortcuts.slot.delay',
    restore: 'shortcuts.slot.restore',
    pause: 'shortcuts.slot.pause',
    topmost: 'shortcuts.slot.topmost',
    pin: 'shortcuts.slot.pin',
    pinToggle: 'shortcuts.slot.pinToggle',
    timer: 'shortcuts.slot.timer',
    'c-detect': 'shortcuts.slot.cDetect',
    'c-copy': 'shortcuts.slot.cCopy',
    'c-save': 'shortcuts.slot.cSave',
    'c-quick': 'shortcuts.slot.cQuick',
    'c-cancel': 'shortcuts.slot.cCancel',
    'c-last': 'shortcuts.slot.cLast',
    'c-prev': 'shortcuts.slot.cPrev',
    'c-next': 'shortcuts.slot.cNext',
    'c-full': 'shortcuts.slot.cFull',
    'c-cursor': 'shortcuts.slot.cCursor',
    'c-color': 'shortcuts.slot.cColor',
    'd-pen': 'shortcuts.slot.dPen',
    'd-marker': 'shortcuts.slot.dMarker',
    'd-text': 'shortcuts.slot.dText',
    'd-select': 'shortcuts.slot.dSelect',
    'd-eraser': 'shortcuts.slot.dEraser',
    'd-clear': 'shortcuts.slot.dClear',
    'd-red': 'shortcuts.slot.dRed',
    'd-yellow': 'shortcuts.slot.dYellow',
    'd-green': 'shortcuts.slot.dGreen',
    'd-orange': 'shortcuts.slot.dOrange',
    'd-blue': 'shortcuts.slot.dBlue',
    'd-lime': 'shortcuts.slot.dLime',
    'd-white': 'shortcuts.slot.dWhite',
    'd-black': 'shortcuts.slot.dBlack',
    'd-undo': 'shortcuts.slot.dUndo',
    'd-redo': 'shortcuts.slot.dRedo',
    'd-copy': 'shortcuts.slot.dCopy',
    'd-save': 'shortcuts.slot.dSave',
    'd-restore': 'shortcuts.slot.dRestore',
    'd-center': 'shortcuts.slot.dCenter',
    'd-zin': 'shortcuts.slot.dZin',
    'd-zout': 'shortcuts.slot.dZout',
    'd-wup': 'shortcuts.slot.dWup',
    'd-wdown': 'shortcuts.slot.dWdown',
    'd-help': 'shortcuts.slot.dHelp',
    'd-exit': 'shortcuts.slot.dExit',
    'd-spot': 'shortcuts.slot.dSpot',
    'd-check': 'shortcuts.slot.dCheck',
    'd-number': 'shortcuts.slot.dNumber',
    'z-exit': 'shortcuts.slot.zExit',
  };

  /**
   * S02 설정 · 단축키 화면(F04).
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   onRecordingChange?: (recording: boolean) => void,
   * }}
   */
  let { client, onRecordingChange = undefined } = $props();

  let state = $state(null);
  let scopeTab = $state('global');
  let recordingId = $state(null);
  let shakingId = $state(null);
  let feedbackById = $state({});
  let exportingPdf = $state(false);

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

  $effect(() => {
    onRecordingChange?.(recordingId !== null);
  });

  let platform = $derived(state?.platform ?? 'windows');
  let shortcuts = $derived(state?.shortcuts ?? []);
  let globalRows = $derived(shortcuts.filter((s) => s.scope === 'global'));
  let inAppRows = $derived(shortcuts.filter((s) => s.scope !== 'global'));
  let visibleGroups = $derived(
    SHORTCUT_GROUPS.filter((g) => (scopeTab === 'global' ? g.scope === 'global' : g.scope !== 'global')),
  );
  let conflicts = $derived(state?.conflicts ?? { count: 0, app: undefined, combos: [] });
  let conflictHeadline = $derived.by(() => {
    const raw = formatConflictBanner(conflicts, platform);
    if (!raw) return null;
    if (!conflicts.app) {
      return { lead: '', message: t('shortcuts.conflict.generic', { n: conflicts.count }) };
    }
    return raw;
  });
  let bannerApp = $derived(scopeTab === 'global' && conflicts.count > 0 ? conflicts.app || '' : '');

  function rowsForGroup(groupId) {
    return shortcuts.filter((s) => s.group === groupId);
  }

  function groupTitle(group) {
    const key = GROUP_I18N_KEYS[group.id];
    return key ? t(key) : group.title;
  }

  function rowLabel(row) {
    const key = SLOT_I18N_KEYS[row.id];
    return key ? t(key) : row.label;
  }

  async function handleExportPdf() {
    if (exportingPdf) return;
    exportingPdf = true;
    try {
      await exportShortcutSheetPdf(client, {
        shortcuts,
        lang: locale.lang,
        os: platform,
        version: state?.version,
      });
    } finally {
      exportingPdf = false;
    }
  }

  function startRecording(id) {
    if (recordingId === id) {
      recordingId = null;
      return;
    }
    recordingId = id;
    const next = { ...feedbackById };
    delete next[id];
    feedbackById = next;
  }

  function triggerShake(id) {
    shakingId = id;
    setTimeout(() => {
      if (shakingId === id) shakingId = null;
    }, 220);
  }

  async function commitCombo(id, combo) {
    if (combo) {
      await client.previewShortcut(id, combo);
    }
    const res = await client.setShortcut(id, combo);
    if (res.ok) {
      recordingId = null;
      const next = { ...feedbackById };
      delete next[id];
      feedbackById = next;
    } else {
      feedbackById = {
        ...feedbackById,
        [id]: {
          error: res.reason || t('shortcuts.error.cannotRegister'),
          suggestions: res.suggestions || [],
        },
      };
      triggerShake(id);
    }
  }

  function handleWindowKeydown(e) {
    if (!recordingId) return;
    e.preventDefault();
    e.stopPropagation();

    if (isEscapeOnly(e)) {
      recordingId = null;
      return;
    }

    if (isClearOnly(e)) {
      commitCombo(recordingId, '');
      return;
    }

    const combo = eventToCombo(e, platform);
    if (!combo) return;
    commitCombo(recordingId, combo);
  }

  function focusFirstConflict() {
    scopeTab = 'global';
    const first = globalRows.find((r) => r.status === 'conflict');
    if (first) startRecording(first.id);
  }

  async function resetGroup(groupId) {
    recordingId = null;
    feedbackById = {};
    await client.resetShortcutGroup(groupId);
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="ink-page">
  <header class="ink-page-head" data-tauri-drag-region>
    <h1 class="ink-page-title" data-tauri-drag-region>{t('shortcuts.title')}</h1>
    <button
      type="button"
      class="ink-pdf-btn ink-focus"
      disabled={exportingPdf}
      onclick={handleExportPdf}
    >
      <Icon name="save" size={14} />
      <span>{exportingPdf ? t('pdf.action.generating') : t('pdf.action.download')}</span>
    </button>
    <Tabs.Root value={scopeTab} onValueChange={(v) => { scopeTab = v; recordingId = null; }}>
      <Tabs.List aria-label={t('shortcuts.scope.aria')} class="ink-scope-tabs">
        <Tabs.Trigger value="global" class="ink-scope-tab ink-focus">
          <span>{t('shortcuts.scope.global')}</span>
          <span class="ink-scope-count ink-mono">{globalRows.length}</span>
        </Tabs.Trigger>
        <Tabs.Trigger value="inapp" class="ink-scope-tab ink-focus">
          <span>{t('shortcuts.scope.inapp')}</span>
          <span class="ink-scope-count ink-mono">{inAppRows.length}</span>
        </Tabs.Trigger>
      </Tabs.List>
    </Tabs.Root>
  </header>

  <div class="ink-page-body ink-scroll">
    {#if scopeTab === 'global' && conflictHeadline}
      <Banner
        kind="conflict"
        lead={conflictHeadline.lead}
        message={conflictHeadline.message}
        primaryLabel={conflicts.app ? t('shortcuts.conflict.howto') : t('shortcuts.conflict.change')}
        onPrimary={() => (conflicts.app ? client.run('snipaste.howto') : focusFirstConflict())}
        secondaryLabel={conflicts.app ? t('shortcuts.conflict.change') : ''}
        onSecondary={conflicts.app ? focusFirstConflict : undefined}
      />
    {/if}

    {#each visibleGroups as group (group.id)}
      {@const items = rowsForGroup(group.id)}
      {#if items.length > 0}
        <section class="ink-section">
          <SectionLabel
            title={groupTitle(group)}
            actionLabel={t('shortcuts.action.resetGroup')}
            onAction={() => resetGroup(group.id)}
          />
          <Group>
            {#each items as row (row.id)}
              {@const isRec = recordingId === row.id}
              {@const disp = getShortcutRowDisplay(row, {
                isRecording: isRec,
                feedback: feedbackById[row.id],
                bannerApp,
              })}
              <Row
                label={rowLabel(row)}
                later={Boolean(row.later)}
                recording={isRec}
                shaking={shakingId === row.id}
                clickable={true}
                status={disp.dotStatus}
                sub={isRec ? t('shortcuts.recording.hint') : disp.sub}
                hoverSub={disp.hoverSub}
                note={disp.note}
                error={disp.error}
                warning={disp.warning}
                suggestions={disp.suggestions}
                onSelectSuggestion={(combo) => commitCombo(row.id, combo)}
                onclick={() => startRecording(row.id)}
              >
                {#if row.combo}
                  <Keycap
                    combo={row.combo}
                    {platform}
                    variant={isRec ? 'waiting' : 'default'}
                  />
                {:else}
                  <span class="ink-empty-key">{isRec ? t('shortcuts.state.recording') : t('shortcuts.state.empty')}</span>
                {/if}
              </Row>
            {/each}
          </Group>
        </section>
      {/if}
    {/each}
  </div>
</div>

<style>
  .ink-page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ink-page-head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 18px 60px 12px 26px;
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
    gap: 12px;
  }

  .ink-page-title {
    margin: 0;
    flex: 1;
    font-size: 19px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
    user-select: none;
  }

  .ink-pdf-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 11px;
    border: 0;
    border-radius: var(--ink-r-btn);
    background: var(--ink-btn-sec-bg);
    box-shadow: var(--ink-btn-sec-shadow);
    color: var(--ink-text);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .ink-pdf-btn:hover:not(:disabled) {
    background: var(--ink-hover);
  }

  .ink-pdf-btn:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  :global(.ink-scope-tabs) {
    display: inline-flex;
    padding: var(--ink-seg-pad);
    gap: var(--ink-seg-gap);
    border-radius: var(--ink-r-seg);
    background: var(--ink-seg-bg);
    box-shadow: var(--ink-seg-ring);
    overflow: hidden;
  }

  :global(.ink-scope-tab) {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--ink-r-seg-item);
    background: transparent;
    color: var(--ink-text-2);
    font-family: inherit;
    font-size: 12px;
    font-weight: 400;
    white-space: nowrap;
    cursor: pointer;
  }

  :global(.ink-scope-tab[data-state='active']) {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    font-weight: 500;
  }

  .ink-scope-count {
    font-size: 11px;
    color: var(--ink-sub);
  }

  :global(.ink-scope-tab[data-state='active']) .ink-scope-count {
    color: var(--ink-key-on-invert-text);
  }

  .ink-section {
    display: flex;
    flex-direction: column;
  }

  .ink-empty-key {
    font-size: 11px;
    color: var(--ink-sub);
    white-space: nowrap;
  }
</style>
