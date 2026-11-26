<script>
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Switch from '../components/Switch.svelte';
  import Segmented from '../components/Segmented.svelte';
  import Sheet from '../components/Sheet.svelte';
  import { locale, syncLang, t } from '../i18n/index.svelte.js';
  import { formatComboReadable } from '../models/shortcutDisplay.js';
  import {
    renderFilenamePreview,
    shortenPath,
    validateFilenameTemplate,
  } from '../models/template.js';

  /**
   * S03 설정 · 캡처·녹화 화면(F05 · R2-FIX-5·9·13).
   * @type {{ client: import('@screen-ink/client').InkClient }}
   */
  let { client } = $props();

  let state = $state(null);
  let draftTemplate = $state('');
  let templateSynced = $state(false);
  let addingExe = $state(false);
  let exeInput = $state('');
  let micSheetOpen = $state(false);
  let selectedMic = $state('');

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (!alive) return;
      state = s;
      syncLang(s);
      if (!templateSynced) {
        draftTemplate = s.settings['output.template'] ?? '';
        templateSynced = true;
      }
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        state = e.state;
        syncLang(e.state);
        const remote = e.state.settings['output.template'] ?? '';
        if (validateFilenameTemplate(draftTemplate, locale.lang) === null && draftTemplate !== remote) {
          draftTemplate = remote;
        }
      }
    });
    return () => {
      alive = false;
      off();
    };
  });

  let settings = $derived(state?.settings ?? {});
  let platform = $derived(state?.platform ?? 'windows');
  let micDevices = $derived([
    { id: '', label: t('page.capture.micDefault') },
    { id: 'built-in', label: t('page.capture.micBuiltIn') },
    { id: 'usb', label: t('page.capture.micUsb') },
  ]);
  let cursorCombo = $derived(state?.shortcuts?.find((s) => s.id === 'c-cursor')?.combo ?? '');
  let cursorKeyLabel = $derived(formatComboReadable(cursorCombo, platform) || '`');
  let templateError = $derived(validateFilenameTemplate(draftTemplate, locale.lang));
  let templatePreview = $derived(renderFilenamePreview(draftTemplate));
  let excluded = $derived(Array.isArray(settings['capture.excluded']) ? settings['capture.excluded'] : []);
  let historyMax = $derived(Number(settings['capture.historyMax'] ?? 20));
  let micDeviceId = $derived(String(settings['record.micDevice'] ?? ''));
  let micDeviceLabel = $derived(
    micDevices.find((d) => d.id === micDeviceId)?.label ?? (micDeviceId || t('page.capture.micDefault')),
  );

  function setSetting(path, value) {
    client.updateSetting(path, value);
  }

  function handleTemplateInput(e) {
    const val = e.currentTarget.value;
    draftTemplate = val;
    if (validateFilenameTemplate(val, locale.lang) === null) {
      setSetting('output.template', val);
    }
  }

  function removeExcluded(exe) {
    setSetting(
      'capture.excluded',
      excluded.filter((item) => item.exe !== exe),
    );
  }

  function addExcluded() {
    const trimmed = exeInput.trim();
    if (!trimmed) {
      addingExe = false;
      return;
    }
    const exe =
      platform === 'macos'
        ? trimmed
        : trimmed.toLowerCase().endsWith('.exe')
          ? trimmed
          : `${trimmed}.exe`;
    if (!excluded.some((item) => item.exe.toLowerCase() === exe.toLowerCase())) {
      setSetting('capture.excluded', [...excluded, { exe, mode: 'exclude', auto: false }]);
    }
    exeInput = '';
    addingExe = false;
  }

  function handleMicToggle(next) {
    if (next && !settings['record.mic']) {
      selectedMic = micDeviceId;
      micSheetOpen = true;
    } else {
      setSetting('record.mic', false);
    }
  }

  function openMicPicker() {
    selectedMic = micDeviceId;
    micSheetOpen = true;
  }

  async function confirmMic() {
    await client.updateSetting('record.micDevice', selectedMic);
    await client.updateSetting('record.mic', true);
  }

  function clearHistory() {
    client.run('capture.history.clear');
  }
</script>

<div class="ink-page">
  <header class="ink-page-head" data-tauri-drag-region>
    <h1 class="ink-page-title" data-tauri-drag-region>{t('page.capture.title')}</h1>
  </header>

  <div class="ink-page-body ink-scroll">
  <section class="ink-section">
    <SectionLabel title={t('page.capture.section.result')} />
    <Group>
      <Row label={t('page.capture.result')}>
        <Segmented
          label={t('page.capture.resultAria')}
          value={settings['capture.result'] ?? 'copy'}
          options={[
            { value: 'copy', label: t('page.capture.result.copy') },
            { value: 'save', label: t('page.capture.result.save') },
            { value: 'both', label: t('page.capture.result.both') },
          ]}
          onChange={(v) => setSetting('capture.result', v)}
        />
      </Row>
      <Row label={t('page.capture.cursor')} sub={t('page.capture.cursorSub', { key: cursorKeyLabel })}>
        <Switch
          checked={Boolean(settings['capture.cursor'])}
          label={t('page.capture.cursor')}
          onCheckedChange={(v) => setSetting('capture.cursor', v)}
        />
      </Row>
      <Row label={t('page.capture.sound')}>
        <Switch
          checked={Boolean(settings['capture.sound'])}
          label={t('page.capture.sound')}
          onCheckedChange={(v) => setSetting('capture.sound', v)}
        />
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title={t('page.capture.section.save')} />
    <Group>
      <Row
        label={t('page.capture.folder')}
        sub={shortenPath(settings['output.folder'] ?? '')}
        subMono={true}
      >
        <div class="ink-inline-actions">
          <button
            type="button"
            class="ink-sec-btn ink-focus"
            aria-label={t('page.capture.folderOpenAria')}
            onclick={() => client.run('folder.open')}
          >
            {t('common.open')}
          </button>
          <button
            type="button"
            class="ink-sec-btn ink-focus"
            onclick={() => client.run('folder.pick')}
          >
            {t('common.change')}
          </button>
        </div>
      </Row>
      <Row
        label={t('page.capture.template')}
        sub={t('page.capture.templateTokensHint', { tokens: '{yyyy} {MM} {dd} {HH} {mm} {ss} {n}' })}
        error={templateError || ''}
      >
        <div class="ink-template-box">
          <input
            type="text"
            aria-label={t('page.capture.templateAria')}
            value={draftTemplate}
            oninput={handleTemplateInput}
            data-invalid={templateError ? 'true' : undefined}
            class="ink-text-input ink-mono ink-focus"
          />
          {#if !templateError}
            <span class="ink-template-preview ink-mono">→ {templatePreview}</span>
          {/if}
        </div>
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title={t('page.capture.section.smart')} />
    <Group>
      <Row label={t('page.capture.element')} sub={t('page.capture.elementSub')}>
        <Switch
          checked={Boolean(settings['capture.element'])}
          label={t('page.capture.element')}
          onCheckedChange={(v) => setSetting('capture.element', v)}
        />
      </Row>
      <Row label={t('page.capture.magnet')} sub={t('page.capture.magnetSub')}>
        <Switch
          checked={Boolean(settings['capture.magnet'])}
          label={t('page.capture.magnet')}
          onCheckedChange={(v) => setSetting('capture.magnet', v)}
        />
      </Row>
      <Row label={t('page.capture.excluded')} sub={t('page.capture.excludedAutoNote')}>
        <div class="ink-chips">
          {#each excluded as item (item.exe)}
            <button
              type="button"
              class="ink-chip ink-mono ink-focus"
              onclick={() => removeExcluded(item.exe)}
              aria-label={t('page.capture.excludedRemoveAria', { app: item.exe })}
            >
              <span>{item.exe}</span>
              <span class="ink-chip-x" aria-hidden="true">×</span>
            </button>
          {/each}
          {#if addingExe}
            <input
              type="text"
              aria-label={t('page.capture.excludedInputAria')}
              placeholder={t('page.capture.excludedPlaceholder')}
              bind:value={exeInput}
              onkeydown={(e) => {
                if (e.key === 'Enter') addExcluded();
                if (e.key === 'Escape') {
                  e.stopPropagation();
                  addingExe = false;
                }
              }}
              onblur={addExcluded}
              class="ink-chip-input ink-mono ink-focus"
            />
          {:else}
            <button type="button" class="ink-sec-btn ink-focus" onclick={() => (addingExe = true)}>
              {t('common.add')}
            </button>
          {/if}
        </div>
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title={t('page.capture.section.record')} />
    <Group>
      <Row label={t('page.capture.fps')}>
        <Segmented
          label={t('page.capture.fpsAria')}
          value={Number(settings['record.fps'] ?? 30)}
          options={[
            { value: 30, label: '30 fps' },
            { value: 60, label: '60 fps' },
          ]}
          onChange={(v) => setSetting('record.fps', Number(v))}
        />
      </Row>
      <Row
        label={t('page.capture.mic')}
        sub={settings['record.mic'] ? micDeviceLabel : t('page.capture.micPickOnFirst')}
      >
        <div class="ink-inline-actions">
          {#if settings['record.mic']}
            <button type="button" class="ink-sec-btn ink-focus" onclick={openMicPicker}>
              {t('common.change')}
            </button>
          {/if}
          <Switch
            checked={Boolean(settings['record.mic'])}
            label={t('page.capture.mic')}
            onCheckedChange={handleMicToggle}
          />
        </div>
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title={t('page.capture.section.history')} />
    <Group>
      <Row label={t('page.capture.history')} sub={t('page.capture.historySub', { n: historyMax })} subMono={true}>
        <button
          type="button"
          class="ink-sec-btn ink-focus"
          disabled={historyMax === 0}
          onclick={clearHistory}
        >
          {t('page.capture.historyClear')}
        </button>
      </Row>
    </Group>
  </section>
  </div>
</div>

<Sheet
  bind:open={micSheetOpen}
  title={t('page.capture.micSheetTitle')}
  description={t('page.capture.micSheetDesc')}
  confirmLabel={t('page.capture.micSheetConfirm')}
  cancelLabel={t('common.cancel')}
  onConfirm={confirmMic}
>
  <div class="ink-mic-list">
    {#each micDevices as dev (dev.id)}
      <button
        type="button"
        class="ink-mic-item ink-focus"
        data-active={selectedMic === dev.id ? 'true' : undefined}
        onclick={() => (selectedMic = dev.id)}
      >
        {dev.label}
      </button>
    {/each}
  </div>
</Sheet>

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

  .ink-inline-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
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

  .ink-template-box {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .ink-text-input {
    width: 196px;
    height: 26px;
    padding: 0 10px;
    box-sizing: border-box;
    border: 1px solid var(--ink-line);
    border-radius: var(--ink-r-row);
    background: var(--ink-input-bg);
    color: var(--ink-text);
    font-size: 11.5px;
  }

  .ink-text-input[data-invalid='true'] {
    border-color: var(--ink-danger);
  }

  .ink-template-preview {
    font-size: 11.5px;
    color: var(--ink-accent-text);
    white-space: nowrap;
  }

  .ink-chips {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-wrap: nowrap;
  }

  /* R2-FIX-9: 지울 수 있는 칩만 테두리 + × 로 표시 */
  .ink-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 8px;
    border: 1px solid var(--ink-line-strong);
    border-radius: 11px;
    background: var(--ink-surface);
    color: var(--ink-text-2);
    font-size: 11px;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-chip:hover {
    border-color: var(--ink-danger);
    color: var(--ink-danger-text);
  }

  .ink-chip-x {
    font-size: 12px;
    line-height: 1;
    opacity: 0.75;
  }

  .ink-chip-input {
    width: 96px;
    height: 22px;
    padding: 0 8px;
    border: 1px solid var(--ink-line-strong);
    border-radius: 11px;
    background: var(--ink-input-bg);
    color: var(--ink-text);
    font-size: 11px;
  }

  .ink-mic-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ink-mic-item {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--ink-line);
    border-radius: var(--ink-r-row);
    background: var(--ink-surface);
    color: var(--ink-text);
    font-family: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }

  .ink-mic-item[data-active='true'] {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    border-color: var(--ink-invert);
  }
</style>
