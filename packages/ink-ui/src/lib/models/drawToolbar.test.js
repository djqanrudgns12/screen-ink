import test from 'node:test';
import assert from 'node:assert/strict';
import { computeTopIslandRect, rectsOverlap } from './popoverPlacement.js';
import {
  DRAW_TOOLBAR_DIM_OPACITY,
  DRAW_TOOLBAR_HISTORY,
  DRAW_TOOLBAR_IDLE_MS,
  DRAW_TOOLBAR_PROXIMITY_PX,
  DRAW_TOOLBAR_TOOLS,
  activeDrawToolId,
  buildDrawIslandPatch,
  clampDrawSettingWidth,
  drawIslandColor,
  drawToolSizeToEngineWidth,
  engineWidthToDrawToolSize,
  formatDrawTooltip,
  isNearRect,
  panelToolIdForDraw,
  placeDrawColorPanel,
  resolveBoardToggleAction,
  resolveDrawToolbarVisibility,
} from './drawToolbar.js';

test('도구 막대 묶음 구성과 활성 도구 매핑', () => {
  assert.deepEqual(
    DRAW_TOOLBAR_TOOLS.map((t) => t.id),
    ['pen', 'marker', 'eraser', 'line', 'rect', 'ellipse', 'text'],
  );
  assert.deepEqual(
    DRAW_TOOLBAR_HISTORY.map((h) => h.id),
    ['undo', 'redo'],
  );
  assert.equal(activeDrawToolId('pen', null), 'pen');
  assert.equal(activeDrawToolId('pen', 'rect'), 'rect');
  assert.equal(activeDrawToolId('pen', 'ellipse'), 'ellipse');
  assert.equal(activeDrawToolId('eraser', 'rect'), 'eraser');
  assert.equal(panelToolIdForDraw('line'), 'pen');
  assert.equal(panelToolIdForDraw('rect'), 'rect');
  assert.equal(panelToolIdForDraw('select'), 'pen');
});

test('패널 크기 ↔ 엔진 굵기 변환과 판서 설정 1~30 범위 제한', () => {
  assert.equal(drawToolSizeToEngineWidth('pen', 12), 12);
  assert.equal(drawToolSizeToEngineWidth('marker', 40), 40);
  assert.equal(drawToolSizeToEngineWidth('eraser', 24), 8);
  assert.equal(drawToolSizeToEngineWidth('text', 24), 4);
  assert.equal(engineWidthToDrawToolSize('eraser', 10), 30);
  assert.equal(engineWidthToDrawToolSize('text', 4), 24);

  // R2-ANN-5: 판서 설정 draw.width는 1~30으로 가두고, 기본 8색 밖은 섬/설정에 넘기지 않는다
  assert.equal(clampDrawSettingWidth(45), 30);
  assert.equal(clampDrawSettingWidth(0), 1);
  assert.equal(clampDrawSettingWidth(12.6), 13);
  assert.equal(drawIslandColor('R'), 'R');
  assert.equal(drawIslandColor('W'), 'W');
  assert.equal(drawIslandColor('NV'), undefined);
  assert.equal(drawIslandColor('#123456'), undefined);
});

test('칠판 ↔ 화면 전환 동작 계산', () => {
  assert.deepEqual(resolveBoardToggleAction({ mode: 'draw', background: 'frame' }), {
    nextBackground: 'white',
    labelKey: 'draw.action.toBoard',
    icon: 'board',
    slot: 'd-white',
  });
  assert.deepEqual(resolveBoardToggleAction({ mode: 'draw', background: 'white' }), {
    nextBackground: 'frame',
    labelKey: 'draw.action.toScreen',
    icon: 'monitor',
    slot: 'd-white',
  });
  assert.deepEqual(resolveBoardToggleAction({ mode: 'board', background: 'white' }), {
    nextBackground: 'black',
    labelKey: 'draw.action.toBlackboard',
    icon: 'dark',
    slot: 'd-black',
  });
  assert.deepEqual(resolveBoardToggleAction({ mode: 'board', background: 'black' }), {
    nextBackground: 'white',
    labelKey: 'draw.action.toWhiteboard',
    icon: 'board',
    slot: 'd-white',
  });
});

test('말풍선 문구는 OS별 키 표기를 따른다', () => {
  assert.equal(formatDrawTooltip({ label: '펜', slot: 'd-pen' }, {}, 'windows'), '펜 · P');
  assert.equal(formatDrawTooltip({ label: '되돌리기', slot: 'd-undo' }, {}, 'windows'), '되돌리기 · Ctrl Z');
  assert.equal(formatDrawTooltip({ label: '되돌리기', slot: 'd-undo' }, {}, 'macos'), '되돌리기 · ⌘Z');
  assert.equal(formatDrawTooltip({ label: '다시 하기', slot: 'd-redo' }, {}, 'macos'), '다시 하기 · ⇧⌘Z');
  assert.equal(formatDrawTooltip({ label: '사각형', modKey: 'primary' }, {}, 'macos'), '사각형 · ⌘');
  assert.equal(formatDrawTooltip({ label: '사각형', modKey: 'primary' }, {}, 'windows'), '사각형 · Ctrl');
  assert.equal(formatDrawTooltip({ label: '직선', modKey: 'Shift' }, {}, 'macos'), '직선 · ⇧');
  assert.equal(formatDrawTooltip({ label: '원', modKey: 'Tab' }, {}, 'windows'), '원 · Tab');
  assert.equal(formatDrawTooltip({ label: '펜', slot: 'd-pen' }, { 'd-pen': '' }, 'macos'), '펜');
});

test('3초 무동작 흐려짐, 그리는 중 통과, 손 근접 복귀', () => {
  assert.equal(DRAW_TOOLBAR_IDLE_MS, 3000);
  assert.equal(DRAW_TOOLBAR_DIM_OPACITY, 0.28);
  assert.equal(DRAW_TOOLBAR_PROXIMITY_PX, 72);

  assert.deepEqual(resolveDrawToolbarVisibility({ hidden: true }), {
    visible: false,
    dimmed: false,
    interactive: false,
  });
  assert.deepEqual(resolveDrawToolbarVisibility({ drawing: true }), {
    visible: true,
    dimmed: true,
    interactive: false,
  });
  assert.deepEqual(resolveDrawToolbarVisibility({ idleDim: true }), {
    visible: true,
    dimmed: true,
    interactive: true,
  });
  assert.deepEqual(resolveDrawToolbarVisibility({ idleDim: true, nearToolbar: true }), {
    visible: true,
    dimmed: false,
    interactive: true,
  });
  assert.deepEqual(resolveDrawToolbarVisibility({ idleDim: true, panelOpen: true }), {
    visible: true,
    dimmed: false,
    interactive: true,
  });

  const rect = { left: 400, top: 650, right: 880, bottom: 690 };
  assert.equal(isNearRect({ x: 350, y: 600 }, rect), true);
  assert.equal(isNearRect({ x: 200, y: 200 }, rect), false);
});

test('색·굵기 패널은 도구 막대 위 가운데에 붙고 화면 안으로 가둔다', () => {
  const pos = placeDrawColorPanel(
    { left: 400, top: 660, width: 480, height: 42 },
    { w: 284, h: 216 },
    { w: 1280, h: 720 },
    610,
  );
  assert.equal(pos.side, 'above');
  assert.equal(pos.x, 610 - 142);
  assert.equal(pos.y, 660 - 10 - 216);
  assert.equal(pos.arrowX, 142);

  const clamped = placeDrawColorPanel(
    { left: 8, top: 660, width: 320, height: 42 },
    { w: 284, h: 216 },
    { w: 340, h: 720 },
    20,
  );
  assert.equal(clamped.x, 8);
  assert.ok(clamped.arrowX >= 18);

  // G19: 위쪽 배치 시 상단 섬(560×88)과 겹치면 아래로 뒤집는다
  const vp = { w: 1280, h: 820 };
  const island = computeTopIslandRect(vp);
  const flipped = placeDrawColorPanel(
    { left: 400, top: 280, width: 480, height: 42 },
    { w: 284, h: 216 },
    vp,
    640,
  );
  assert.equal(flipped.side, 'below');
  assert.equal(flipped.y, 280 + 42 + 10);
  assert.equal(rectsOverlap({ x: flipped.x, y: flipped.y, w: 284, h: 216 }, island), false);
});

test('칠판 쪽·배경·색·굵기 섬 패치를 정규화해 만든다', () => {
  assert.deepEqual(
    buildDrawIslandPatch(
      7,
      {
        color: 'B',
        tool: 'pen',
        widthPx: 42,
        zoom: 1,
        background: 'black',
        pageIndex: 1,
        pageCount: 3,
        suggestion: null,
      },
      { idleDim: true, avoid: false },
    ),
    {
      session: 7,
      color: 'B',
      tool: 'pen',
      widthPx: 30,
      zoom: 1,
      background: 'black',
      pageIndex: 1,
      pageCount: 3,
      suggestion: null,
      idleDim: true,
      avoid: false,
    },
  );
  const custom = buildDrawIslandPatch(9, { color: '#123456', tool: 'marker', widthPx: 8, background: 'white', pageIndex: 0, pageCount: 1 });
  assert.equal('color' in custom, false, '기본 8색 밖 HEX는 섬 패치에서 생략');
});
