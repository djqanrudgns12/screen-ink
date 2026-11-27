import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  POPOVER_EDGE,
  TOP_ISLAND_HEIGHT,
  TOP_ISLAND_TOP_OFFSET,
  TOP_ISLAND_WIDTH,
  computeTopIslandRect,
  normalizeRect,
  placeAnchoredPopover,
  rectsOverlap,
} from './popoverPlacement.js';

test('상단 섬(560×88) 회피 영역은 뷰포트·작업 영역·혼합 배율·다중 모니터 좌표를 반영한다', () => {
  const std = computeTopIslandRect({ w: 1280, h: 820 });
  assert.deepEqual(std, {
    left: 360,
    top: TOP_ISLAND_TOP_OFFSET,
    right: 920,
    bottom: TOP_ISLAND_TOP_OFFSET + TOP_ISLAND_HEIGHT,
    width: TOP_ISLAND_WIDTH,
    height: TOP_ISLAND_HEIGHT,
  });

  // 좁은 화면에서는 뷰포트 폭으로 제한
  const narrow = computeTopIslandRect({ w: 480, h: 360 });
  assert.deepEqual(narrow, { left: 0, top: 12, right: 480, bottom: 100, width: 480, height: 88 });

  // macOS Retina(2x) 메뉴 막대(물리 74px = CSS 37px) 반영
  const mac = computeTopIslandRect(
    { w: 1512, h: 982 },
    {
      bounds: { x: 0, y: 0, w: 3024, h: 1964 },
      work: { x: 0, y: 74, w: 3024, h: 1890 },
      scale: 2,
    },
  );
  assert.deepEqual(mac, { left: 476, top: 49, right: 1036, bottom: 137, width: 560, height: 88 });

  // Windows 오른쪽 보조 모니터(1.5x, 왼쪽 작업 표시줄 90 물리px = 60 CSS px)
  const winSecond = computeTopIslandRect(
    { w: 1920, h: 1200 },
    {
      bounds: { x: 1920, y: 0, w: 2880, h: 1800 },
      work: { x: 2010, y: 0, w: 2790, h: 1740 },
      scale: 1.5,
    },
  );
  assert.equal(winSecond?.left, 60 + (1860 - 560) / 2);
  assert.equal(winSecond?.top, 12);
  assert.equal(winSecond?.bottom, 100);

  // 음수 좌표 왼쪽 모니터
  const leftMon = computeTopIslandRect({ left: -1280, top: 0, w: 1280, h: 720 });
  assert.deepEqual(leftMon, { left: -920, top: 12, right: -360, bottom: 100, width: 560, height: 88 });

  assert.equal(computeTopIslandRect({ w: 0, h: 800 }), null);
  assert.equal(normalizeRect({ left: 10, top: 20, width: 0, height: 10 }), null);
  assert.equal(rectsOverlap({ left: 0, top: 0, right: 100, bottom: 100 }, { left: 100, top: 0, right: 200, bottom: 100 }), false);
  assert.equal(rectsOverlap({ left: 0, top: 0, right: 100, bottom: 100 }, { left: 100, top: 0, right: 200, bottom: 100 }, 1), true);
});

test('위 공간이 충분하고 섬과 안 겹치면 위에 열고, 섬과 겹치거나 위가 모자라면 아래로 뒤집는다', () => {
  const viewport = { w: 1280, h: 820 };
  const island = computeTopIslandRect(viewport);
  assert.ok(island);

  // 1) 아래쪽 툴바/트리거 → 위에 충분히 들어오고 섬과 겹치지 않음
  const above = placeAnchoredPopover(
    { left: 467, top: 580, width: 260, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    viewport,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [island] },
  );
  assert.equal(above.side, 'above');
  assert.equal(above.clamped, false);
  assert.equal(above.maxHeight, 320);
  assert.equal(above.top, 580 - 6 - 320);
  assert.equal(rectsOverlap(above, island), false);

  // 2) G18 실측 재현: 화면 위쪽 범위(트리거 top=348) — 위 340px는 화면엔 들어가지만 상단 섬(12..100)과 겹침 → 아래로 뒤집기
  const flippedByIsland = placeAnchoredPopover(
    { left: 467, top: 348, width: 260, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    viewport,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [island] },
  );
  assert.equal(flippedByIsland.side, 'below');
  assert.equal(flippedByIsland.clamped, false);
  assert.equal(flippedByIsland.maxHeight, 320);
  assert.equal(flippedByIsland.top, 348 + 30 + 6);
  assert.equal(rectsOverlap(flippedByIsland, island), false);
  assert.ok(flippedByIsland.bottom <= viewport.h - POPOVER_EDGE);

  // 3) 위 공간 자체가 부족(top=40) → 아래로 뒤집기
  const flippedByTopEdge = placeAnchoredPopover(
    { left: 40, top: 40, width: 260, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    viewport,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [island] },
  );
  assert.equal(flippedByTopEdge.side, 'below');
  assert.equal(flippedByTopEdge.top, 40 + 30 + 6);
  assert.equal(rectsOverlap(flippedByTopEdge, island), false);

  // 4) 넓은 화면(1920×1080) 왼쪽 끝(상단 섬 680..1240 바깥) → 위 공간이 충분하면 불필요하게 뒤집지 않음
  const wide = { w: 1920, h: 1080 };
  const wideIsland = computeTopIslandRect(wide);
  const farLeft = placeAnchoredPopover(
    { left: 20, top: 340, width: 260, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    wide,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [wideIsland] },
  );
  assert.equal(farLeft.side, 'above');
  assert.equal(farLeft.top, 340 - 6 - 320);
  assert.equal(rectsOverlap(farLeft, wideIsland), false);
});

test('양쪽 공간이 모두 모자라면 화면 안쪽·섬 바깥으로 밀고, 필요 시 높이를 압축한다', () => {
  const shortViewport = { w: 1280, h: 420 };
  const island = computeTopIslandRect(shortViewport);
  assert.ok(island);

  // 1) 접을 수 있는 글꼴 팝오버(height 320, minHeight 180): 위 clear 124px, 아래 clear 146px → 아래 선택 + 섬 아래·화면 안 클램프
  const bothTightCollapsible = placeAnchoredPopover(
    { left: 500, top: 230, width: 200, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    shortViewport,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [island] },
  );
  assert.equal(bothTightCollapsible.side, 'below');
  assert.equal(bothTightCollapsible.clamped, true);
  assert.equal(bothTightCollapsible.maxHeight, 180);
  assert.ok(bothTightCollapsible.top >= island.bottom, '상단 섬 아래로 밀어 겹침 0');
  assert.ok(bothTightCollapsible.bottom <= shortViewport.h - POPOVER_EDGE, '화면 아래를 넘지 않음');
  assert.equal(rectsOverlap(bothTightCollapsible, island), false);

  // 2) 고정 높이 색·굵기 패널(height 268): 위 clear 92px, 아래 clear 164px → 아래 선택 + 섬 아래·화면 안 클램프
  const bothTightFixed = placeAnchoredPopover(
    { left: 377, top: 200, width: 540, height: 40 },
    { width: 284, height: 268, minHeight: 268 },
    shortViewport,
    { gap: 8, edge: POPOVER_EDGE, align: 'center', targetCenterX: 600, avoidRects: [island] },
  );
  assert.equal(bothTightFixed.side, 'below');
  assert.equal(bothTightFixed.clamped, true);
  assert.ok(bothTightFixed.top >= island.bottom, '상단 섬 아래 유지');
  assert.ok(bothTightFixed.bottom <= shortViewport.h - POPOVER_EDGE, '화면 아래 경계 안');
  assert.equal(rectsOverlap(bothTightFixed, island), false);

  // 3) 극단적으로 낮은 화면(240px): 섬 아래 공간도 minHeight보다 작으면 화면 전체 안쪽으로 압축·클램프
  const tinyViewport = { w: 480, h: 240 };
  const tinyIsland = computeTopIslandRect(tinyViewport);
  const tinyPlaced = placeAnchoredPopover(
    { left: 100, top: 110, width: 160, height: 28 },
    { width: 264, height: 320, minHeight: 180 },
    tinyViewport,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [tinyIsland] },
  );
  assert.equal(tinyPlaced.clamped, true);
  assert.ok(tinyPlaced.top >= POPOVER_EDGE);
  assert.ok(tinyPlaced.bottom <= tinyViewport.h - POPOVER_EDGE);
  assert.ok(tinyPlaced.left >= POPOVER_EDGE);
  assert.ok(tinyPlaced.right <= tinyViewport.w - POPOVER_EDGE);

  // 4) 기준점이 섬 내부(top=16..56)에 있어 아래 시작점(64)이 섬(12..100)을 걸치면 섬 아래(100)로 밀기
  const insideIsland = placeAnchoredPopover(
    { left: 500, top: 16, width: 200, height: 40 },
    { width: 264, height: 220, minHeight: 220 },
    { w: 1280, h: 820 },
    { gap: 8, edge: POPOVER_EDGE, avoidRects: [island] },
  );
  assert.equal(insideIsland.side, 'below');
  assert.equal(insideIsland.clamped, true);
  assert.equal(insideIsland.top, island.bottom);
  assert.equal(rectsOverlap(insideIsland, island), false);
});

test('두 모니터 경계(오른쪽·왼쪽·음수 좌표·혼합 배율)에서 모니터 밖으로 새지 않고 섬을 피한다', () => {
  // 1) 주 모니터 [0..1920]와 오른쪽 모니터 [1920..3840] 경계
  const primaryBounds = { left: 0, top: 0, w: 1920, h: 1080 };
  const primaryIsland = computeTopIslandRect(primaryBounds);
  const nearRightSeam = placeAnchoredPopover(
    { left: 1850, top: 500, width: 120, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    primaryBounds,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [primaryIsland] },
  );
  assert.ok(nearRightSeam.right <= primaryBounds.w - POPOVER_EDGE, '주 모니터 오른쪽 경계 안');
  assert.ok(nearRightSeam.left >= POPOVER_EDGE);

  const secondaryBounds = { left: 1920, top: 0, w: 1920, h: 1080 };
  const secondaryIsland = computeTopIslandRect(secondaryBounds);
  const nearLeftSeam = placeAnchoredPopover(
    { left: 1922, top: 200, width: 120, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    secondaryBounds,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [secondaryIsland] },
  );
  assert.ok(nearLeftSeam.left >= secondaryBounds.left + POPOVER_EDGE, '오른쪽 모니터 왼쪽 경계 안');
  assert.ok(nearLeftSeam.right <= secondaryBounds.left + secondaryBounds.w - POPOVER_EDGE);
  assert.equal(nearLeftSeam.side, 'below', '위 공간 부족으로 아래로 뒤집힘');

  // 2) 음수 좌표 왼쪽 모니터 [-1280..0] 양쪽 경계
  const leftMonBounds = { left: -1280, top: 0, w: 1280, h: 720 };
  const leftMonIsland = computeTopIslandRect(leftMonBounds);
  const leftMonLeftEdge = placeAnchoredPopover(
    { left: -1290, top: 450, width: 100, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    leftMonBounds,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [leftMonIsland] },
  );
  assert.equal(leftMonLeftEdge.left, -1280 + POPOVER_EDGE);
  assert.ok(leftMonLeftEdge.right <= -POPOVER_EDGE);

  const leftMonRightEdge = placeAnchoredPopover(
    { left: -50, top: 450, width: 100, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    leftMonBounds,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [leftMonIsland] },
  );
  assert.equal(leftMonRightEdge.right, -POPOVER_EDGE);

  // 3) macOS Retina(2x) 메뉴 막대 아래 섬(bottom=137) 때문에 top=450이어도 아래로 뒤집힘
  const macViewport = { w: 1512, h: 982 };
  const macIsland = computeTopIslandRect(macViewport, {
    bounds: { x: 1920, y: 0, w: 3024, h: 1964 },
    work: { x: 1920, y: 74, w: 3024, h: 1890 },
    scale: 2,
  });
  assert.ok(macIsland && macIsland.bottom === 137);
  const macNearTop = placeAnchoredPopover(
    { left: 600, top: 450, width: 260, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    macViewport,
    { gap: 6, edge: POPOVER_EDGE, avoidRects: [macIsland] },
  );
  // 450 - 6 - 320 = 124 < 137 → 섬과 겹치므로 아래로 뒤집는다
  assert.equal(macNearTop.side, 'below');
  assert.equal(macNearTop.top, 486);
  assert.equal(rectsOverlap(macNearTop, macIsland), false);

  // 4) preferredSide='below' 전달 시(패널이 툴바 아래로 뒤집혔을 때) 아래 우선, 모자라면 위
  const preferBelow = placeAnchoredPopover(
    { left: 467, top: 452, width: 260, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    { w: 1280, h: 820 },
    { gap: 6, edge: POPOVER_EDGE, preferredSide: 'below', avoidRects: [computeTopIslandRect({ w: 1280, h: 820 })] },
  );
  assert.equal(preferBelow.side, 'below');
  assert.equal(preferBelow.top, 488);

  const preferBelowFallbackAbove = placeAnchoredPopover(
    { left: 467, top: 452, width: 260, height: 30 },
    { width: 264, height: 320, minHeight: 180 },
    { w: 1280, h: 720 },
    { gap: 6, edge: POPOVER_EDGE, preferredSide: 'below', avoidRects: [computeTopIslandRect({ w: 1280, h: 720 })] },
  );
  assert.equal(preferBelowFallbackAbove.side, 'above');
  assert.equal(preferBelowFallbackAbove.top, 452 - 6 - 320);
});
