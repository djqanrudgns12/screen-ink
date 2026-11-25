<script>
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Switch from '../components/Switch.svelte';
  import Segmented from '../components/Segmented.svelte';
  import Sheet from '../components/Sheet.svelte';
  import {
    renderFilenamePreview,
    shortenPath,
    validateFilenameTemplate,
  } from '../models/template.js';

  /**
   * S03 설정 · 캡처·녹화 화면(F05).
   * @type {{ client: import('@screen-ink/client').InkClient }}
   */
  let { client } = $props();

  const MIC_DEVICES = [
    { id: '', label: '기본 장치' },
    { id: 'built-in', label: '내장 마이크(Realtek)' },
    { id: 'usb', label: 'USB 마이크' },
  ];

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
      if (!templateSynced) {
        draftTemplate = s.settings['output.template'] ?? '';
        templateSynced = true;
      }
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        state = e.state;
        const remote = e.state.settings['output.template'] ?? '';
        if (validateFilenameTemplate(draftTemplate) === null && draftTemplate !== remote) {
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
  let templateError = $derived(validateFilenameTemplate(draftTemplate));
  let templatePreview = $derived(renderFilenamePreview(draftTemplate));
  let excluded = $derived(Array.isArray(settings['capture.excluded']) ? settings['capture.excluded'] : []);
  let historyMax = $derived(Number(settings['capture.historyMax'] ?? 20));
  let micDeviceId = $derived(String(settings['record.micDevice'] ?? ''));
  let micDeviceLabel = $derived(
    MIC_DEVICES.find((d) => d.id === micDeviceId)?.label ?? (micDeviceId || '기본 장치'),
  );

  function setSetting(path, value) {
    client.updateSetting(path, value);
  }

  function handleTemplateInput(e) {
    const val = e.currentTarget.value;
    draftTemplate = val;
    if (validateFilenameTemplate(val) === null) {
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
    const exe = trimmed.endsWith('.exe') ? trimmed : `${trimmed}.exe`;
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
    <h1 class="ink-page-title" data-tauri-drag-region>캡처·녹화</h1>
  </header>

  <div class="ink-page-body ink-scroll">
  <section class="ink-section">
    <SectionLabel title="결과" />
    <Group>
      <Row label="Enter · 더블클릭 하면">
        <Segmented
          label="Enter · 더블클릭 결과"
          value={settings['capture.result'] ?? 'copy'}
          options={[
            { value: 'copy', label: '복사' },
            { value: 'save', label: '저장' },
            { value: 'both', label: '둘 다' },
          ]}
          onChange={(v) => setSetting('capture.result', v)}
        />
      </Row>
      <Row label="마우스 커서도 찍기" sub="캡처 중 ` 키">
        <Switch
          checked={Boolean(settings['capture.cursor'])}
          label="마우스 커서도 찍기"
          onCheckedChange={(v) => setSetting('capture.cursor', v)}
        />
      </Row>
      <Row label="찰칵 소리">
        <Switch
          checked={Boolean(settings['capture.sound'])}
          label="찰칵 소리"
          onCheckedChange={(v) => setSetting('capture.sound', v)}
        />
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title="저장" />
    <Group>
      <Row
        label="저장 폴더"
        sub={shortenPath(settings['output.folder'] ?? '')}
        subMono={true}
      >
        <div class="ink-inline-actions">
          <button
            type="button"
            class="ink-sec-btn ink-focus"
            onclick={() => client.run('folder.open')}
          >
            열기
          </button>
          <button
            type="button"
            class="ink-sec-btn ink-focus"
            onclick={() => client.run('folder.pick')}
          >
            바꾸기
          </button>
        </div>
      </Row>
      <Row label="파일 이름" error={templateError || ''}>
        <div class="ink-template-box">
          <input
            type="text"
            aria-label="파일 이름 규칙"
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
    <SectionLabel title="스마트 인식" />
    <Group>
      <Row label="UI 요소까지 찾기" sub="끄면 창 단위만">
        <Switch
          checked={Boolean(settings['capture.element'])}
          label="UI 요소까지 찾기"
          onCheckedChange={(v) => setSetting('capture.element', v)}
        />
      </Row>
      <Row label="모서리 자석" sub="Alt를 누르는 동안 끔">
        <Switch
          checked={Boolean(settings['capture.magnet'])}
          label="모서리 자석"
          onCheckedChange={(v) => setSetting('capture.magnet', v)}
        />
      </Row>
      <Row label="찾지 않을 앱">
        <div class="ink-chips">
          {#each excluded as item (item.exe)}
            <button
              type="button"
              class="ink-chip ink-mono ink-focus"
              onclick={() => removeExcluded(item.exe)}
              aria-label={`${item.exe} 빼기`}
            >
              {item.exe} ×
            </button>
          {/each}
          <span class="ink-chip ink-chip-auto">느림 · 창 단위 자동</span>
          {#if addingExe}
            <input
              type="text"
              aria-label="제외할 실행 파일 이름"
              placeholder="app.exe"
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
              추가
            </button>
          {/if}
        </div>
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title="녹화" />
    <Group>
      <Row label="부드러움">
        <Segmented
          label="녹화 초당 프레임"
          value={Number(settings['record.fps'] ?? 30)}
          options={[
            { value: 30, label: '30 fps' },
            { value: 60, label: '60 fps' },
          ]}
          onChange={(v) => setSetting('record.fps', Number(v))}
        />
      </Row>
      <Row
        label="마이크 소리"
        sub={settings['record.mic'] ? micDeviceLabel : '처음 켤 때 장치 고르기'}
      >
        <Switch
          checked={Boolean(settings['record.mic'])}
          label="마이크 소리"
          onCheckedChange={handleMicToggle}
        />
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title="기록" />
    <Group>
      <Row label="캡처 기록" sub={`${historyMax}개 보관`} subMono={true}>
        <button
          type="button"
          class="ink-sec-btn ink-focus"
          disabled={historyMax === 0}
          onclick={clearHistory}
        >
          기록 지우기
        </button>
      </Row>
    </Group>
  </section>
  </div>
</div>

<Sheet
  bind:open={micSheetOpen}
  title="마이크 고르기"
  description="녹화에 함께 담을 마이크 장치를 고르세요."
  confirmLabel="켜기"
  cancelLabel="취소"
  onConfirm={confirmMic}
>
  <div class="ink-mic-list">
    {#each MIC_DEVICES as dev (dev.id)}
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

  .ink-chip {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 9px;
    border: 0;
    border-radius: 11px;
    background: var(--ink-chip-bg);
    color: var(--ink-text-2);
    font-size: 11px;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-chip-auto {
    cursor: default;
    font-size: 11.5px;
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
