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
  resolveCaptureFrame,
  formatRectSize,
  clipGuideToMonitor,
  resolveMonitorGuides,
  toCapturePointerInput,
  toCaptureWheelInput,
  shouldPreventCaptureKey,
  toCaptureKeyInput,
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

test('converts pointer, wheel, and keyboard events into CaptureInput contracts across DPI and IME', () => {
  const scaled = { bounds: { x: -1920, y: 120, w: 1920, h: 1080 }, scale: 1.5 };
  const rootRect = { left: 10, top: 20 };

  assert.deepEqual(
    toCapturePointerInput('move', { clientX: 410, clientY: 320 }, scaled, rootRect),
    { type: 'move', x: -1320, y: 570 },
  );
  assert.deepEqual(
    toCapturePointerInput('down', { clientX: 110, clientY: 220, button: 0 }, scaled, rootRect),
    { type: 'down', x: -1770, y: 420, button: 0 },
  );
  assert.deepEqual(
    toCapturePointerInput('up', { clientX: 110, clientY: 220, button: 2 }, scaled, rootRect),
    { type: 'up', x: -1770, y: 420, button: 2 },
  );
  assert.deepEqual(
    toCapturePointerInput('dblclick', { clientX: 110, clientY: 220, button: 0 }, scaled, rootRect),
    { type: 'dblclick', x: -1770, y: 420, button: 0 },
  );
  // Middle / auxiliary mouse buttons are ignored
  assert.equal(toCapturePointerInput('down', { clientX: 110, clientY: 220, button: 1 }, scaled, rootRect), null);
  assert.equal(toCapturePointerInput('up', { clientX: 110, clientY: 220, button: 3 }, scaled, rootRect), null);

  // Wheel: up = -1 (widen), down = 1 (narrow), zero = null
  assert.deepEqual(toCaptureWheelInput({ deltaY: -120 }), { type: 'wheel', dy: -1 });
  assert.deepEqual(toCaptureWheelInput({ deltaY: 100 }), { type: 'wheel', dy: 1 });
  assert.equal(toCaptureWheelInput({ deltaY: 0 }), null);

  // Keyboard: passes key + physical code + modifier flags (works under Korean IME too)
  assert.deepEqual(
    toCaptureKeyInput({ key: 'Process', code: 'KeyR', ctrlKey: false, shiftKey: false, altKey: false }),
    { type: 'key', key: 'Process', code: 'KeyR', ctrl: false, shift: false, alt: false, meta: false },
  );
  assert.deepEqual(
    toCaptureKeyInput({ key: 's', code: 'KeyS', ctrlKey: true, shiftKey: true, altKey: false }),
    { type: 'key', key: 's', code: 'KeyS', ctrl: true, shift: true, alt: false, meta: false },
  );

  // Default prevention covers all capture keys while leaving F12 / Ctrl+R alone
  for (const ev of [
    { key: 'Escape', code: 'Escape' },
    { key: 'Tab', code: 'Tab' },
    { key: 'Enter', code: 'Enter' },
    { key: 'ArrowUp', code: 'ArrowUp', ctrlKey: true },
    { key: 'Process', code: 'KeyW' },
    { key: 'r', code: 'KeyR' },
    { key: ',', code: 'Comma' },
    { key: '.', code: 'Period' },
    { key: '`', code: 'Backquote' },
    { key: '?', code: 'Slash', shiftKey: true },
    { key: 'a', code: 'KeyA', ctrlKey: true },
    { key: 'c', code: 'KeyC', ctrlKey: true },
    { key: 's', code: 'KeyS', ctrlKey: true },
    { key: 'Alt', code: 'AltLeft', altKey: true },
  ]) {
    assert.equal(shouldPreventCaptureKey(ev), true, `should prevent ${ev.code}`);
  }
  assert.equal(shouldPreventCaptureKey({ key: 'F12', code: 'F12' }), false);
  assert.equal(shouldPreventCaptureKey({ key: 'r', code: 'KeyR', ctrlKey: true }), false);
});

test('resolves CaptureView phase and rect into SelectionFrame state without falling back to stale highlight', () => {
  const mon = { bounds: { x: 0, y: 0, w: 1920, h: 1080 }, scale: 2 };
  const initialHl = { x: 0, y: 0, w: 1920, h: 1080 };

  // Before first capture view: uses initial session highlight
  assert.deepEqual(resolveCaptureFrame(null, initialHl, mon), {
    x: 0,
    y: 0,
    width: 960,
    height: 540,
    state: 'picking',
    instant: false,
  });

  // Drag phase disables transition lag (instant: true)
  assert.deepEqual(
    resolveCaptureFrame(
      { session: 1, phase: 'drag', rect: { x: 200, y: 100, w: 400, h: 200 }, path: [], level: -1, guides: [], magnet: true, cursorIncluded: false, help: false },
      initialHl,
      mon,
    ),
    { x: 100, y: 50, width: 200, height: 100, state: 'picking', instant: true },
  );

  // Confirmed phase renders corner handles (state: 'confirmed')
  assert.deepEqual(
    resolveCaptureFrame(
      { session: 1, phase: 'confirmed', rect: { x: 200, y: 100, w: 400, h: 200 }, path: [], level: -1, guides: [], magnet: true, cursorIncluded: false, help: false },
      initialHl,
      mon,
    ),
    { x: 100, y: 50, width: 200, height: 100, state: 'confirmed', instant: false },
  );

  // Explicit null rect in CaptureView clears highlight instead of reverting to sessionHighlight
  assert.equal(
    resolveCaptureFrame(
      { session: 1, phase: 'hover', rect: null, path: [], level: -1, guides: [], magnet: true, cursorIncluded: false, help: false },
      initialHl,
      mon,
    ),
    null,
  );
});

test('clips magnetic snap guides to monitor bounds (including monitor edges) and converts to logical CSS', () => {
  const mon1 = { bounds: { x: 1920, y: 0, w: 2560, h: 1440 }, scale: 1.5 };

  // Horizontal guide spanning across monitor 0 and monitor 1 -> clipped to monitor 1
  assert.deepEqual(
    clipGuideToMonitor({ x1: 1000, y1: 300, x2: 3420, y2: 300 }, mon1),
    { x1: 0, y1: 200, x2: 1000, y2: 200 },
  );

  // Vertical guide right on monitor 1's left boundary -> kept
  assert.deepEqual(
    clipGuideToMonitor({ x1: 1920, y1: 150, x2: 1920, y2: 750 }, mon1),
    { x1: 0, y1: 100, x2: 0, y2: 500 },
  );

  // Guide completely on monitor 0 -> filtered out
  assert.equal(clipGuideToMonitor({ x1: 500, y1: 0, x2: 500, y2: 1080 }, mon1), null);

  // Batch resolution filters and clips
  const resolved = resolveMonitorGuides(
    [
      { x1: 500, y1: 0, x2: 500, y2: 1080 },
      { x1: 2220, y1: -100, x2: 2220, y2: 600 },
    ],
    mon1,
  );
  assert.deepEqual(resolved, [{ x1: 200, y1: 0, x2: 200, y2: 400 }]);

  // Rect size badge formatting
  assert.equal(formatRectSize({ w: 718, h: 40 }), '718×40');
  assert.equal(formatRectSize({ w: 0, h: 40 }), '');
  assert.equal(formatRectSize(null), '');
});
