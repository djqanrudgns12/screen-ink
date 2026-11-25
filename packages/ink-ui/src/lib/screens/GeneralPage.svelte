<script>
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Switch from '../components/Switch.svelte';
  import Segmented from '../components/Segmented.svelte';
  import Sheet from '../components/Sheet.svelte';
  import Keycap from '../components/Keycap.svelte';
  import { describeSettingChange, formatLocationSub } from '../models/settingsMeta.js';

  /**
   * S05 설정 · 일반 화면(F07).
   * @type {{ client: import('@screen-ink/client').InkClient }}
   */
  let { client } = $props();

  let state = $state(null);
  let resetSheetOpen = $state(false);
  let importSheetOpen = $state(false);
  let importPreview = $state(null);
  let importError = $state('');

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
  let platform = $derived(state?.platform ?? 'windows');
  let pinnedCount = $derived(state?.pinnedCount ?? 0);
  let locationText = $derived(formatLocationSub(state?.location, 32));
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
      importError = res.error || '이 파일은 screen-ink 설정이 아니에요';
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
      importError = res.error || '설정을 가져오지 못했어요';
    }
  }

  function handleImportCancel() {
    importPreview = null;
  }
</script>

<div class="ink-page">
  <header class="ink-page-head" data-tauri-drag-region>
    <h1 class="ink-page-title" data-tauri-drag-region>일반</h1>
  </header>

  <div class="ink-page-body ink-scroll">
  <section class="ink-section">
    <SectionLabel title="모양" />
    <Group>
      <Row label="테마">
        <Segmented
          label="테마"
          value={settings['general.theme'] ?? 'system'}
          options={[
            { value: 'system', label: '시스템 따라' },
            { value: 'light', label: '라이트' },
            { value: 'dark', label: '다크' },
          ]}
          onChange={(v) => setSetting('general.theme', v)}
        />
      </Row>
      <Row label="언어">
        <Segmented
          label="언어"
          value={settings['general.lang'] ?? 'ko'}
          options={[
            { value: 'ko', label: '한국어' },
            { value: 'en', label: 'English' },
          ]}
          onChange={(v) => setSetting('general.lang', v)}
        />
      </Row>
    </Group>
  </section>

  <section class="ink-section">
    <SectionLabel title="시작" />
    <Group>
      <Row label="로그인하면 실행" sub="트레이에만 조용히">
        <Switch
          checked={Boolean(settings['general.autostart'])}
          label="로그인하면 실행"
          onCheckedChange={(v) => setSetting('general.autostart', v)}
        />
      </Row>
      <Row label="첫 실행 안내">
        <button
          type="button"
          class="ink-sec-btn ink-focus"
          onclick={() => client.run('firstRun.open')}
        >
          다시 보기
        </button>
      </Row>
    </Group>
  </section>

  {#if platform !== 'macos'}
    <section class="ink-section">
      <SectionLabel title="창 항상 위" tag="WINDOWS" />
      <Group>
        <Row label="고정한 창 테두리" sub="px" subMono={true}>
          <Segmented
            label="고정한 창 테두리 두께"
            mono={true}
            value={Number(settings['topmost.border'] ?? 4)}
            options={[
              { value: 0, label: '끔' },
              { value: 2, label: '2' },
              { value: 4, label: '4' },
              { value: 6, label: '6' },
            ]}
            onChange={(v) => setSetting('topmost.border', Number(v))}
          />
        </Row>
        <Row label="고정할 때 소리">
          <Switch
            checked={Boolean(settings['topmost.sound'])}
            label="고정할 때 소리"
            onCheckedChange={(v) => setSetting('topmost.sound', v)}
          />
        </Row>
        <Row label="고정한 창" sub={`${pinnedCount}개`} subMono={true}>
          <button
            type="button"
            class="ink-sec-btn ink-focus"
            disabled={pinnedCount === 0}
            onclick={() => client.run('pin.releaseAll')}
          >
            모두 풀기
          </button>
        </Row>
      </Group>
    </section>
  {/if}

  <section class="ink-section">
    <SectionLabel title="데이터" />
    <Group>
      <Row label="설정 위치" sub={locationText} subMono={true}>
        <button
          type="button"
          class="ink-sec-btn ink-focus"
          onclick={() => client.run('folder.open')}
        >
          열기
        </button>
      </Row>
      <Row label="설정 옮기기" error={importError}>
        <div class="ink-inline-actions">
          <button
            type="button"
            class="ink-sec-btn ink-focus"
            onclick={() => client.run('settings.export')}
          >
            내보내기
          </button>
          <button
            type="button"
            class="ink-sec-btn ink-focus"
            onclick={handleImportClick}
          >
            가져오기
          </button>
        </div>
      </Row>
      <Row label="로그" sub="7일 보관 · 창 제목은 남기지 않아요">
        <button
          type="button"
          class="ink-sec-btn ink-focus"
          onclick={() => client.run('logs.open')}
        >
          폴더 열기
        </button>
      </Row>
      <Row
        label="모두 처음으로"
        sub="확인을 한 번 더 받아요"
        danger={true}
        clickable={true}
        onclick={() => (resetSheetOpen = true)}
      />
    </Group>
  </section>
  </div>
</div>

<Sheet
  bind:open={importSheetOpen}
  width="md"
  title={importChanges.length > 0 ? `${importChanges.length}개 항목이 바뀌어요` : '바뀌는 항목이 없어요'}
  description={importChanges.length > 0 ? '지금 설정과 다른 항목만 아래에 모았어요.' : '지금 설정과 내용이 같아요.'}
  confirmLabel="가져오기"
  cancelLabel="취소"
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
                <span class="ink-import-before">비움</span>
              {/if}
              <span class="ink-import-arrow" aria-hidden="true">→</span>
              {#if item.afterCombo}
                <Keycap combo={item.afterCombo} {platform} />
              {:else}
                <span class="ink-import-after">비움</span>
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
  title="설정을 모두 처음으로 되돌릴까요?"
  description="바꾼 단축키와 설정이 모두 기본값으로 돌아가요."
  confirmLabel="되돌리기"
  cancelLabel="취소"
  danger={true}
  dangerDelaySeconds={3}
  onConfirm={handleResetAll}
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
