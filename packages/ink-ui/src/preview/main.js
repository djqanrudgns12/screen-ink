// 미리보기 진입점 — FakeInkClient로 화면·부품 판·캡처 오버레이를 띄운다.
import { mount } from 'svelte';
import { createFakeClient } from '@screen-ink/client/fake';
import sampleFrameUrl from '../../../../docs/mockups/v3/png/F04-keys.png';
import Preview from './Preview.svelte';

const params = new URLSearchParams(location.search);
const client = createFakeClient({
  platform: params.get('platform') ?? 'windows',
  frameUrl: sampleFrameUrl,
});

// ?evidence=1 — 숨은 "인식 근거 보기"(capture.evidence)를 켠 채로
if (params.get('evidence') === '1') client.updateSetting('capture.evidence', true);

mount(Preview, { target: document.getElementById('app'), props: { client } });
