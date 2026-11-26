// 미리보기 진입점 — FakeInkClient로 화면·부품 판·캡처 오버레이를 띄운다.
import { mount } from 'svelte';
import { createFakeClient } from '@screen-ink/client/fake';
import sampleFrameUrl from '../../../../docs/mockups/v3/png/F04-keys.png';
import Preview from './Preview.svelte';
import DrawPreview from './DrawPreview.svelte';

const params = new URLSearchParams(location.search);
const platform = params.get('platform') === 'macos' ? 'macos' : 'windows';
const lang = params.get('lang') === 'en' ? 'en' : 'ko';
const perm = params.get('perm');

function resolvePreviewPermissions(p) {
  if (!p) return undefined;
  if (p === 'all-off') return { screen: false, accessibility: false, screenRevoked: false, accessibilityRevoked: false };
  if (p === 'all-on') return { screen: true, accessibility: true, screenRevoked: false, accessibilityRevoked: false };
  if (p === 'ax-revoked') return { screen: true, accessibility: false, screenRevoked: false, accessibilityRevoked: true };
  return {
    screen: !p.startsWith('screen'),
    accessibility: p !== 'ax-off',
    screenRevoked: p === 'screen-revoked',
    accessibilityRevoked: false,
  };
}

const client = createFakeClient({
  platform,
  lang,
  snipasteRunning: params.get('conflict') !== '0',
  screenNeedsRelaunch: params.get('relaunch') === '1',
  frameUrl: sampleFrameUrl,
  permissions: resolvePreviewPermissions(perm),
});

const stepParam = params.get('step');
if (stepParam !== null && stepParam !== '') {
  const stepNum = Number(stepParam);
  if (Number.isInteger(stepNum) && stepNum >= 0 && stepNum <= 4) {
    client.updateSetting('onboarding.step', stepNum);
  }
}

// ?evidence=1 — 숨은 "인식 근거 보기"(capture.evidence)를 켠 채로
if (params.get('evidence') === '1') client.updateSetting('capture.evidence', true);

// ?draw=1 — 판서 캔버스(DrawCanvas) 데모. 그 밖에는 기존 미리보기 그대로.
const target = /** @type {HTMLElement} */ (document.getElementById('app'));
if (params.get('draw') === '1') mount(DrawPreview, { target, props: { client } });
else mount(Preview, { target, props: { client } });
