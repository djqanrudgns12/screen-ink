<script>
  import AppIcon from '../icons/AppIcon.svelte';
  import Icon from '../icons/Icon.svelte';
  import Banner from '../components/Banner.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import MakerCredit from '../components/MakerCredit.svelte';
  import ShortcutsPage from './ShortcutsPage.svelte';
  import CapturePage from './CapturePage.svelte';
  import DrawZoomPage from './DrawZoomPage.svelte';
  import GeneralPage from './GeneralPage.svelte';
  import FirstRunSheet from './FirstRunSheet.svelte';

  /**
   * 설정 창(S02~S05 통합 호스트, 900×600) + S06 첫 실행 시트.
   * 설정 창 안 Ctrl+Z(맥 Cmd+Z) = client.undo().
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   initialTab?: 'shortcuts' | 'capture' | 'draw' | 'general' | 'info',
   *   firstRun?: boolean,
   *   onClose?: () => void,
   *   onFirstRunClose?: () => void,
   *   class?: string,
   * }}
   */
  let {
    client,
    initialTab = 'shortcuts',
    firstRun = false,
    onClose = undefined,
    onFirstRunClose = undefined,
    class: className = '',
  } = $props();

  const NAV_ITEMS = [
    { id: 'shortcuts', label: '단축키', icon: 'shortcuts' },
    { id: 'capture', label: '캡처·녹화', icon: 'region' },
    { id: 'draw', label: '판서·확대', icon: 'draw' },
    { id: 'general', label: '일반', icon: 'settings' },
  ];

  let activeTab = $state('shortcuts');
  let state = $state(null);
  let isRecordingShortcut = $state(false);
  let firstRunOpen = $state(false);
  let noticesDismissed = $state(false);

  $effect(() => {
    activeTab = initialTab;
  });

  $effect(() => {
    firstRunOpen = firstRun;
  });

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

  let noticeText = $derived(
    !noticesDismissed && Array.isArray(state?.notices) && state.notices.length > 0
      ? state.notices.join(' · ')
      : '',
  );

  function handleWindowKeydown(e) {
    if (isRecordingShortcut || firstRunOpen) return;
    if (typeof document !== 'undefined' && document.querySelector('[data-dialog-content]')) {
      return;
    }

    const tag = e.target?.tagName;
    const isEditable = tag === 'INPUT' || tag === 'TEXTAREA' || e.target?.isContentEditable;
    const isMac = state?.platform === 'macos';
    const undoMod = isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;

    if (undoMod && !e.altKey && !e.shiftKey && (e.code === 'KeyZ' || e.key.toLowerCase() === 'z')) {
      if (!isEditable) {
        e.preventDefault();
        client.undo();
      }
      return;
    }

    if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
      onClose?.();
    }
  }

  function handleFirstRunClose() {
    firstRunOpen = false;
    onFirstRunClose?.();
  }

  function handleFirstRunShortcuts() {
    firstRunOpen = false;
    activeTab = 'shortcuts';
    isRecordingShortcut = false;
    onFirstRunClose?.();
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div role="dialog" aria-label="슉쇽 설정" class="ink-settings-win {className}">
  <nav aria-label="설정 묶음" class="ink-sidebar">
    <div class="ink-brand" data-tauri-drag-region>
      <span class="ink-brand-icon" data-tauri-drag-region>
        <AppIcon size={28} />
      </span>
      <div class="ink-brand-meta" data-tauri-drag-region>
        <p class="ink-brand-name" data-tauri-drag-region>슉쇽</p>
        <p class="ink-brand-ver ink-mono" data-tauri-drag-region>{state?.version ?? '0.1.0'}</p>
      </div>
    </div>

    {#each NAV_ITEMS as item (item.id)}
      {@const on = activeTab === item.id}
      <button
        type="button"
        class="ink-nav ink-focus"
        data-active={on ? 'true' : undefined}
        aria-current={on ? 'page' : undefined}
        onclick={() => {
          activeTab = item.id;
          isRecordingShortcut = false;
        }}
      >
        <Icon name={item.icon} size={16} accent={on} inverted={on} />
        <span>{item.label}</span>
      </button>
    {/each}

    <div class="ink-nav-spacer" data-tauri-drag-region></div>

    <button
      type="button"
      class="ink-nav ink-focus"
      data-active={activeTab === 'info' ? 'true' : undefined}
      aria-current={activeTab === 'info' ? 'page' : undefined}
      onclick={() => {
        activeTab = 'info';
        isRecordingShortcut = false;
      }}
    >
      <Icon name="info" size={16} accent={false} inverted={activeTab === 'info'} />
      <span>정보</span>
    </button>
  </nav>

  <main class="ink-main">
    <button
      type="button"
      aria-label="닫기"
      class="ink-close-btn ink-focus"
      onclick={() => onClose?.()}
    >
      <Icon name="close" size={16} />
    </button>

    {#if state?.readonly || noticeText}
      <div class="ink-top-banners">
        {#if state?.readonly}
          <Banner
            kind="conflict"
            message="다른 버전이 만든 설정이라 바꿔도 저장하지 않아요"
          />
        {/if}
        {#if noticeText}
          <Banner
            kind="info"
            message={noticeText}
            secondaryLabel="닫기"
            onSecondary={() => (noticesDismissed = true)}
          />
        {/if}
      </div>
    {/if}

    {#if activeTab === 'shortcuts'}
      <ShortcutsPage {client} onRecordingChange={(v) => (isRecordingShortcut = v)} />
    {:else if activeTab === 'capture'}
      <CapturePage {client} />
    {:else if activeTab === 'draw'}
      <DrawZoomPage {client} />
    {:else if activeTab === 'general'}
      <GeneralPage {client} />
    {:else}
      <div class="ink-info-page">
        <header class="ink-info-head" data-tauri-drag-region>
          <h1 class="ink-info-title" data-tauri-drag-region>정보</h1>
        </header>
        <div class="ink-info-body ink-scroll">
          <section>
            <SectionLabel title="앱" />
            <Group>
              <Row label="버전" sub={state?.version ?? '0.1.0'} subMono={true} />
              <Row label="만든 이" sub="찰떡쌤" />
              <Row label="라이선스" sub="MIT" />
              <Row label="오픈소스 고지" sub="IBM Plex · Svelte · bits-ui · Tauri" />
            </Group>
          </section>
          <section>
            <SectionLabel title="연결" tag="2차" />
            <Group>
              <Row label="통계·자동 업데이트" sub="1차 버전에서는 꺼져 있어요" />
            </Group>
          </section>
          <footer class="ink-info-credit">
            <p class="ink-info-credit-app">슉쇽 · v{state?.version ?? '0.1.0'}</p>
            <MakerCredit />
          </footer>
        </div>
      </div>
    {/if}
  </main>
</div>

{#if firstRunOpen}
  <FirstRunSheet
    {client}
    bind:open={firstRunOpen}
    onClose={handleFirstRunClose}
    onChangeShortcuts={handleFirstRunShortcuts}
  />
{/if}

<style>
  .ink-settings-win {
    position: relative;
    width: 900px;
    height: 600px;
    max-width: 100vw;
    max-height: 100vh;
    border-radius: var(--ink-r-window);
    overflow: hidden;
    display: flex;
    background: var(--ink-window-bg);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-window);
    font-family: var(--ink-font-sans);
  }

  .ink-sidebar {
    width: 196px;
    flex: none;
    box-sizing: border-box;
    padding: 16px 10px;
    background: var(--ink-sidebar);
    border-right: 1px solid var(--ink-sidebar-border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ink-brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 10px 16px;
    user-select: none;
  }

  .ink-brand-icon {
    display: inline-flex;
    flex: none;
  }

  .ink-brand-meta {
    min-width: 0;
  }

  .ink-brand-name {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
  }

  .ink-brand-ver {
    margin: 0;
    font-size: 10.5px;
    color: var(--ink-sub);
  }

  .ink-nav {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 31px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--ink-r-row);
    background: transparent;
    color: var(--ink-text-2);
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
    width: 100%;
    box-sizing: border-box;
  }

  .ink-nav:hover:not([data-active='true']) {
    background: var(--ink-hover);
    color: var(--ink-text);
  }

  .ink-nav[data-active='true'] {
    background: var(--ink-nav-on-bg);
    color: var(--ink-nav-on-text);
    font-weight: 500;
  }

  .ink-nav-spacer {
    flex: 1;
  }

  .ink-main {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-sizing: border-box;
  }

  .ink-top-banners {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 48px 0 26px;
  }

  .ink-close-btn {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 3;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--ink-close-r);
    background: var(--ink-close-bg);
    color: var(--ink-text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .ink-close-btn:hover {
    background: var(--ink-hover);
    color: var(--ink-text);
  }

  .ink-info-page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ink-info-head {
    flex: none;
    padding: 18px 60px 12px 26px;
  }

  .ink-info-title {
    margin: 0;
    font-size: 19px;
    font-weight: 600;
    color: var(--ink-text);
    user-select: none;
  }

  .ink-info-credit {
    margin-top: auto;
    padding-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ink-info-credit-app {
    margin: 0;
    font-size: 11px;
    line-height: 16px;
    color: var(--ink-sub);
    user-select: none;
  }

  .ink-info-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    padding: 2px 20px 22px 26px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
</style>
