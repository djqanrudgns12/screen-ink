import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createFakeClient } from '../../../../ink-client/src/fake.js';
import {
  SHORTCUT_PRESETS,
  applyPreset,
  detectActivePreset,
  diffPreset,
  getPresetCombos,
  planPresetApplyOrder,
  previewPresetWithClient,
  undoPresetApply,
} from './presets.js';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../../../..');
const CATALOG = JSON.parse(readFileSync(resolve(ROOT, 'docs/shortcuts/catalog-v1.json'), 'utf8'));
const DEFAULTS = JSON.parse(readFileSync(resolve(ROOT, 'docs/shortcuts/defaults-v1.json'), 'utf8'));

test('추천 묶음 3종(presentation·snipaste·lefthand)의 맥·Windows 키가 카탈로그에 유효하고 중복이 없다', () => {
  assert.deepEqual(
    SHORTCUT_PRESETS.map((p) => p.id),
    ['presentation', 'snipaste', 'lefthand'],
  );

  const validSlots = new Set(CATALOG.slots.map((s) => s.id));
  const macSlots = new Set(CATALOG.slots.filter((s) => !s.windowsOnly).map((s) => s.id));

  for (const preset of SHORTCUT_PRESETS) {
    for (const [platform, platCode] of [
      ['windows', 'win'],
      ['macos', 'mac'],
    ]) {
      const combos = getPresetCombos(preset.id, platform);
      const entries = Object.entries(combos);
      assert.ok(entries.length >= 10);

      const used = new Set();
      for (const [id, combo] of entries) {
        assert.ok(validSlots.has(id), `${preset.id}:${id} 알 수 없는 슬롯`);
        if (platform === 'macos') {
          assert.ok(macSlots.has(id), `${preset.id}:${id} 맥 미지원 슬롯`);
        }
        assert.ok(combo && !used.has(combo), `${preset.id}(${platCode}): 중복 키 ${combo}`);
        used.add(combo);
      }
    }
  }
});

test('발표용 기본 묶음(presentation)은 defaults-v1.json과 정확히 일치한다', () => {
  for (const [platform, platCode] of [
    ['windows', 'win'],
    ['macos', 'mac'],
  ]) {
    const combos = getPresetCombos('presentation', platform);
    for (const [id, combo] of Object.entries(combos)) {
      assert.equal(combo, DEFAULTS.slots[id]?.[platCode], `presentation(${platCode}).${id} 불일치`);
    }
  }
});

test('자리바꿈·순환이 있는 변경 목록도 충돌 없이 적용 순서를 짠다', () => {
  const chainOrder = planPresetApplyOrder([
    { id: 'record', before: 'F3', after: 'F4' },
    { id: 'pin', before: 'Alt+Shift+V', after: 'F3' },
  ]);
  assert.deepEqual(chainOrder, [
    { id: 'record', combo: 'F4' },
    { id: 'pin', combo: 'F3' },
  ]);

  const cycleOrder = planPresetApplyOrder([
    { id: 'region', before: 'F1', after: 'F2' },
    { id: 'window', before: 'F2', after: 'F1' },
  ]);
  assert.equal(cycleOrder[0].combo, '');
  assert.equal(cycleOrder[0].isTempClear, true);
  assert.equal(cycleOrder.at(-1).id, 'region');
  assert.equal(cycleOrder.at(-1).combo, 'F2');
});

test('가짜 클라이언트(Win·Mac)에서 미리보기 → 적용 → 되돌리기 왕복이 원래 값으로 복구된다', async () => {
  for (const platform of ['windows', 'macos']) {
    const client = createFakeClient({ platform, snipasteRunning: false });
    const initial = await client.getState();
    assert.equal(detectActivePreset(initial.shortcuts, platform), 'presentation');
    assert.deepEqual(diffPreset('presentation', initial.shortcuts, platform), []);

    // Snipaste 습관 미리보기 및 적용
    const preview = await previewPresetWithClient(client, 'snipaste', initial.shortcuts, platform);
    assert.ok(preview.changes.length >= 2);
    const pinChange = preview.changes.find((c) => c.id === 'pin');
    assert.equal(pinChange?.after, 'F3');
    assert.equal(pinChange?.status, 'ok', '내부 자리바꿈(record F3→F4)은 겹침으로 오판하지 않아야 함');

    const applied = await applyPreset(client, 'snipaste', initial.shortcuts, platform);
    assert.equal(applied.ok, true);
    assert.ok(applied.undoSteps > 0);

    const afterSnipaste = await client.getState();
    assert.equal(detectActivePreset(afterSnipaste.shortcuts, platform), 'snipaste');

    // 기존 undo() 스택으로 원복
    const undone = await undoPresetApply(client, applied.undoSteps);
    assert.equal(undone, true);
    const restored = await client.getState();
    assert.equal(detectActivePreset(restored.shortcuts, platform), 'presentation');

    // 한 손으로(왼손) 적용 및 원복
    const leftApplied = await applyPreset(client, 'lefthand', restored.shortcuts, platform);
    assert.equal(leftApplied.ok, true);
    const afterLeft = await client.getState();
    assert.equal(detectActivePreset(afterLeft.shortcuts, platform), 'lefthand');

    await undoPresetApply(client, leftApplied.undoSteps);
    const restored2 = await client.getState();
    assert.equal(detectActivePreset(restored2.shortcuts, platform), 'presentation');
  }
});

test('다른 앱과 겹치는 키가 있으면 미리보기에 겹침을 표시한다', async () => {
  const client = createFakeClient({ platform: 'windows', snipasteRunning: true });
  await client.setShortcut('record', 'F4');
  const next = await client.getState();
  const preview = await previewPresetWithClient(client, 'snipaste', next.shortcuts, 'windows');
  const pinChange = preview.changes.find((c) => c.id === 'pin');
  assert.equal(pinChange?.status, 'conflict');
  assert.equal(pinChange?.owner, 'Snipaste');
  assert.ok(preview.conflictCount >= 1);
});

test('되돌리기 요청이 성공해도 원래 키가 남지 않으면 복원 실패로 알린다', async () => {
  const previous = [{ id: 'region', combo: 'F1' }];
  const client = createFakeClient({ platform: 'windows' });
  const state = await client.getState();
  const undo = async () => ({ ok: true, state });
  assert.equal(await undoPresetApply({ ...client, undo }, 1, previous), true);
  state.shortcuts = state.shortcuts.map((row) => row.id === 'region' ? { ...row, combo: 'Alt+Q' } : row);
  assert.equal(await undoPresetApply({ ...client, undo }, 1, previous), false);
});
