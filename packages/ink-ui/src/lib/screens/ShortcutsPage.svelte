<script>
  import { Tabs } from 'bits-ui';
  import { SHORTCUT_GROUPS } from '@screen-ink/client';
  import Banner from '../components/Banner.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import Keycap from '../components/Keycap.svelte';
  import { eventToCombo, isClearOnly, isEscapeOnly } from '../models/keyCombo.js';
  import { formatConflictBanner, getShortcutRowDisplay } from '../models/shortcutDisplay.js';

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
  let conflictHeadline = $derived(formatConflictBanner(conflicts, platform));
  let bannerApp = $derived(scopeTab === 'global' && conflicts.count > 0 ? conflicts.app || '' : '');

  function rowsForGroup(groupId) {
    return shortcuts.filter((s) => s.group === groupId);
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
          error: res.reason || '이 키는 등록할 수 없어요',
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
    <h1 class="ink-page-title" data-tauri-drag-region>단축키</h1>
    <Tabs.Root value={scopeTab} onValueChange={(v) => { scopeTab = v; recordingId = null; }}>
      <Tabs.List aria-label="단축키 범위" class="ink-scope-tabs">
        <Tabs.Trigger value="global" class="ink-scope-tab ink-focus">
          <span>어디서나</span>
          <span class="ink-scope-count ink-mono">{globalRows.length}</span>
        </Tabs.Trigger>
        <Tabs.Trigger value="inapp" class="ink-scope-tab ink-focus">
          <span>화면 안</span>
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
        primaryLabel={conflicts.app ? '끄는 법' : '키 바꾸기'}
        onPrimary={() => (conflicts.app ? client.run('snipaste.howto') : focusFirstConflict())}
        secondaryLabel={conflicts.app ? '키 바꾸기' : ''}
        onSecondary={conflicts.app ? focusFirstConflict : undefined}
      />
    {/if}

    {#each visibleGroups as group (group.id)}
      {@const items = rowsForGroup(group.id)}
      {#if items.length > 0}
        <section class="ink-section">
          <SectionLabel
            title={group.title}
            actionLabel="기본값"
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
                label={row.label}
                later={Boolean(row.later)}
                recording={isRec}
                shaking={shakingId === row.id}
                clickable={true}
                status={disp.dotStatus}
                sub={disp.sub}
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
                  <span class="ink-empty-key">{isRec ? '입력 중' : '비움'}</span>
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
    gap: 14px;
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
