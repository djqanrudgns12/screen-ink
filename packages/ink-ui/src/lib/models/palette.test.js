import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  BASIC_COLORS,
  BASIC_COLOR_CODES,
  EXTENDED_COLORS,
  INK_COLORS,
  INK_COLOR_CODES,
  MAX_RECENT_COLORS,
  PALETTE_COLORS,
  colorForKey,
  isHexColor,
  isLightColor,
  markerPreviewColor,
  normalizeColorValue,
  normalizeHex,
  pushRecentColor,
  resolvePaletteHex,
  rgbToHex,
} from './palette.js';

test('기본 8색 키와 HEX가 기존 판서와 동일하게 유지되고 넓힌 8색이 더해진다', () => {
  assert.deepEqual(BASIC_COLOR_CODES, ['R', 'Y', 'G', 'O', 'B', 'L', 'K', 'W']);
  assert.equal(INK_COLORS.R, '#E5484D');
  assert.equal(INK_COLORS.Y, '#F2B705');
  assert.equal(INK_COLORS.G, '#2BA36B');
  assert.equal(INK_COLORS.O, '#F97316');
  assert.equal(INK_COLORS.B, '#2F6FED');
  assert.equal(INK_COLORS.L, '#84CC16');
  assert.equal(INK_COLORS.K, '#1A1A1A');
  assert.equal(INK_COLORS.W, '#FFFFFF');
  assert.equal(BASIC_COLORS.length, 8);
  assert.equal(EXTENDED_COLORS.length, 8);
  assert.equal(PALETTE_COLORS.length, 16);
  assert.equal(INK_COLOR_CODES.length, 16);
});

test('HEX 정규화, RGB 변환, 형광펜 반투명, 키 매핑', () => {
  assert.equal(isHexColor('#ff9f0a'), true);
  assert.equal(isHexColor('#abc'), true);
  assert.equal(isHexColor('red'), false);
  assert.equal(normalizeHex('#abc'), '#AABBCC');
  assert.equal(normalizeHex(' #1e3a8a '), '#1E3A8A');
  assert.equal(normalizeHex('bad'), null);
  assert.equal(rgbToHex([255, 159, 10]), '#FF9F0A');
  assert.equal(rgbToHex([0, 300, -5]), '#00FF00');
  assert.equal(resolvePaletteHex('NV'), '#1E3A8A');
  assert.equal(resolvePaletteHex('#12ab34'), '#12AB34');
  assert.equal(resolvePaletteHex('UNKNOWN'), INK_COLORS.R);
  assert.equal(normalizeColorValue('PU'), 'PU');
  assert.equal(normalizeColorValue('#ff00aa'), '#FF00AA');
  assert.equal(normalizeColorValue('nope'), null);
  assert.equal(markerPreviewColor('R'), 'rgba(229, 72, 77, 0.38)');
  assert.equal(markerPreviewColor('#112233', 0.5), 'rgba(17, 34, 51, 0.5)');
  assert.equal(isLightColor('W'), true);
  assert.equal(isLightColor('K'), false);
  assert.equal(colorForKey('KeyR'), 'R');
  assert.equal(colorForKey('KeyK'), 'K');
  assert.equal(colorForKey('KeyW'), 'W');
  assert.equal(colorForKey('KeyZ'), null);
});

test('최근 색은 중복 없이 최신 순으로 최대 5개만 기억한다', () => {
  let list = [];
  for (const c of ['#111111', '#222222', '#333333', '#444444', '#555555', '#666666']) {
    list = pushRecentColor(list, c);
  }
  assert.equal(list.length, MAX_RECENT_COLORS);
  assert.deepEqual(list, ['#666666', '#555555', '#444444', '#333333', '#222222']);
  // 이미 있던 색을 다시 고르면 맨 앞으로 이동
  list = pushRecentColor(list, '#333333');
  assert.deepEqual(list, ['#333333', '#666666', '#555555', '#444444', '#222222']);
  // 잘못된 값은 무시
  assert.deepEqual(pushRecentColor(list, 'bad'), list);
});
