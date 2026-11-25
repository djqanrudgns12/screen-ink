// 미리보기 진입점 — FakeInkClient로 화면·부품 판을 띄운다. (Antigravity가 채운다)
import { mount } from 'svelte';
import { createFakeClient } from '@screen-ink/client/fake';
import Preview from './Preview.svelte';

const params = new URLSearchParams(location.search);
const client = createFakeClient({ platform: params.get('platform') ?? 'windows' });

mount(Preview, { target: document.getElementById('app'), props: { client } });
