import test from 'node:test';
import assert from 'node:assert/strict';
import {
  MAGNIFIER_LENS,
  findMonitorForOverlay,
  clientToPhysical,
  physicalToClient,
  isPhysicalPointInMonitor,
  isPhysicalRectOverlappingMonitor,
  physicalRectToClient,
  resolveMonitorHighlight,
  clientToImagePixel,
  computeSampleRect,
  buildMagnifierPatch,
  computeMagnifierPosition,
  formatHex,
  formatRgb,
  formatColor,
  toggleColorFormat,
} from './captureOverlay.js';

test('converts logical CSS coordinates to physical pixels and back across DPI scales and negative monitor bounds', () => {
  const primary = { bounds: { x: 0, y: 0, w: 2560, h: 1440 }, scale: 1.25 };
  assert.deepEqual(clientToPhysical(100, 200, primary), { x: 125, y: 250 });
  assert.deepEqual(physicalToClient(125, 250, primary), { x: 100, y: 200 });

  const leftSecondary = { bounds: { x: -1920, y: 120, w: 1920, h: 1080 }, scale: 1.5 };
  assert.deepEqual(clientToPhysical(400, 300, leftSecondary), { x: -1320, y: 570 });
  assert.deepEqual(physicalToClient(-1320, 570, leftSecondary), { x: 400, y: 300 });
});

test('checks whether a physical point belongs to a monitor bounds rect', () => {
  const mon = { bounds: { x: 1920, y: 0, w: 1920, h: 1080 } };
  assert.equal(isPhysicalPointInMonitor(1920, 0, mon), true);
  assert.equal(isPhysicalPointInMonitor(3839, 1079, mon), true);
  assert.equal(isPhysicalPointInMonitor(3840, 0, mon), false);
  assert.equal(isPhysicalPointInMonitor(1919, 500, mon), false);
});

test('maps client coordinates to clamped image pixels identically to 1:1 physical coords when viewport matches monitor', () => {
  // 150% DPI monitor: physical 1920x1080, CSS viewport 1280x720
  assert.deepEqual(clientToImagePixel(400, 300, 1280, 720, 1920, 1080), { x: 600, y: 450 });
  // Out-of-range coordinates clamp safely
  assert.deepEqual(clientToImagePixel(-10, 9999, 1280, 720, 1920, 1080), { x: 0, y: 1079 });
});

test('computes safe sample rect and builds 13x11 magnifier patch including monitor edges', () => {
  // Top-left corner (0, 0) on a 10x10 image
  const edgeRect = computeSampleRect(0, 0, 10, 10);
  assert.deepEqual(edgeRect, { sx: 0, sy: 0, sw: 7, sh: 6, dstCol: 6, dstRow: 5 });

  // Fill 7x6 source patch with distinct pixel at (0,0) = orange #FF9F0A
  const src = new Uint8ClampedArray(edgeRect.sw * edgeRect.sh * 4);
  src[0] = 255;
  src[1] = 159;
  src[2] = 10;
  src[3] = 255;

  const patch = buildMagnifierPatch(src, edgeRect);
  assert.equal(patch.width, MAGNIFIER_LENS.cols);
  assert.equal(patch.height, MAGNIFIER_LENS.rows);
  assert.deepEqual(patch.centerRgb, [255, 159, 10]);

  // Top-left of patch (out-of-bounds) stays transparent
  assert.equal(patch.data[3], 0);

  // Bottom-right corner (9, 9) on a 10x10 image
  const brRect = computeSampleRect(9, 9, 10, 10);
  assert.deepEqual(brRect, { sx: 3, sy: 4, sw: 7, sh: 6, dstCol: 0, dstRow: 0 });
});

test('positions magnifier at bottom-right by default and flips near viewport edges', () => {
  // Middle of 1920x1080
  const mid = computeMagnifierPosition(500, 400, 1920, 1080, 196, 111);
  assert.deepEqual(mid, { x: 518, y: 418, flipX: false, flipY: false });

  // Near right edge -> flips horizontally to left of cursor
  const right = computeMagnifierPosition(1850, 400, 1920, 1080, 196, 111);
  assert.equal(right.flipX, true);
  assert.equal(right.flipY, false);
  assert.equal(right.x, 1850 - 18 - 196);

  // Near bottom edge -> flips vertically above cursor
  const bottom = computeMagnifierPosition(500, 1020, 1920, 1080, 196, 111);
  assert.equal(bottom.flipX, false);
  assert.equal(bottom.flipY, true);
  assert.equal(bottom.y, 1020 - 18 - 111);

  // Bottom-right corner -> flips both axes
  const corner = computeMagnifierPosition(1900, 1060, 1920, 1080, 196, 111);
  assert.equal(corner.flipX, true);
  assert.equal(corner.flipY, true);
  assert.ok(corner.x + 196 <= 1920 - 8);
  assert.ok(corner.y + 111 <= 1080 - 8);
});

test('formats HEX and RGB color strings with clamping and Shift format toggle', () => {
  assert.equal(formatHex([255, 159, 10]), '#FF9F0A');
  assert.equal(formatHex([0, 260, -5]), '#00FF00');
  assert.equal(formatRgb([255, 159, 10]), '255, 159, 10');
  assert.equal(formatColor([18, 18, 18], 'hex'), '#121212');
  assert.equal(formatColor([18, 18, 18], 'rgb'), '18, 18, 18');
  assert.equal(toggleColorFormat('hex'), 'rgb');
  assert.equal(toggleColorFormat('rgb'), 'hex');
});

test('finds only matching monitor from overlay session', () => {
  const overlay = {
    session: 1,
    kind: 'region',
    cursor: { x: 100, y: 100 },
    monitors: [
      { id: 0, bounds: { x: 0, y: 0, w: 1920, h: 1080 }, work: { x: 0, y: 0, w: 1920, h: 1040 }, scale: 1, primary: true },
      { id: 1, bounds: { x: 1920, y: 0, w: 1920, h: 1080 }, work: { x: 1920, y: 0, w: 1920, h: 1040 }, scale: 1.5, primary: false },
    ],
  };
  assert.equal(findMonitorForOverlay(overlay, 1)?.scale, 1.5);
  assert.equal(findMonitorForOverlay(overlay, 2), null);
});

test('detects overlap between a physical rect and monitor bounds including edge-touching and zero-size rects', () => {
  const mon0 = { bounds: { x: 0, y: 0, w: 1920, h: 1080 }, scale: 1 };
  const mon1 = { bounds: { x: 1920, y: 0, w: 2560, h: 1440 }, scale: 1.5 };

  // Inside monitor 0
  assert.equal(isPhysicalRectOverlappingMonitor({ x: 200, y: 150, w: 600, h: 400 }, mon0), true);
  assert.equal(isPhysicalRectOverlappingMonitor({ x: 200, y: 150, w: 600, h: 400 }, mon1), false);

  // Spanning across monitor 0 and monitor 1
  const spanning = { x: 1620, y: 150, w: 600, h: 450 };
  assert.equal(isPhysicalRectOverlappingMonitor(spanning, mon0), true);
  assert.equal(isPhysicalRectOverlappingMonitor(spanning, mon1), true);

  // Touching boundary only -> not overlapping
  assert.equal(isPhysicalRectOverlappingMonitor({ x: 1920, y: 100, w: 400, h: 300 }, mon0), false);
  assert.equal(isPhysicalRectOverlappingMonitor({ x: 0, y: 1080, w: 400, h: 300 }, mon0), false);

  // Zero/negative size or missing rect
  assert.equal(isPhysicalRectOverlappingMonitor({ x: 100, y: 100, w: 0, h: 200 }, mon0), false);
  assert.equal(isPhysicalRectOverlappingMonitor({ x: 100, y: 100, w: 200, h: -10 }, mon0), false);
  assert.equal(isPhysicalRectOverlappingMonitor(null, mon0), false);
});

test('converts physical highlight rect to logical CSS rect across DPI scales and negative bounds', () => {
  const scaled = { bounds: { x: 1920, y: 0, w: 2560, h: 1440 }, scale: 1.5 };
  assert.deepEqual(physicalRectToClient({ x: 2220, y: 150, w: 900, h: 600 }, scaled), {
    x: 200,
    y: 100,
    width: 600,
    height: 400,
  });

  const leftNeg = { bounds: { x: -1920, y: 120, w: 1920, h: 1080 }, scale: 1.25 };
  assert.deepEqual(physicalRectToClient({ x: -1420, y: 370, w: 500, h: 250 }, leftNeg), {
    x: 400,
    y: 200,
    width: 400,
    height: 200,
  });

  // Spanning window on monitor 1 keeps negative x so border aligns seamlessly across seam
  assert.deepEqual(resolveMonitorHighlight({ x: 1620, y: 150, w: 600, h: 450 }, scaled), {
    x: -200,
    y: 100,
    width: 400,
    height: 300,
  });

  // Non-overlapping highlight resolves to null
  assert.equal(resolveMonitorHighlight({ x: 100, y: 100, w: 400, h: 300 }, scaled), null);
});
