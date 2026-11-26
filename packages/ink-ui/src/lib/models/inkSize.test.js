import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  ANNOT_STYLE_MAX_BYTES,
  SNAP_TOLERANCE,
  TOOL_SIZE_SPECS,
  clampToolSize,
  computePreviewMetrics,
  createDefaultAnnotateStyle,
  formatSizeValue,
  getToolSizeSpec,
  parseAnnotateStyle,
  serializeAnnotateStyle,
  sliderToValue,
  stepSize,
  valueToSlider,
} from './inkSize.js';

test('도구별 크기 범위와 단위가 PRD 규격과 일치한다', () => {
  assert.deepEqual([getToolSizeSpec('pen').min, getToolSizeSpec('pen').max, getToolSizeSpec('pen').unit], [1, 32, 'px']);
  assert.deepEqual([getToolSizeSpec('rect').min, getToolSizeSpec('rect').max], [1, 32]);
  assert.deepEqual([getToolSizeSpec('marker').min, getToolSizeSpec('marker').max, getToolSizeSpec('marker').unit], [6, 48, 'px']);
  assert.deepEqual([getToolSizeSpec('eraser').min, getToolSizeSpec('eraser').max, getToolSizeSpec('eraser').unit], [8, 64, 'px']);
  assert.deepEqual([getToolSizeSpec('mosaic').min, getToolSizeSpec('mosaic').max, getToolSizeSpec('mosaic').unit], [4, 48, 'px']);
  assert.deepEqual([getToolSizeSpec('text').min, getToolSizeSpec('text').max, getToolSizeSpec('text').unit], [12, 120, 'pt']);
  assert.equal(formatSizeValue(6, TOOL_SIZE_SPECS.stroke), '6 px');
  assert.equal(formatSizeValue(24, TOOL_SIZE_SPECS.text), '24 pt');
  assert.equal(clampToolSize(999, TOOL_SIZE_SPECS.stroke), 32);
  assert.equal(clampToolSize(-5, TOOL_SIZE_SPECS.marker), 6);
});

test('로그 슬라이더는 작은 값 구간이 촘촘하고 왕복 변환과 ±8% 자석이 동작한다', () => {
  const spec = TOOL_SIZE_SPECS.stroke; // 1~32, presets: 2, 5, 12
  assert.equal(valueToSlider(1, spec), 0);
  assert.equal(valueToSlider(32, spec), 1);
  // 1..4 구간이 0..0.4 (16..32 구간보다 넓음)
  assert.ok(Math.abs(valueToSlider(4, spec) - 0.4) < 1e-9);
  assert.ok(1 - valueToSlider(16, spec) < valueToSlider(4, spec));

  // 자석 ±8% 안이면 프리셋으로 붙음
  const medPos = valueToSlider(spec.presets.medium, spec);
  const nearMed = sliderToValue(medPos + SNAP_TOLERANCE * 0.6, spec);
  assert.equal(nearMed.snapped, 'medium');
  assert.equal(nearMed.value, spec.presets.medium);

  // 자석 범위 밖이면 일반 로그 값
  const farPos = valueToSlider(25, spec);
  const free = sliderToValue(farPos, spec);
  assert.equal(free.snapped, null);
  assert.equal(free.value, 25);

  // 자석 끄면 정확히 복원
  assert.equal(sliderToValue(valueToSlider(7, spec), spec, { snap: false }).value, 7);
});

test('작은 단계([·])와 큰 단계(⇧+[·]) 이동이 범위 안에서 단조로 동작한다', () => {
  const spec = TOOL_SIZE_SPECS.text; // 12..120
  const s1 = stepSize(24, 1, spec);
  const s2 = stepSize(24, 1, spec, { large: true });
  assert.ok(s1 > 24);
  assert.ok(s2 > s1);
  assert.equal(stepSize(12, -1, spec), 12);
  assert.equal(stepSize(120, 1, spec, { large: true }), 120);
});

test('미리보기는 화면 배율과 확대 배율을 곱하고 칸을 넘으면 비율을 계산한다', () => {
  const small = computePreviewMetrics({ value: 6, displayScale: 1, zoomScale: 1, maxPx: 38 });
  assert.deepEqual(small, { actualPx: 6, drawPx: 6, scalePercent: 100, scaled: false });

  const zoomed = computePreviewMetrics({ value: 6, displayScale: 2, zoomScale: 2, maxPx: 38 });
  assert.deepEqual(zoomed, { actualPx: 24, drawPx: 24, scalePercent: 100, scaled: false });

  const overflow = computePreviewMetrics({ value: 32, displayScale: 2, zoomScale: 1, maxPx: 32 });
  assert.equal(overflow.actualPx, 64);
  assert.equal(overflow.drawPx, 32);
  assert.equal(overflow.scaled, true);
  assert.equal(overflow.scalePercent, 50);
});

test('annotate.style 직렬화·파싱과 손상 데이터 방어', () => {
  const def = createDefaultAnnotateStyle();
  def.tools.pen = { color: 'NV', size: 8 };
  def.tools.text = { color: '#FF9F0A', size: 36 };
  def.recentColors = ['#FF9F0A', '#1E3A8A'];
  def.lastColor = '#FF9F0A';

  const raw = serializeAnnotateStyle(def);
  assert.ok(raw.length <= ANNOT_STYLE_MAX_BYTES);
  const parsed = parseAnnotateStyle(raw);
  assert.deepEqual(parsed.tools.pen, { color: 'NV', size: 8 });
  assert.deepEqual(parsed.tools.text, { color: '#FF9F0A', size: 36 });
  assert.deepEqual(parsed.recentColors, ['#FF9F0A', '#1E3A8A']);
  assert.equal(parsed.lastColor, '#FF9F0A');

  // 손상 JSON·범위 밖 값 복구
  const broken = parseAnnotateStyle('{not json');
  assert.equal(broken.tools.pen.size, 4);
  const clamped = parseAnnotateStyle(JSON.stringify({ tools: { pen: { color: 'bad', size: 999 } }, recentColors: ['bad', '#abc'] }));
  assert.equal(clamped.tools.pen.size, 32);
  assert.equal(clamped.tools.pen.color, 'R');
  assert.deepEqual(clamped.recentColors, ['#AABBCC']);
});
