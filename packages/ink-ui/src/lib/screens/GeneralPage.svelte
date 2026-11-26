<script>
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Switch from '../components/Switch.svelte';
  import Segmented from '../components/Segmented.svelte';
  import Sheet from '../components/Sheet.svelte';
  import Keycap from '../components/Keycap.svelte';
  import Icon from '../icons/Icon.svelte';
  import ConflictHowtoModal from './onboarding/ConflictHowtoModal.svelte';
  import { describeSettingChange } from '../models/settingsMeta.js';
  import { shortenPath } from '../models/template.js';
  import { summarizeCheckup } from '../models/onboarding.js';
  import { t } from '../i18n/index.svelte.js';

  /**
   * S05 설정 · 일반 화면(F07 · R2-ONB-6 · R2-FIX-1, 13).
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   onNavigateTab?: (tab: 'shortcuts' | 'capture' | 'draw' | 'general' | 'help' | 'info') => void,
   *   onOpenFirstRun?: () => void,
   * }}
   */
  let {
    client,
    onNavigateTab = undefined,
    onOpenFirstRun = undefined,
  } = $props();

  let state = $state(null);
  let resetSheetOpen = $state(false);
  let importSheetOpen = $state(false);
  let importPreview = $state(null);
  let importError = $state('');
  let screenAsked = $state(false);
  let howtoOpen = $state(false);
  let checkingUpdate = $state(false);

  function runAction(action) {
    return client.run(/** @type {any} */ (action));
  }

  async function handleCheckUpdate() {
    if (checkingUpdate) return;
    checkingUpdate = true;
    try {
      await runAction('update.check');
    } catch {
      // 확인 실패 안내는 Rust 섬에서 처리
    } finally {
      checkingUpdate = false;
    }
  }

  function refreshPermsIfMac() {
    if (state?.platform === 'macos') {
      void runAction('perm.refresh');
    }
  }

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (!alive) return;
      state = s;
      if (s?.platform === 'macos') void runAction('perm.refresh');
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') state = e.state;
    });
    return () => {
      alive = false;
      off();
    };
  });

  function handleWindowFocus() {
    refreshPermsIfMac();
  }

  function handleVisibilityChange() {
    if (typeof document !== 'undefined' && document.visibilityState === 'visible') {
      refreshPermsIfMac();
    }
  }

  let settings = $derived(state?.settings ?? {});
  let platform = $derived(state?.platform ?? 'windows');
  let pinnedCount = $derived(state?.pinnedCount ?? 0);
  let autostartNotice = $derived(state?.autostartNotice ?? '');
  let checkup = $derived(summarizeCheckup(state, { screenAsked }));
  let conflicts = $derived(state?.conflicts ?? { count: 0, app: undefined, combos: [] });

  let locationText = $derived.by(() => {
    const loc = state?.location;
    if (!loc) return '';
    const isMsix =
      Boolean(loc.packaged) ||
      state?.channel === 'msix-store' ||
      state?.channel === 'msix-direct';
    const kind = isMsix
      ? t('general.data.locationKindMsix')
      : loc.portable
        ? t('general.data.locationKindPortable')
        : t('general.data.locationKindInstalled');
    const short = shortenPath(loc.path ?? '', 32);
    return short ? `${kind} · ${short}` : kind;
  });

  let importChanges = $derived(
    (importPreview?.changes ?? []).map((c) => describeSettingChange(c, state?.shortcuts ?? [])),
  );
  let importWarnings = $derived(importPreview?.warnings ?? []);

  function setSetting(path, value) {
    client.updateSetting(path, value);
  }

  async function handleResetAll() {
    await client.resetAll();
  }

  async function handleImportClick() {
    importError = '';
    const res = await client.previewImport();
    if (!res || res.cancelled) return;
    if (!res.ok) {
      importError = res.error || t('general.data.importInvalid');
      return;
    }
    importPreview = res;
    importSheetOpen = true;
  }

  async function handleImportConfirm() {
    if (!importPreview?.path) return;
    const path = importPreview.path;
    importPreview = null;
    const res = await client.applyImport(path);
    if (res && !res.ok) {
      importError = res.error || t('general.data.importFailed');
    }
  }

  function handleImportCancel() {
    importPreview = null;
  }

  function handleFirstRunClick() {
    if (onOpenFirstRun) {
      onOpenFirstRun();
    } else {
      client.run('firstRun.open');
    }
  }
</script>

<svelte:window onfocus={handleWindowFocus} onvisibilitychange={handleVisibilityChange} />

<div class="ink-page">
  <header class="ink-page-head" data-tauri-drag-region>
    <h1 class="ink-page-title" data-tauri-drag-region>{t('general.title')}</h1>
  </header>

  <div class="ink-page-body ink-scroll">
    <section class="ink-section" data-setting-id="general.checkup">
      <div class="ink-checkup-head">
        <SectionLabel title={t('general.checkup.title')} />
        <span class="ink-checkup-summary" data-ok={checkup.allOk ? 'true' : undefined}>
          <Icon name={checkup.allOk ? 'check-circle' : 'warning-triangle'} size={13} />
          <span>
            {checkup.allOk
              ? t('general.checkup.summaryAllOk')
              : t('general.checkup.summaryIssues', { n: checkup.issueCount })}
          </span>
        </span>
      </div>
      <Group>
        {#if checkup.platform === 'macos'}
          {@const screenItem = checkup.items[0]}
          {@const axItem = checkup.items[1]}
          <Row
            label={t('general.checkup.screenLabel')}
            sub={screenItem.status === 'ok'
              ? t('general.checkup.screenOk')
              : screenItem.status === 'revoked'
                ? t('general.checkup.screenRevoked')
                : screenItem.status === 'relaunch'
                  ? t('general.checkup.screenRelaunch')
                  : t('general.checkup.screenOff')}
            status={screenItem.status === 'ok' ? 'ok' : 'attn'}
          >
            <div class="ink-inline-actions">
              {#if screenItem.status !== 'ok'}
                <button
                  type="button"
                  class="ink-sec-btn ink-focus"
                  onclick={() => {
                    screenAsked = true;
                    runAction('perm.screen');
                  }}
                >
                  {t('general.checkup.openBtn')}
                </button>
                <button
                  type="button"
                  class="ink-sec-btn ink-focus"
                  onclick={() => runAction('app.relaunch')}
                >
                  {t('general.checkup.relaunchBtn')}
                </button>
              {:else}
                <button
                  type="button"
                  class="ink-sec-btn ink-focus"
                  onclick={() => runAction('perm.screen')}
                >
                  {t('general.checkup.openBtn')}
                </button>
              {/if}
            </div>
          </Row>

          <Row
            label={t('general.checkup.axLabel')}
            sub={axItem.status === 'ok'
              ? t('general.checkup.axOk')
              : axItem.status === 'revoked'
                ? t('general.checkup.axRevoked')
                : t('general.checkup.axOff')}
            status={axItem.status === 'ok' ? 'ok' : 'attn'}
          >
            <button
              type="button"
              class="ink-sec-btn ink-focus"
              onclick={() => runAction('perm.accessibility')}
            >
              {t('general.checkup.openBtn')}
            </button>
          </Row>
        {:else}
          {@const conflictItem = checkup.items[0]}
          {@const autoItem = checkup.items[1]}
          <Row
            label={t('general.checkup.conflictsLabel')}
            sub={conflictItem.status === 'ok'
              ? t('general.checkup.conflictsOk')
              : conflictItem.app
                ? t('general.checkup.conflictsApp', { app: conflictItem.app, n: conflictItem.count })
                : t('general.checkup.conflictsGeneric', { n: conflictItem.count })}
            status={conflictItem.status === 'ok' ? 'ok' : 'attn'}
          >
            <div class="ink-inline-actions">
              {#if conflictItem.status !== 'ok' && conflictItem.app}
                <button
                  type="button"
                  class="ink-sec-btn ink-focus"
                  onclick={() => (howtoOpen = true)}
                >
                  {t('general.checkup.howtoBtn')}
                </button>
              {/if}
              <button
                type="button"
                class="ink-sec-btn ink-focus"
                onclick={() => onNavigateTab?.('shortcuts')}
              >
                {t('general.checkup.shortcutsBtn')}
              </button>
            </div>
          </Row>

          <Row
            label={t('general.checkup.autostartLabel')}
            sub={autoItem.notice
              ? autoItem.notice
              : autoItem.enabled
                ? t('general.checkup.autostartOn')
                : t('general.checkup.autostartOff')}
            status={autoItem.status === 'ok' ? 'ok' : autoItem.status === 'warn' ? 'attn' : ''}
          >
            {#if !autoItem.enabled}
              <button
                type="button"
                class="ink-sec-btn ink-focus"
                onclick={() => setSetting('general.autostart', true)}
              >
                {t('general.checkup.enableBtn')}
              </button>
            {/if}
          </Row>
        {/if}
      </Group>
    </section>

    <section class="ink-section">
      <SectionLabel title={t('general.appearance.title')} />
      <Group>
        <div data-setting-id="general.theme">
          <Row label={t('general.appearance.theme')}>
            <Segmented
              label={t('general.appearance.theme')}
              value={settings['general.theme'] ?? 'system'}
              options={[
                { value: 'system', label: t('general.appearance.themeSystem') },
                { value: 'light', label: t('general.appearance.themeLight') },
                { value: 'dark', label: t('general.appearance.themeDark') },
              ]}
              onChange={(v) => setSetting('general.theme', v)}
            />
          </Row>
        </div>
        <div data-setting-id="general.lang">
          <Row label={t('general.appearance.lang')}>
            <Segmented
              label={t('general.appearance.lang')}
              value={settings['general.lang'] ?? 'ko'}
              options={[
                { value: 'ko', label: t('general.appearance.langKo') },
                { value: 'en', label: t('general.appearance.langEn') },
              ]}
              onChange={(v) => setSetting('general.lang', v)}
            />
          </Row>
        </div>
      </Group>
    </section>

    <section class="ink-section">
      <SectionLabel title={t('general.startup.title')} />
      <Group>
        <div data-setting-id="general.autostart">
          <Row
            label={t('general.startup.autostart')}
            sub={t('general.startup.autostartSub')}
            warning={autostartNotice}
          >
            <Switch
              checked={Boolean(settings['general.autostart'])}
              label={t('general.startup.autostart')}
              onCheckedChange={(v) => setSetting('general.autostart', v)}
            />
          </Row>
        </div>
        <div data-setting-id="general.firstRun">
          <Row label={t('general.startup.firstRun')}>
            <button
              type="button"
              class="ink-sec-btn ink-focus"
              onclick={handleFirstRunClick}
            >
              {t('general.startup.reviewBtn')}
            </button>
          </Row>
        </div>
      </Group>
    </section>

    {#if platform !== 'macos'}
      <section class="ink-section" data-setting-id="general.topmost">
        <SectionLabel title={t('general.topmost.title')} tag={t('general.topmost.tag')} />
        <Group>
          <Row label={t('general.topmost.border')} sub="px" subMono={true}>
            <Segmented
              label={t('general.topmost.borderAria')}
              mono={true}
              value={Number(settings['topmost.border'] ?? 4)}
              options={[
                { value: 0, label: t('general.topmost.borderOff') },
                { value: 2, label: '2' },
                { value: 4, label: '4' },
                { value: 6, label: '6' },
              ]}
              onChange={(v) => setSetting('topmost.border', Number(v))}
            />
          </Row>
          <Row label={t('general.topmost.sound')}>
            <Switch
              checked={Boolean(settings['topmost.sound'])}
              label={t('general.topmost.sound')}
              onCheckedChange={(v) => setSetting('topmost.sound', v)}
            />
          </Row>
          <Row
            label={t('general.topmost.pinned')}
            sub={t('general.topmost.pinnedCount', { n: pinnedCount })}
            subMono={true}
          >
            <button
              type="button"
              class="ink-sec-btn ink-focus"
              disabled={pinnedCount === 0}
              onclick={() => client.run('pin.releaseAll')}
            >
              {t('general.topmost.releaseAll')}
            </button>
          </Row>
        </Group>
      </section>
    {/if}

    <section class="ink-section">
      <SectionLabel title={t('general.data.title')} />
      <Group>
        <div data-setting-id="general.location">
          <Row label={t('general.data.location')} sub={locationText} subMono={true}>
            <button
              type="button"
              class="ink-sec-btn ink-focus"
              aria-label={t('general.data.openLocationAria')}
              onclick={() => runAction('settings.reveal')}
            >
              {t('general.data.openBtn')}
            </button>
          </Row>
        </div>
        <div data-setting-id="general.transfer">
          <Row label={t('general.data.transfer')} error={importError}>
            <div class="ink-inline-actions">
              <button
                type="button"
                class="ink-sec-btn ink-focus"
                onclick={() => client.run('settings.export')}
              >
                {t('general.data.export')}
              </button>
              <button
                type="button"
                class="ink-sec-btn ink-focus"
                onclick={handleImportClick}
              >
                {t('general.data.import')}
              </button>
            </div>
          </Row>
        </div>
        <div data-setting-id="general.logs">
          <Row label={t('general.data.logs')} sub={t('general.data.logsSub')}>
            <button
              type="button"
              class="ink-sec-btn ink-focus"
              onclick={() => client.run('logs.open')}
            >
              {t('general.data.logsOpen')}
            </button>
          </Row>
        </div>
        <div data-setting-id="general.reset">
          <Row
            label={t('general.data.reset')}
            sub={t('general.data.resetSub')}
            danger={true}
          >
            <button
              type="button"
              class="ink-danger-btn ink-focus"
              onclick={() => (resetSheetOpen = true)}
            >
              {t('general.data.resetBtn')}
            </button>
          </Row>
        </div>
      </Group>
    </section>

    <section class="ink-section" data-setting-id="general.update">
      <SectionLabel title={t('updateUi.sectionTitle')} />
      <Group>
        <div data-setting-id="update.auto">
          <Row label={t('updateUi.autoLabel')} sub={t('updateUi.autoSub')}>
            <Switch
              checked={settings['update.auto'] ?? true}
              label={t('updateUi.autoLabel')}
              onCheckedChange={(v) => setSetting('update.auto', v)}
            />
          </Row>
        </div>
        <div data-setting-id="update.check">
          <Row
            label={t('updateUi.versionLabel')}
            sub={`v${state?.version ?? '0.1.0'}`}
            subMono={true}
          >
            <button
              type="button"
              class="ink-sec-btn ink-focus"
              disabled={checkingUpdate}
              onclick={handleCheckUpdate}
            >
              {checkingUpdate ? t('updateUi.checkingBtn') : t('updateUi.checkBtn')}
            </button>
          </Row>
        </div>
      </Group>
    </section>
  </div>
</div>

<Sheet
  bind:open={importSheetOpen}
  width="md"
  title={importChanges.length > 0
    ? t('general.importSheet.titleCount', { n: importChanges.length })
    : t('general.importSheet.titleEmpty')}
  description={importChanges.length > 0
    ? t('general.importSheet.descCount')
    : t('general.importSheet.descEmpty')}
  confirmLabel={t('general.importSheet.confirm')}
  cancelLabel={t('general.importSheet.cancel')}
  confirmDisabled={importChanges.length === 0}
  onConfirm={handleImportConfirm}
  onCancel={handleImportCancel}
>
  {#if importChanges.length > 0}
    <div class="ink-import-list ink-scroll">
      {#each importChanges as item (item.path)}
        <div class="ink-import-row">
          <span class="ink-import-label">{item.label}</span>
          <span class="ink-import-diff">
            {#if item.isShortcut}
              {#if item.beforeCombo}
                <Keycap combo={item.beforeCombo} {platform} variant="dim" />
              {:else}
                <span class="ink-import-before">{t('general.importSheet.emptyKey')}</span>
              {/if}
              <span class="ink-import-arrow" aria-hidden="true">→</span>
              {#if item.afterCombo}
                <Keycap combo={item.afterCombo} {platform} />
              {:else}
                <span class="ink-import-after">{t('general.importSheet.emptyKey')}</span>
              {/if}
            {:else}
              <span class="ink-import-before">{item.beforeText}</span>
              <span class="ink-import-arrow" aria-hidden="true">→</span>
              <span class="ink-import-after">{item.afterText}</span>
            {/if}
          </span>
        </div>
      {/each}
    </div>
  {/if}
  {#if importWarnings.length > 0}
    <div class="ink-import-warns">
      {#each importWarnings as warn, i (i)}
        <p class="ink-import-warn">{warn}</p>
      {/each}
    </div>
  {/if}
</Sheet>

<Sheet
  bind:open={resetSheetOpen}
  title={t('general.resetSheet.title')}
  description={t('general.resetSheet.desc')}
  confirmLabel={t('general.resetSheet.confirm')}
  cancelLabel={t('general.resetSheet.cancel')}
  danger={true}
  dangerDelaySeconds={3}
  onConfirm={handleResetAll}
/>

<ConflictHowtoModal
  bind:open={howtoOpen}
  appName={conflicts.app ?? ''}
  onOpenShortcuts={() => onNavigateTab?.('shortcuts')}
/>

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
    padding: 18px 60px 12px 26px;
  }

  .ink-page-title {
    margin: 0;
    font-family: var(--ink-font-display);
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
    gap: 12px;
  }

  .ink-section {
    display: flex;
    flex-direction: column;
  }

  .ink-checkup-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .ink-checkup-summary {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 500;
    color: var(--ink-attn);
    margin-bottom: 6px;
  }

  .ink-checkup-summary[data-ok='true'] {
    color: var(--ink-ok);
  }

  .ink-inline-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .ink-sec-btn {
    height: 24px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--ink-r-btn);
    background: var(--ink-btn-sec-bg);
    box-shadow: var(--ink-btn-sec-shadow);
    color: var(--ink-text);
    font-family: inherit;
    font-size: 12px;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .ink-sec-btn:hover:not(:disabled) {
    background: var(--ink-hover);
  }

  .ink-sec-btn:active:not(:disabled) {
    opacity: 0.85;
  }

  .ink-sec-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .ink-danger-btn {
    height: 26px;
    padding: 0 11px;
    border: 0;
    border-radius: var(--ink-r-btn);
    background: color-mix(in srgb, var(--ink-danger) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink-danger) 35%, transparent);
    color: var(--ink-danger-text);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .ink-danger-btn:hover {
    background: var(--ink-danger);
    color: var(--ink-island-text);
  }

  .ink-import-list {
    max-height: 220px;
    overflow-y: auto;
    border-top: 1px solid var(--ink-hair);
    border-bottom: 1px solid var(--ink-hair);
  }

  .ink-import-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 30px;
    padding: 4px 2px;
    border-top: 1px solid var(--ink-hair);
    font-size: 12.5px;
  }

  .ink-import-row:first-child {
    border-top: 0;
  }

  .ink-import-label {
    color: var(--ink-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .ink-import-diff {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
    white-space: nowrap;
  }

  .ink-import-before {
    font-size: 11.5px;
    color: var(--ink-sub);
  }

  .ink-import-arrow {
    font-size: 11px;
    color: var(--ink-sub);
  }

  .ink-import-after {
    font-size: 12px;
    font-weight: 500;
    color: var(--ink-text);
  }

  .ink-import-warns {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .ink-import-warn {
    margin: 0;
    font-size: 11.5px;
    color: var(--ink-accent-text);
  }
</style>
