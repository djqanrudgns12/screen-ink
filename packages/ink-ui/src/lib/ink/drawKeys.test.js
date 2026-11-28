import test from 'node:test';
import assert from 'node:assert/strict';
import {
  DRAW_KEY_IDS,
  DRAW_TOOLBAR_TOGGLE_CODE,
  DRAW_TOOLBAR_TOGGLE_KEY,
  IME_GUARD_MS,
  LATER_DRAW_KEYS,
  buildDrawKeymap,
  createImeGuard,
  defaultDrawCombo,
  isComposingEvent,
  isDrawToolbarToggleKey,
  matchBoardPageKey,
  matchDrawKey,
  matchDrawSizeBracketKey,
  normalizeCombo,
} from './drawKeys.js';
import catalog from '../../../../../docs/shortcuts/catalog-v1.json' with { type: 'json' };

const KEY_CODES = /** @type {Record<string, string>} */ ({
  Esc: 'Escape',
  Space: 'Space',
  Up: 'ArrowUp',
  Down: 'ArrowDown',
  '/': 'Slash',
});

/**
 * 조합 문자열 → KeyboardEvent 모양(시험용).
 * @param {string} combo
 * @param {'windows'|'macos'} platform
 */
function eventOf(combo, platform = 'windows') {
  const parts = combo.split('+');
  const key = parts.pop() ?? '';
  const code = KEY_CODES[key] ?? (/^[A-Z]$/.test(key) ? `Key${key}` : key);
  return {
    code,
    key,
    ctrlKey: parts.includes('Ctrl'),
    altKey: parts.includes('Alt') || parts.includes('Option'),
    shiftKey: parts.includes('Shift'),
    metaKey: parts.includes(platform === 'macos' ? 'Cmd' : 'Win'),
  };
}

test('판서_화면안키_설정값_반영_및_맥_Cmd', () => {
  const rows = [{ id: 'd-pen', combo: 'A' }];
  const map = buildDrawKeymap(rows);
  assert.equal(matchDrawKey({ code: 'KeyA', key: 'a' }, map), 'd-pen');
  assert.equal(matchDrawKey({ code: 'KeyP', key: 'p' }, map), null, '옛 키 P는 무시');
  assert.equal(matchDrawKey({ code: 'KeyM', key: 'm' }, map), 'd-marker', '표에 없는 칸은 기본값');

  const mac = buildDrawKeymap([], 'macos');
  assert.equal(matchDrawKey({ code: 'KeyZ', metaKey: true }, mac, 'macos'), 'd-undo');
  assert.equal(matchDrawKey({ code: 'KeyZ', ctrlKey: true }, mac, 'macos'), null, '맥에서 Ctrl+Z는 실행 취소 아님');
  assert.equal(matchDrawKey({ code: 'KeyZ', metaKey: true, shiftKey: true }, mac, 'macos'), 'd-redo');
  const win = buildDrawKeymap([]);
  assert.equal(matchDrawKey({ code: 'KeyZ', ctrlKey: true }, win), 'd-undo');
  assert.equal(matchDrawKey({ code: 'KeyY', ctrlKey: true }, win), 'd-redo');
});

test('확정본_29칸_기본값이_모두_판정된다', () => {
  assert.equal(DRAW_KEY_IDS.length, 29);
  const drawSlots = catalog.slots.filter((s) => s.scope === 'draw').map((s) => s.id);
  assert.deepEqual([...DRAW_KEY_IDS].sort(), [...drawSlots].sort(), 'catalog-v1 scope "draw"와 같은 칸');
  for (const platform of /** @type {const} */ (['windows', 'macos'])) {
    const map = buildDrawKeymap(null, platform);
    assert.equal(map.size, 29, `${platform} 29칸 모두 서로 다른 키`);
    for (const id of DRAW_KEY_IDS) {
      const combo = defaultDrawCombo(id, platform);
      assert.ok(combo, `${id} 기본값`);
      assert.equal(matchDrawKey(eventOf(combo, platform), map, platform), id, `${platform} ${combo} → ${id}`);
    }
  }
  assert.deepEqual([...LATER_DRAW_KEYS], ['d-spot', 'd-check', 'd-number']);
});

test('비운칸은_동작하지_않는다', () => {
  const map = buildDrawKeymap([{ id: 'd-clear', combo: '' }]);
  assert.equal(matchDrawKey({ code: 'KeyE' }, map), null);
  assert.equal(map.size, 28);
});

test('한글_자판에서도_물리키로_판정한다', () => {
  const map = buildDrawKeymap(null);
  assert.equal(matchDrawKey({ code: 'KeyP', key: 'ㅔ' }, map), 'd-pen');
  assert.equal(matchDrawKey({ code: 'Slash', key: '?', shiftKey: true }, map), 'd-help');
  assert.equal(matchDrawKey({ code: 'KeyP', key: 'p', shiftKey: true }, map), null, 'Shift+P는 펜 아님(Shift 직선 드래그와 겹치지 않게)');
});

test('IME_조합중과_끝난뒤_40ms는_막는다', () => {
  const map = buildDrawKeymap(null);
  assert.equal(isComposingEvent({ isComposing: true }), true);
  assert.equal(isComposingEvent({ keyCode: 229 }), true);
  assert.equal(matchDrawKey({ code: 'KeyP', isComposing: true }, map), null);
  assert.equal(matchDrawKey({ code: 'KeyP', keyCode: 229 }, map), null);

  const guard = createImeGuard();
  assert.equal(guard.blocks({ code: 'KeyP' }, 1000), false);
  guard.start();
  assert.equal(guard.composing, true);
  assert.equal(guard.blocks({ code: 'KeyP' }, 1001), true);
  guard.end(1100);
  assert.equal(guard.blocks({ code: 'KeyP' }, 1100 + IME_GUARD_MS - 1), true, '끝난 직후 새는 키');
  assert.equal(guard.blocks({ code: 'KeyP' }, 1100 + IME_GUARD_MS), false);
});

test('조합_문자열_정규화_순서와_별칭', () => {
  assert.equal(normalizeCombo('Shift+Ctrl+r'), 'Ctrl+Shift+R');
  assert.equal(normalizeCombo('Cmd+Shift+Z', 'macos'), 'Shift+Cmd+Z');
  assert.equal(normalizeCombo('Alt+Shift+1', 'macos'), 'Option+Shift+1');
  assert.equal(normalizeCombo('Option+Up', 'windows'), 'Alt+Up');
  assert.equal(normalizeCombo(''), '');
  assert.equal(normalizeCombo('Ctrl+'), '');
  assert.equal(normalizeCombo(null), '');
});

test('도구막대_숨기기_V키와_크기조절_대괄호키_사용자키_우선', () => {
  assert.equal(DRAW_TOOLBAR_TOGGLE_CODE, 'KeyV');
  assert.equal(DRAW_TOOLBAR_TOGGLE_KEY, 'V');

  const defaultMap = buildDrawKeymap(null);
  assert.equal(isDrawToolbarToggleKey({ code: 'KeyV' }, defaultMap), true);
  assert.equal(isDrawToolbarToggleKey({ code: 'KeyV', shiftKey: true }, defaultMap), false);
  assert.equal(isDrawToolbarToggleKey({ code: 'KeyV', ctrlKey: true }, defaultMap), false);
  assert.equal(isDrawToolbarToggleKey({ code: 'KeyV', isComposing: true }, defaultMap), false);

  const customMap = buildDrawKeymap([{ id: 'd-pen', combo: 'V' }]);
  assert.equal(isDrawToolbarToggleKey({ code: 'KeyV' }, customMap), false, '사용자가 V를 묶었으면 양보');

  assert.deepEqual(matchDrawSizeBracketKey({ code: 'BracketLeft' }, defaultMap), { dir: -1, large: false });
  assert.deepEqual(matchDrawSizeBracketKey({ code: 'BracketRight', shiftKey: true }, defaultMap), { dir: 1, large: true });
  assert.equal(matchDrawSizeBracketKey({ code: 'BracketLeft', ctrlKey: true }, defaultMap), null);
  assert.equal(matchDrawSizeBracketKey({ code: 'KeyP' }, defaultMap), null);
});

test('칠판_쪽넘기기_PgDn_PgUp_판정과_수식키_IME_사용자키_양보', () => {
  const defaultMap = buildDrawKeymap(null);
  assert.equal(matchBoardPageKey({ code: 'PageDown' }, defaultMap), 'next');
  assert.equal(matchBoardPageKey({ code: 'PageUp' }, defaultMap), 'prev');
  assert.equal(matchBoardPageKey({ code: '', key: 'PageDown' }, defaultMap), 'next');
  assert.equal(matchBoardPageKey({ code: 'PageDown', shiftKey: true }, defaultMap), null);
  assert.equal(matchBoardPageKey({ code: 'PageDown', ctrlKey: true }, defaultMap), null);
  assert.equal(matchBoardPageKey({ code: 'PageDown', isComposing: true }, defaultMap), null);
  assert.equal(matchBoardPageKey({ code: 'PageUp', keyCode: 229 }, defaultMap), null);

  const customMap = buildDrawKeymap([{ id: 'd-clear', combo: 'PgDn' }]);
  assert.equal(matchBoardPageKey({ code: 'PageDown' }, customMap), null, '사용자 단축키가 묶였으면 양보');
});
