<script>
  import { Dialog } from 'bits-ui';
  import AppIcon from '../icons/AppIcon.svelte';
  import Icon from '../icons/Icon.svelte';
  import Keycap from '../components/Keycap.svelte';
  import MakerCredit from '../components/MakerCredit.svelte';
  import Switch from '../components/Switch.svelte';
  import ConflictHowtoModal from './onboarding/ConflictHowtoModal.svelte';
  import { formatFirstRunConflict } from '../models/settingsMeta.js';
  import {
    ONBOARDING_STEP_COUNT,
    canAdvanceStep,
    canJumpToStep,
    getOnboardingSteps,
    resolveInitialStep,
  } from '../models/onboarding.js';
  import { syncLang, t } from '../i18n/index.svelte.js';

  /**
   * S06 첫 실행 카드 5장(R2-ONB-1~5, 7, 9 · R2-FIX-2, 3).
   * ① 반가워요 ② 화면 기록(맥)/단축키 겹침(Win) ③ 손쉬운 사용(맥)/켤 때 같이 켜기(Win) ④ 꼭 알아 둘 키 세 개 ⑤ 준비 끝
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   open?: boolean,
   *   initialStep?: number,
   *   onClose?: () => void,
   *   onChangeShortcuts?: () => void,
   * }}
   */
  let {
    client,
    open = $bindable(true),
    initialStep = undefined,
    onClose = undefined,
    onChangeShortcuts = undefined,
  } = $props();

  let state = $state(null);
  let step = $state(0);
  let stepInitialized = $state(false);
  let screenAsked = $state(false);
  let skippedPermissions = $state(false);
  let skipModalOpen = $state(false);
  let howtoOpen = $state(false);
  let justGrantedScreen = $state(false);
  let justGrantedAx = $state(false);
  let prevScreen = $state(/** @type {boolean | undefined} */ (undefined));
  let prevAx = $state(/** @type {boolean | undefined} */ (undefined));

  function runAction(action) {
    return client.run(/** @type {any} */ (action));
  }

  $effect(() => {
    let alive = true;
    const applyState = (s) => {
      if (!alive || !s) return;
      state = s;
      syncLang(s);

      if (!stepInitialized) {
        step =
          typeof initialStep === 'number'
            ? Math.max(0, Math.min(ONBOARDING_STEP_COUNT - 1, Math.trunc(initialStep)))
            : resolveInitialStep(s);
        stepInitialized = true;
      }

      const perms = s.permissions;
      if (perms) {
        if (prevScreen === false && perms.screen === true) {
          justGrantedScreen = true;
        }
        if (prevAx === false && perms.accessibility === true) {
          justGrantedAx = true;
        }
        prevScreen = Boolean(perms.screen);
        prevAx = Boolean(perms.accessibility);
      }
    };

    client.getState().then(applyState);
    const off = client.subscribe((e) => {
      if (e.type === 'state') applyState(e.state);
    });
    return () => {
      alive = false;
      off();
    };
  });

  $effect(() => {
    if (!open) {
      stepInitialized = false;
      skipModalOpen = false;
      howtoOpen = false;
    }
  });

  let platform = $derived(state?.platform ?? 'windows');
  let lang = $derived(state?.settings?.['general.lang'] ?? 'ko');
  let shortcuts = $derived(state?.shortcuts ?? []);
  let permissions = $derived(state?.permissions ?? null);
  let steps = $derived(getOnboardingSteps(platform));
  let currentId = $derived(steps[step] ?? 'welcome');
  let conflictInfo = $derived(formatFirstRunConflict(state?.conflicts, platform));
  let conflicts = $derived(state?.conflicts ?? { count: 0, app: undefined, combos: [] });
  let autostartOn = $derived(Boolean(state?.settings?.['general.autostart']));
  let autostartNotice = $derived(state?.autostartNotice ?? '');
  let canNext = $derived(canAdvanceStep(step, platform, permissions, skippedPermissions));
  let hasMissingPerm = $derived(
    platform === 'macos' &&
      permissions != null &&
      (!permissions.screen || !permissions.accessibility),
  );

  // 맥 권한 카드가 열려 있는 동안 1초마다 상태를 다시 읽는다(R2-ONB-2)
  $effect(() => {
    if (!open || platform !== 'macos') return;
    runAction('perm.refresh');
    const timer = setInterval(() => {
      runAction('perm.refresh');
    }, 1000);
    return () => clearInterval(timer);
  });

  const CORE_ITEMS = [
    { id: 'region', icon: 'region', labelKey: 'onboarding.keys.regionLabel', hintKey: 'onboarding.keys.regionHint', fallbackWin: 'F1', fallbackMac: 'F1' },
    { id: 'draw', icon: 'draw', labelKey: 'onboarding.keys.drawLabel', hintKey: 'onboarding.keys.drawHint', fallbackWin: 'Alt+1', fallbackMac: 'Option+1' },
    { id: 'zoom', icon: 'zoom', labelKey: 'onboarding.keys.zoomLabel', hintKey: 'onboarding.keys.zoomHint', fallbackWin: 'Alt+2', fallbackMac: 'Option+2' },
  ];

  function comboFor(item) {
    const row = shortcuts.find((s) => s.id === item.id);
    if (row) return row.combo;
    return platform === 'macos' ? item.fallbackMac : item.fallbackWin;
  }

  function stepTitleKey(id) {
    if (id === 'welcome') return 'onboarding.welcome.title';
    if (id === 'permScreen') return 'onboarding.permScreen.title';
    if (id === 'permAx') return 'onboarding.permAx.title';
    if (id === 'conflicts') return 'onboarding.conflicts.title';
    if (id === 'autostart') return 'onboarding.autostart.title';
    if (id === 'keys') return 'onboarding.keys.title';
    return 'onboarding.ready.title';
  }

  function goToStep(next) {
    const clamped = Math.max(0, Math.min(ONBOARDING_STEP_COUNT - 1, next));
    if (clamped === step) return;
    step = clamped;
    void client.updateSetting('onboarding.step', clamped);
  }

  function handlePrev() {
    if (step > 0) goToStep(step - 1);
  }

  function handleNext() {
    if (!canNext) return;
    if (step < ONBOARDING_STEP_COUNT - 1) {
      goToStep(step + 1);
    } else {
      handleTryNow();
    }
  }

  function completeAndClose(triggerCapture = false) {
    void client.updateSetting('onboarding.done', true);
    void client.updateSetting('onboarding.step', 0);
    open = false;
    onClose?.();
    if (triggerCapture) {
      void client.run('capture.region');
    }
  }

  function handleTryNow() {
    completeAndClose(true);
  }

  function handleOpenChange(next) {
    open = next;
    if (!next) onClose?.();
  }

  function handleChangeShortcuts() {
    open = false;
    if (onChangeShortcuts) {
      onChangeShortcuts();
    } else {
      client.run('settings.open');
      onClose?.();
    }
  }

  function confirmSkip() {
    skippedPermissions = true;
    skipModalOpen = false;
    if (step < ONBOARDING_STEP_COUNT - 1) {
      goToStep(step + 1);
    }
  }

  function handleWindowKeydown(e) {
    if (!open || skipModalOpen || howtoOpen) return;
    const el = /** @type {HTMLElement | null} */ (e.target);
    if (el?.closest('input, textarea, select')) return;

    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      handlePrev();
    } else if (e.key === 'ArrowRight') {
      if (canNext && step < ONBOARDING_STEP_COUNT - 1) {
        e.preventDefault();
        handleNext();
      }
    } else if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey) {
      if (el?.closest('button, [role="switch"], a')) return;
      if (canNext) {
        e.preventDefault();
        handleNext();
      }
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="ink-first-overlay" />
    <Dialog.Content class="ink-first-dialog" aria-label={t('onboarding.dialogAria')}>
      <div class="ink-first-topbar">
        <span class="ink-first-step-pill ink-mono">
          {t('onboarding.stepLabel', { current: step + 1, total: ONBOARDING_STEP_COUNT })}
        </span>
        {#if currentId === 'welcome'}
          <div class="ink-first-lang" role="group" aria-label={t('onboarding.langLabel')}>
            <button
              type="button"
              class="ink-first-lang-btn ink-focus"
              data-active={lang === 'ko' ? 'true' : undefined}
              onclick={() => client.updateSetting('general.lang', 'ko')}
            >
              한국어
            </button>
            <button
              type="button"
              class="ink-first-lang-btn ink-focus"
              data-active={lang === 'en' ? 'true' : undefined}
              onclick={() => client.updateSetting('general.lang', 'en')}
            >
              English
            </button>
          </div>
        {:else}
          <button
            type="button"
            class="ink-first-x ink-focus"
            aria-label={t('onboarding.nav.close')}
            onclick={() => completeAndClose(false)}
          >
            <Icon name="close" size={14} />
          </button>
        {/if}
      </div>

      {#if currentId === 'welcome'}
        <div class="ink-first-head">
          <AppIcon size={48} label={t('settings.brand')} />
          <div class="ink-first-titles">
            <Dialog.Title class="ink-first-title">{t('onboarding.welcome.title')}</Dialog.Title>
            <Dialog.Description class="ink-first-sub">
              {t('onboarding.welcome.sub')}
            </Dialog.Description>
            <MakerCredit variant="maker" class="ink-first-maker" />
          </div>
        </div>

        <section class="ink-first-section">
          <p class="ink-first-sec-label">{t('onboarding.welcome.pillarsLabel')}</p>
          <div class="ink-first-pillars">
            <div class="ink-first-pillar">
              <span class="ink-first-pillar-icon"><Icon name="region" size={20} /></span>
              <p class="ink-first-pillar-title">{t('onboarding.welcome.captureTitle')}</p>
              <p class="ink-first-pillar-desc">{t('onboarding.welcome.captureDesc')}</p>
            </div>
            <div class="ink-first-pillar">
              <span class="ink-first-pillar-icon"><Icon name="draw" size={20} /></span>
              <p class="ink-first-pillar-title">{t('onboarding.welcome.drawTitle')}</p>
              <p class="ink-first-pillar-desc">{t('onboarding.welcome.drawDesc')}</p>
            </div>
            <div class="ink-first-pillar">
              <span class="ink-first-pillar-icon"><Icon name="zoom" size={20} /></span>
              <p class="ink-first-pillar-title">{t('onboarding.welcome.zoomTitle')}</p>
              <p class="ink-first-pillar-desc">{t('onboarding.welcome.zoomDesc')}</p>
            </div>
          </div>
        </section>

      {:else if currentId === 'permScreen'}
        {@const granted = Boolean(permissions?.screen)}
        {@const revoked = Boolean(permissions?.screenRevoked)}
        <div class="ink-first-card-head">
          <span class="ink-first-card-badge" data-ok={granted ? 'true' : undefined}>
            <Icon name={granted ? 'check-circle' : 'lock'} size={22} />
          </span>
          <div class="ink-first-titles">
            <Dialog.Title class="ink-first-title">{t('onboarding.permScreen.title')}</Dialog.Title>
            <Dialog.Description class="ink-first-sub">{t('onboarding.permScreen.sub')}</Dialog.Description>
          </div>
        </div>

        <div
          role="status"
          class="ink-perm-status"
          data-ok={granted ? 'true' : undefined}
          data-pop={justGrantedScreen ? 'true' : undefined}
        >
          <span class="ink-perm-status-icon">
            <Icon name={granted ? 'check-circle' : 'warning-triangle'} size={18} />
          </span>
          <span class="ink-perm-status-text">
            {#if granted}
              {t('onboarding.permScreen.statusOk')}
            {:else if revoked}
              {t('onboarding.permScreen.statusRevoked')}
            {:else}
              {t('onboarding.permScreen.statusOff')}
            {/if}
          </span>
        </div>

        <ol class="ink-first-steps">
          <li><span class="ink-step-num ink-mono">1</span><span>{t('onboarding.permScreen.step1')}</span></li>
          <li><span class="ink-step-num ink-mono">2</span><span>{t('onboarding.permScreen.step2')}</span></li>
          <li><span class="ink-step-num ink-mono">3</span><span>{t('onboarding.permScreen.step3')}</span></li>
        </ol>

        {#if !granted}
          <div class="ink-perm-actions">
            <button
              type="button"
              class="ink-first-btn ink-first-btn-pri ink-focus"
              onclick={() => {
                screenAsked = true;
                runAction('perm.screen');
              }}
            >
              {t('onboarding.permScreen.openBtn')}
            </button>
            <button
              type="button"
              class="ink-first-btn ink-first-btn-sec ink-focus"
              onclick={() => runAction('app.relaunch')}
            >
              {t('onboarding.permScreen.relaunchBtn')}
            </button>
          </div>
          {#if screenAsked}
            <p class="ink-first-note">{t('onboarding.permScreen.relaunchHint')}</p>
          {/if}
          <button type="button" class="ink-skip-link ink-focus" onclick={() => (skipModalOpen = true)}>
            {t('onboarding.skip.link')}
          </button>
        {/if}

      {:else if currentId === 'permAx'}
        {@const granted = Boolean(permissions?.accessibility)}
        {@const revoked = Boolean(permissions?.accessibilityRevoked)}
        <div class="ink-first-card-head">
          <span class="ink-first-card-badge" data-ok={granted ? 'true' : undefined}>
            <Icon name={granted ? 'check-circle' : 'settings'} size={22} />
          </span>
          <div class="ink-first-titles">
            <Dialog.Title class="ink-first-title">{t('onboarding.permAx.title')}</Dialog.Title>
            <Dialog.Description class="ink-first-sub">{t('onboarding.permAx.sub')}</Dialog.Description>
          </div>
        </div>

        <div
          role="status"
          class="ink-perm-status"
          data-ok={granted ? 'true' : undefined}
          data-pop={justGrantedAx ? 'true' : undefined}
        >
          <span class="ink-perm-status-icon">
            <Icon name={granted ? 'check-circle' : 'warning-triangle'} size={18} />
          </span>
          <span class="ink-perm-status-text">
            {#if granted}
              {t('onboarding.permAx.statusOk')}
            {:else if revoked}
              {t('onboarding.permAx.statusRevoked')}
            {:else}
              {t('onboarding.permAx.statusOff')}
            {/if}
          </span>
        </div>

        <ol class="ink-first-steps">
          <li><span class="ink-step-num ink-mono">1</span><span>{t('onboarding.permAx.step1')}</span></li>
          <li><span class="ink-step-num ink-mono">2</span><span>{t('onboarding.permAx.step2')}</span></li>
          <li><span class="ink-step-num ink-mono">3</span><span>{t('onboarding.permAx.step3')}</span></li>
        </ol>

        {#if !granted}
          <div class="ink-perm-actions">
            <button
              type="button"
              class="ink-first-btn ink-first-btn-pri ink-focus"
              onclick={() => runAction('perm.accessibility')}
            >
              {t('onboarding.permAx.openBtn')}
            </button>
          </div>
          <button type="button" class="ink-skip-link ink-focus" onclick={() => (skipModalOpen = true)}>
            {t('onboarding.skip.link')}
          </button>
        {/if}

      {:else if currentId === 'conflicts'}
        <div class="ink-first-card-head">
          <span class="ink-first-card-badge" data-ok={conflicts.count === 0 ? 'true' : undefined}>
            <Icon name={conflicts.count === 0 ? 'check-circle' : 'warning-triangle'} size={22} />
          </span>
          <div class="ink-first-titles">
            <Dialog.Title class="ink-first-title">{t('onboarding.conflicts.title')}</Dialog.Title>
            <Dialog.Description class="ink-first-sub">{t('onboarding.conflicts.sub')}</Dialog.Description>
          </div>
        </div>

        {#if conflicts.count > 0}
          <div role="status" class="ink-first-notice" data-kind="conflict">
            <span class="ink-first-notice-icon" data-kind="conflict">
              <Icon name="warning-triangle" size={16} />
            </span>
            <div class="ink-first-notice-copy">
              <p class="ink-first-notice-text">
                {conflicts.app
                  ? t('onboarding.conflicts.appTitle', { app: conflicts.app, n: conflicts.count })
                  : t('onboarding.conflicts.genericTitle', { n: conflicts.count })}
              </p>
              {#if (conflicts.combos ?? []).length > 0}
                <p class="ink-first-notice-sub ink-mono">
                  {t('onboarding.conflicts.conflictKeys', { keys: conflicts.combos.slice(0, 4).join(' · ') })}
                </p>
              {/if}
            </div>
            <div class="ink-first-notice-btns">
              {#if conflictInfo?.canHowto || conflicts.app}
                <button
                  type="button"
                  class="ink-first-howto ink-focus"
                  onclick={() => (howtoOpen = true)}
                >
                  {t('onboarding.conflicts.howtoBtn')}
                </button>
              {/if}
              <button
                type="button"
                class="ink-first-howto ink-focus"
                onclick={handleChangeShortcuts}
              >
                {t('onboarding.conflicts.changeBtn')}
              </button>
            </div>
          </div>
        {:else}
          <div role="status" class="ink-perm-status" data-ok="true">
            <span class="ink-perm-status-icon">
              <Icon name="check-circle" size={18} />
            </span>
            <span class="ink-perm-status-text">{t('onboarding.conflicts.okTitle')}</span>
          </div>
          <p class="ink-first-note">{t('onboarding.conflicts.okDesc')}</p>
        {/if}

        <div role="status" class="ink-first-notice" data-kind="info">
          <span class="ink-first-notice-icon" data-kind="info">
            <Icon name="info" size={16} />
          </span>
          <p class="ink-first-notice-text">{t('onboarding.conflicts.fkeyInfo')}</p>
        </div>

      {:else if currentId === 'autostart'}
        <div class="ink-first-card-head">
          <span class="ink-first-card-badge" data-ok={autostartOn ? 'true' : undefined}>
            <Icon name="settings" size={22} />
          </span>
          <div class="ink-first-titles">
            <Dialog.Title class="ink-first-title">{t('onboarding.autostart.title')}</Dialog.Title>
            <Dialog.Description class="ink-first-sub">{t('onboarding.autostart.sub')}</Dialog.Description>
          </div>
        </div>

        <div class="ink-first-group">
          <div class="ink-first-row ink-first-row-tall">
            <div class="ink-first-row-copy">
              <span class="ink-first-row-label">{t('onboarding.autostart.rowLabel')}</span>
              <span class="ink-first-row-sub">{t('onboarding.autostart.rowSub')}</span>
            </div>
            <Switch
              checked={autostartOn}
              label={t('onboarding.autostart.rowLabel')}
              onCheckedChange={(v) => client.updateSetting('general.autostart', v)}
            />
          </div>
        </div>

        {#if autostartNotice}
          <div role="status" class="ink-first-notice" data-kind="conflict">
            <span class="ink-first-notice-icon" data-kind="conflict">
              <Icon name="warning-triangle" size={16} />
            </span>
            <p class="ink-first-notice-text">{autostartNotice}</p>
          </div>
        {/if}
        <p class="ink-first-note">{t('onboarding.autostart.hint')}</p>

      {:else if currentId === 'keys'}
        <div class="ink-first-card-head">
          <span class="ink-first-card-badge">
            <Icon name="shortcuts" size={22} />
          </span>
          <div class="ink-first-titles">
            <Dialog.Title class="ink-first-title">{t('onboarding.keys.title')}</Dialog.Title>
            <Dialog.Description class="ink-first-sub">{t('onboarding.keys.sub')}</Dialog.Description>
          </div>
        </div>

        <div class="ink-first-group">
          {#each CORE_ITEMS as item (item.id)}
            {@const combo = comboFor(item)}
            <div class="ink-first-row">
              <span class="ink-first-icon">
                <Icon name={item.icon} size={18} />
              </span>
              <span class="ink-first-row-label">
                {t(item.labelKey)}
                <small class="ink-first-row-hint">{t(item.hintKey)}</small>
              </span>
              {#if combo}
                <Keycap {combo} {platform} />
              {:else}
                <span class="ink-first-empty">{t('onboarding.keys.empty')}</span>
              {/if}
            </div>
          {/each}
        </div>

        <p class="ink-first-fkey">{t('onboarding.keys.fkeyNote')}</p>
        <div class="ink-first-keyhelp">
          <p class="ink-first-help">
            <Keycap label="?" />
            <span>{t('onboarding.keys.cheatHint')}</span>
          </p>
          <button
            type="button"
            class="ink-first-howto ink-focus"
            onclick={handleChangeShortcuts}
          >
            {t('onboarding.keys.changeBtn')}
          </button>
        </div>

      {:else}
        {@const regionCombo = comboFor(CORE_ITEMS[0])}
        <div class="ink-first-card-head">
          <span class="ink-first-card-badge" data-ok="true">
            <Icon name="check-circle" size={22} />
          </span>
          <div class="ink-first-titles">
            <Dialog.Title class="ink-first-title">{t('onboarding.ready.title')}</Dialog.Title>
            <Dialog.Description class="ink-first-sub">{t('onboarding.ready.sub')}</Dialog.Description>
          </div>
        </div>

        {#if hasMissingPerm}
          <div role="status" class="ink-first-notice" data-kind="conflict">
            <span class="ink-first-notice-icon" data-kind="conflict">
              <Icon name="warning-triangle" size={16} />
            </span>
            <p class="ink-first-notice-text">{t('onboarding.ready.skippedWarn')}</p>
          </div>
        {/if}

        <div class="ink-practice-card">
          <div class="ink-practice-top">
            <span class="ink-practice-badge">{t('onboarding.ready.practiceBadge')}</span>
            {#if regionCombo}
              <Keycap combo={regionCombo} {platform} />
            {/if}
          </div>
          <p class="ink-practice-title">{t('onboarding.ready.practiceTitle')}</p>
          <p class="ink-practice-desc">
            {regionCombo
              ? t('onboarding.ready.practiceDesc', { key: regionCombo })
              : t('onboarding.ready.practiceDescNoKey')}
          </p>
        </div>
      {/if}

      <div class="ink-first-footer">
        <div class="ink-first-dots" role="tablist" aria-label={t('onboarding.dialogAria')}>
          {#each steps as stepId, idx (stepId)}
            {@const jumpAllowed = canJumpToStep(idx, step, platform, permissions, skippedPermissions)}
            <button
              type="button"
              class="ink-first-dot ink-focus"
              data-active={idx === step ? 'true' : undefined}
              data-done={idx < step ? 'true' : undefined}
              disabled={!jumpAllowed}
              aria-label={t('onboarding.dotAria', { n: idx + 1, title: t(stepTitleKey(stepId)) })}
              aria-current={idx === step ? 'step' : undefined}
              onclick={() => goToStep(idx)}
            ></button>
          {/each}
        </div>

        <div class="ink-first-actions">
          {#if step > 0}
            <button
              type="button"
              class="ink-first-btn ink-first-btn-sec ink-focus"
              onclick={handlePrev}
            >
              {t('onboarding.nav.prev')}
            </button>
          {/if}
          {#if step < ONBOARDING_STEP_COUNT - 1}
            <button
              type="button"
              class="ink-first-btn ink-first-btn-pri ink-focus"
              disabled={!canNext}
              onclick={handleNext}
            >
              {t('onboarding.nav.next')}
            </button>
          {:else}
            <button
              type="button"
              class="ink-first-btn ink-first-btn-sec ink-focus"
              onclick={() => completeAndClose(false)}
            >
              {t('onboarding.nav.close')}
            </button>
            <button
              type="button"
              class="ink-first-btn ink-first-btn-pri ink-focus"
              onclick={handleTryNow}
            >
              {t('onboarding.nav.tryNow')}
            </button>
          {/if}
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<Dialog.Root bind:open={skipModalOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="ink-skip-overlay" />
    <Dialog.Content class="ink-skip-dialog" aria-label={t('onboarding.skip.title')}>
      <Dialog.Title class="ink-skip-title">{t('onboarding.skip.title')}</Dialog.Title>
      <Dialog.Description class="ink-skip-desc">{t('onboarding.skip.desc')}</Dialog.Description>
      <div class="ink-skip-footer">
        <Dialog.Close class="ink-first-btn ink-first-btn-sec ink-focus">
          {t('onboarding.skip.back')}
        </Dialog.Close>
        <button type="button" class="ink-first-btn ink-first-btn-pri ink-focus" onclick={confirmSkip}>
          {t('onboarding.skip.confirm')}
        </button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<ConflictHowtoModal
  bind:open={howtoOpen}
  appName={conflicts.app ?? ''}
  onOpenShortcuts={handleChangeShortcuts}
/>

<style>
  :global(.ink-first-overlay) {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: var(--ink-sheet-scrim);
  }

  :global(.ink-first-dialog) {
    position: fixed;
    left: 50%;
    top: 50%;
    z-index: 71;
    transform: translate(-50%, -50%);
    width: min(540px, calc(100vw - 32px));
    box-sizing: border-box;
    padding: 20px 26px 22px;
    border-radius: var(--ink-r-modal);
    background: var(--ink-surface);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-modal);
    font-family: var(--ink-font-sans);
    display: flex;
    flex-direction: column;
    gap: 16px;
    outline: none;
  }

  .ink-first-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .ink-first-step-pill {
    font-size: 11px;
    font-weight: 500;
    color: var(--ink-sub);
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--ink-raised);
  }

  .ink-first-lang {
    display: inline-flex;
    padding: 2px;
    border-radius: var(--ink-r-seg);
    background: var(--ink-raised);
    gap: 2px;
  }

  .ink-first-lang-btn {
    height: 22px;
    padding: 0 9px;
    border: 0;
    border-radius: var(--ink-r-btn);
    background: transparent;
    color: var(--ink-sub);
    font-family: inherit;
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
  }

  .ink-first-lang-btn[data-active='true'] {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
  }

  .ink-first-x {
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: var(--ink-close-r);
    background: var(--ink-close-bg);
    color: var(--ink-sub);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .ink-first-x:hover {
    background: var(--ink-hover);
    color: var(--ink-text);
  }

  .ink-first-head,
  .ink-first-card-head {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .ink-first-card-badge {
    width: 44px;
    height: 44px;
    border-radius: var(--ink-r-group);
    background: var(--ink-raised);
    color: var(--ink-text);
    display: grid;
    place-items: center;
    flex: none;
  }

  .ink-first-card-badge[data-ok='true'] {
    background: color-mix(in srgb, var(--ink-ok) 15%, transparent);
    color: var(--ink-ok);
  }

  .ink-first-titles {
    min-width: 0;
  }

  :global(.ink-first-title) {
    margin: 0;
    font-family: var(--ink-font-display);
    font-size: 20px;
    font-weight: 600;
    line-height: 1.25;
    color: var(--ink-text);
  }

  :global(.ink-first-sub) {
    margin: 4px 0 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--ink-sub);
  }

  :global(.ink-first-maker) {
    margin-top: 6px;
  }

  .ink-first-section {
    display: flex;
    flex-direction: column;
  }

  .ink-first-sec-label {
    margin: 0 2px 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-sub);
  }

  .ink-first-pillars {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .ink-first-pillar {
    padding: 12px;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ink-first-pillar-icon {
    width: 30px;
    height: 30px;
    border-radius: var(--ink-r-row);
    background: var(--ink-raised);
    color: var(--ink-text);
    display: grid;
    place-items: center;
  }

  .ink-first-pillar-title {
    margin: 2px 0 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--ink-text);
  }

  .ink-first-pillar-desc {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--ink-sub);
  }

  @keyframes ink-perm-pop {
    0% {
      transform: scale(0.95);
    }
    60% {
      transform: scale(1.02);
    }
    100% {
      transform: scale(1);
    }
  }

  .ink-perm-status {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: var(--ink-r-group);
    background: var(--ink-attn-bg);
    box-shadow: var(--ink-attn-ring);
    color: var(--ink-attn);
    font-size: 13px;
    font-weight: 600;
  }

  .ink-perm-status[data-ok='true'] {
    background: color-mix(in srgb, var(--ink-ok) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink-ok) 38%, transparent);
    color: var(--ink-ok);
  }

  .ink-perm-status[data-pop='true'] {
    animation: ink-perm-pop 220ms ease-out;
  }

  .ink-perm-status-icon {
    display: inline-flex;
    flex: none;
  }

  .ink-perm-status-text {
    color: var(--ink-text);
  }

  .ink-first-steps {
    margin: 0;
    padding: 12px 14px;
    list-style: none;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ink-first-steps li {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--ink-text-2);
  }

  .ink-step-num {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--ink-raised);
    color: var(--ink-text-2);
    font-size: 10.5px;
    font-weight: 600;
    display: grid;
    place-items: center;
    flex: none;
    margin-top: 1px;
  }

  .ink-perm-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .ink-first-note {
    margin: -4px 2px 0;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--ink-sub);
  }

  .ink-skip-link {
    align-self: flex-start;
    padding: 2px 0;
    border: 0;
    background: transparent;
    color: var(--ink-sub);
    font-family: inherit;
    font-size: 11.5px;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }

  .ink-skip-link:hover {
    color: var(--ink-text);
  }

  .ink-first-group {
    background: var(--ink-group-bg);
    border-radius: var(--ink-r-group);
    box-shadow: var(--ink-group-ring);
    overflow: hidden;
  }

  .ink-first-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 40px;
    padding: 0 14px;
    font-size: 13px;
    color: var(--ink-text);
    border-top: 1px solid var(--ink-hair);
  }

  .ink-first-row:first-child {
    border-top: 0;
  }

  .ink-first-row-tall {
    padding: 10px 14px;
    justify-content: space-between;
  }

  .ink-first-row-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ink-first-row-sub {
    font-size: 11.5px;
    color: var(--ink-sub);
  }

  .ink-first-icon {
    display: inline-flex;
    color: var(--ink-text-2);
    flex: none;
  }

  .ink-first-row-label {
    flex: 1;
    min-width: 0;
  }

  .ink-first-row-hint {
    margin-left: 6px;
    font-size: 12px;
    color: var(--ink-sub);
  }

  .ink-first-fkey {
    margin: -2px 2px 0;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--ink-sub);
  }

  .ink-first-keyhelp {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .ink-first-empty {
    font-size: 11px;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-first-notice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
  }

  .ink-first-notice[data-kind='conflict'] {
    background: var(--ink-attn-bg);
    box-shadow: var(--ink-attn-ring);
  }

  .ink-first-notice-icon {
    display: inline-flex;
    flex: none;
    color: var(--ink-text-2);
  }

  .ink-first-notice-icon[data-kind='conflict'] {
    color: var(--ink-attn);
  }

  .ink-first-notice-copy {
    flex: 1;
    min-width: 0;
  }

  .ink-first-notice-text {
    margin: 0;
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--ink-text);
  }

  .ink-first-notice-sub {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--ink-sub);
  }

  .ink-first-notice-btns {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }

  .ink-first-howto {
    height: 26px;
    padding: 0 11px;
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

  .ink-first-howto:hover {
    background: var(--ink-hover);
  }

  .ink-practice-card {
    padding: 14px 16px;
    border-radius: var(--ink-r-group);
    border: 1.5px dashed var(--ink-line-strong);
    background: var(--ink-raised);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ink-practice-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .ink-practice-badge {
    font-size: 10.5px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--ink-accent-soft);
    color: var(--ink-accent-text);
  }

  .ink-practice-title {
    margin: 2px 0 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--ink-text);
  }

  .ink-practice-desc {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--ink-sub);
  }

  .ink-first-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-top: 2px;
  }

  .ink-first-dots {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .ink-first-dot {
    width: 8px;
    height: 8px;
    padding: 0;
    border: 0;
    border-radius: 999px;
    background: var(--ink-line-strong);
    cursor: pointer;
    transition: width 140ms ease-out, background-color 140ms ease-out;
  }

  .ink-first-dot[data-done='true'] {
    background: var(--ink-text-2);
  }

  .ink-first-dot[data-active='true'] {
    width: 20px;
    background: var(--ink-accent);
  }

  .ink-first-dot:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .ink-first-help {
    margin: 0;
    flex: 1;
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--ink-sub);
  }

  .ink-first-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }

  .ink-first-btn {
    height: 33px;
    border: 0;
    border-radius: var(--ink-r-btn);
    font-family: inherit;
    font-size: 12.5px;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-first-btn-sec {
    padding: 0 15px;
    background: var(--ink-btn-sec-bg);
    box-shadow: var(--ink-btn-sec-shadow);
    color: var(--ink-text);
  }

  .ink-first-btn-sec:hover:not(:disabled) {
    background: var(--ink-hover);
  }

  .ink-first-btn-pri {
    padding: 0 18px;
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    font-weight: 600;
  }

  .ink-first-btn:disabled {
    opacity: 0.42;
    cursor: not-allowed;
  }

  .ink-first-btn:active:not(:disabled) {
    opacity: 0.85;
  }

  :global(.ink-skip-overlay) {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: var(--ink-sheet-scrim);
  }

  :global(.ink-skip-dialog) {
    position: fixed;
    left: 50%;
    top: 50%;
    z-index: 81;
    transform: translate(-50%, -50%);
    width: min(390px, calc(100vw - 32px));
    box-sizing: border-box;
    padding: 20px 22px 18px;
    border-radius: var(--ink-r-window);
    background: var(--ink-surface);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-modal);
    font-family: var(--ink-font-sans);
    outline: none;
  }

  :global(.ink-skip-title) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--ink-text);
  }

  :global(.ink-skip-desc) {
    margin: 8px 0 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--ink-text-2);
  }

  .ink-skip-footer {
    margin-top: 16px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
