// 시안 4(책상 옆 한 장) 벡터 PDF 생성기 — pdf-lib + @pdf-lib/fontkit, 네트워크 요청 0
import { PDFDocument, rgb } from 'pdf-lib';
import fontkit from '@pdf-lib/fontkit';
import { loadPdfFonts } from './fonts.js';
import { buildShortcutSheetModel } from './layoutModel.js';

const SYMBOL_CHARS = new Set([
  '⌥', '⌘', '⇧', '⌃', '⌫', '⏎', '↩', '⇥', '⎋',
  '←', '↑', '→', '↓', '↔', '↕', '↖', '↗', '↘', '↙',
  '●', '○', '✓', '▾', '▸',
]);

const MONO_EXTRA = new Set(['↑', '↓', '·', '×', '÷', '–', '—']);

const COLORS = {
  white: hex('#FFFFFF'),
  ink: hex('#0F0F10'),
  ink2: hex('#28282A'),
  sub: hex('#4D4D52'),
  hair: hex('#DCDCD8'),
  surface: hex('#F4F4F0'),
  surfaceBorder: hex('#CFCFCA'),
  keyBorder: hex('#2B2B2F'),
  accent: hex('#FF9F0A'),
  accentDeep: hex('#B45D00'),
  accentKeyBg: hex('#FFF3DF'),
  heroBg: hex('#141416'),
  heroTop: hex('#303036'),
  heroKeyBg: hex('#2B2B30'),
  heroKeyBorder: hex('#48484F'),
  heroModBg: hex('#33240D'),
  heroModText: hex('#FFBD59'),
  danger: hex('#FF5A5F'),
};

function hex(code) {
  const s = code.replace('#', '');
  const n = parseInt(s, 16);
  return rgb(((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255);
}

/** @type {WeakMap<object, Map<number, boolean>>} */
const glyphSupportCache = new WeakMap();

function hasGlyph(pdfFont, ch) {
  const cp = ch.codePointAt(0);
  if (cp == null) return false;
  let cache = glyphSupportCache.get(pdfFont);
  if (!cache) {
    cache = new Map();
    glyphSupportCache.set(pdfFont, cache);
  }
  if (cache.has(cp)) return cache.get(cp);

  let ok = false;
  const fkFont = pdfFont?.embedder?.font;
  if (fkFont && typeof fkFont.hasGlyphForCodePoint === 'function') {
    ok = Boolean(fkFont.hasGlyphForCodePoint(cp));
  } else {
    try {
      pdfFont.encodeText(ch);
      ok = true;
    } catch {
      ok = false;
    }
  }
  cache.set(cp, ok);
  return ok;
}

function pickFontForChar(ch, role, fonts) {
  if (SYMBOL_CHARS.has(ch) && hasGlyph(fonts.symbols, ch)) {
    return fonts.symbols;
  }
  if (role === 'mono') {
    const cp = ch.codePointAt(0) ?? 0;
    if (((cp >= 0x20 && cp <= 0x7e) || cp === 0xa0 || MONO_EXTRA.has(ch)) && hasGlyph(fonts.mono, ch)) {
      return fonts.mono;
    }
    return hasGlyph(fonts.bodyBold, ch) ? fonts.bodyBold : fonts.body;
  }
  if (role === 'display') {
    if (hasGlyph(fonts.display, ch)) return fonts.display;
    if (hasGlyph(fonts.bodyBold, ch)) return fonts.bodyBold;
  }
  if (role === 'bodyBold') {
    if (hasGlyph(fonts.bodyBold, ch)) return fonts.bodyBold;
  }
  if (hasGlyph(fonts.body, ch)) return fonts.body;
  if (hasGlyph(fonts.symbols, ch)) return fonts.symbols;
  return fonts.body;
}

function splitRuns(text, role, fonts) {
  const runs = [];
  for (const ch of String(text ?? '')) {
    const font = pickFontForChar(ch, role, fonts);
    const prev = runs.at(-1);
    if (prev && prev.font === font) {
      prev.text += ch;
    } else {
      runs.push({ font, text: ch });
    }
  }
  return runs;
}

function measureMixedText(text, size, role, fonts) {
  let width = 0;
  for (const run of splitRuns(text, role, fonts)) {
    const runSize = run.font === fonts.symbols ? size * 1.04 : size;
    width += run.font.widthOfTextAtSize(run.text, runSize);
  }
  return width;
}

function truncateMixedText(text, maxWidth, size, role, fonts) {
  const s = String(text ?? '');
  if (maxWidth <= 0) return '';
  if (measureMixedText(s, size, role, fonts) <= maxWidth) return s;
  const ell = '…';
  const ellW = measureMixedText(ell, size, role, fonts);
  const chars = Array.from(s);
  let acc = '';
  for (const ch of chars) {
    const next = acc + ch;
    if (measureMixedText(next, size, role, fonts) + ellW > maxWidth) break;
    acc = next;
  }
  return acc ? `${acc}${ell}` : ell;
}

function drawMixedText(page, text, { x, baselineTopY, size, role, color, fonts, pageH }) {
  let cx = x;
  for (const run of splitRuns(text, role, fonts)) {
    const runSize = run.font === fonts.symbols ? size * 1.04 : size;
    const isSymbol = run.font === fonts.symbols;
    const yOffset = isSymbol ? size * 0.04 : 0;
    page.drawText(run.text, {
      x: cx,
      y: pageH - (baselineTopY + yOffset),
      size: runSize,
      font: run.font,
      color,
    });
    cx += run.font.widthOfTextAtSize(run.text, runSize);
  }
  return cx - x;
}

function roundedRectPath(w, h, r) {
  const cr = Math.min(r, w / 2, h / 2);
  if (cr <= 0) {
    return `M 0 0 H ${w} V ${h} H 0 Z`;
  }
  return [
    `M ${cr} 0`,
    `H ${w - cr}`,
    `A ${cr} ${cr} 0 0 1 ${w} ${cr}`,
    `V ${h - cr}`,
    `A ${cr} ${cr} 0 0 1 ${w - cr} ${h}`,
    `H ${cr}`,
    `A ${cr} ${cr} 0 0 1 0 ${h - cr}`,
    `V ${cr}`,
    `A ${cr} ${cr} 0 0 1 ${cr} 0`,
    'Z',
  ].join(' ');
}

function drawRoundedRect(page, { x, topY, w, h, r = 0, color, borderColor, borderWidth = 0, pageH }) {
  page.drawSvgPath(roundedRectPath(w, h, r), {
    x,
    y: pageH - topY,
    color,
    borderColor,
    borderWidth,
  });
}

function drawIcon(page, kind, { x, topY, size, color, accentColor = COLORS.accent, pageH }) {
  const s = size / 24;
  const stroke = (d, col = color, bw = 2 * s) =>
    page.drawSvgPath(d, {
      x,
      y: pageH - topY,
      scale: s,
      borderColor: col,
      borderWidth: bw,
    });
  const fillPath = (d, col) =>
    page.drawSvgPath(d, {
      x,
      y: pageH - topY,
      scale: s,
      color: col,
    });
  const dot = (cx, cy, r, col) =>
    page.drawCircle({
      x: x + cx * s,
      y: pageH - (topY + cy * s),
      size: r * s,
      color: col,
    });

  if (kind === 'shortcuts') {
    stroke('M 4.5 6 H 19.5 A 2 2 0 0 1 21.5 8 V 16 A 2 2 0 0 1 19.5 18 H 4.5 A 2 2 0 0 1 2.5 16 V 8 A 2 2 0 0 1 4.5 6 Z');
    dot(6.5, 10, 1, color);
    dot(10, 10, 1, color);
    dot(14, 10, 1, color);
    dot(17.5, 10, 1, color);
    stroke('M 7.5 14 H 16.5');
    return;
  }
  if (kind === 'region') {
    stroke('M 4 8 V 6 A 2 2 0 0 1 6 4 H 8 M 16 4 H 18 A 2 2 0 0 1 20 6 V 8 M 20 16 V 18 A 2 2 0 0 1 18 20 H 16 M 8 20 H 6 A 2 2 0 0 1 4 18 V 16');
    dot(12, 12, 2.2, accentColor);
    return;
  }
  if (kind === 'window') {
    stroke('M 5.5 5 H 18.5 A 2 2 0 0 1 20.5 7 V 17 A 2 2 0 0 1 18.5 19 H 5.5 A 2 2 0 0 1 3.5 17 V 7 A 2 2 0 0 1 5.5 5 Z');
    stroke('M 3.5 9 H 20.5');
    dot(6.2, 7, 1, accentColor);
    return;
  }
  if (kind === 'record') {
    page.drawCircle({
      x: x + 12 * s,
      y: pageH - (topY + 12 * s),
      size: 8 * s,
      borderColor: color,
      borderWidth: 2 * s,
    });
    dot(12, 12, 3.4, COLORS.danger);
    return;
  }
  if (kind === 'draw') {
    stroke('M 15 4.5 L 19.5 9 L 9.5 19 H 5 V 14.5 Z');
    stroke('M 13 6.5 L 17.5 11');
    dot(4.8, 20.4, 1.5, accentColor);
    return;
  }
  if (kind === 'zoom') {
    page.drawCircle({
      x: x + 10.5 * s,
      y: pageH - (topY + 10.5 * s),
      size: 6.5 * s,
      borderColor: color,
      borderWidth: 2 * s,
    });
    stroke('M 15.5 15.5 L 20 20');
    dot(10.5, 10.5, 1.8, accentColor);
    return;
  }
  if (kind === 'board') {
    stroke('M 5 4 H 19 A 1.5 1.5 0 0 1 20.5 5.5 V 14 A 1.5 1.5 0 0 1 19 15.5 H 5 A 1.5 1.5 0 0 1 3.5 14 V 5.5 A 1.5 1.5 0 0 1 5 4 Z');
    stroke('M 8.5 20 L 10.5 15.5 M 15.5 20 L 13.5 15.5');
    fillPath('M 15 12 A 1.4 1.4 0 1 0 17.8 12 A 1.4 1.4 0 1 0 15 12 Z', accentColor);
  }
}

function measureKeycap(tok, { size, minW, padX }, fonts) {
  const tw = measureMixedText(tok, size, 'mono', fonts);
  return Math.max(minW, tw + padX * 2);
}

function drawKeycap(page, tok, { x, topY, w, h, size, r, bg, border, shadow, fg, fonts, pageH }) {
  if (shadow) {
    drawRoundedRect(page, { x, topY: topY + 1.1, w, h, r, color: shadow, pageH });
  }
  drawRoundedRect(page, {
    x,
    topY,
    w,
    h,
    r,
    color: bg,
    borderColor: border,
    borderWidth: 0.75,
    pageH,
  });
  const tw = measureMixedText(tok, size, 'mono', fonts);
  drawMixedText(page, tok, {
    x: x + (w - tw) / 2,
    baselineTopY: topY + h * 0.5 + size * 0.34,
    size,
    role: 'mono',
    color: fg,
    fonts,
    pageH,
  });
}

function measureRowRight(row, fonts) {
  const capSize = 7.4;
  const capH = 12.2;
  const minW = 12.8;
  const padX = 3.6;
  const capGap = 2.2;

  if (row.note) {
    const nw = measureMixedText(row.note, 7.2, 'bodyBold', fonts) + 8;
    return { kind: 'note', width: nw, h: 12.2 };
  }

  if (Array.isArray(row.keyGroups) && row.keyGroups.length > 0) {
    const sep = row.separator || '/';
    const sepW = measureMixedText(sep, 6.8, 'bodyBold', fonts) + 5;
    let totalW = 0;
    const groups = row.keyGroups.map((grp, idx) => {
      const caps = grp.map((tok) => ({
        tok,
        w: measureKeycap(tok, { size: capSize, minW, padX }, fonts),
      }));
      const gw = caps.reduce((s, c) => s + c.w, 0) + Math.max(0, caps.length - 1) * capGap;
      totalW += gw + (idx > 0 ? sepW : 0);
      return caps;
    });
    return { kind: 'groups', width: totalW, groups, sep, sepW, capSize, capH, capGap };
  }

  const tokens = row.tokens ?? [];
  const prefixW = row.keyPrefix ? measureMixedText(row.keyPrefix, 7.4, 'bodyBold', fonts) + 2.5 : 0;
  const caps = tokens.map((tok) => ({
    tok,
    w: measureKeycap(tok, { size: capSize, minW, padX }, fonts),
  }));
  const capsW = caps.reduce((s, c) => s + c.w, 0) + Math.max(0, caps.length - 1) * capGap;
  return { kind: 'tokens', width: prefixW + capsW, prefix: row.keyPrefix || '', prefixW, caps, capSize, capH, capGap };
}

function drawRowRight(page, rightLayout, row, { rightX, rowTopY, rowH, fonts, pageH }) {
  const capTopY = rowTopY + (rowH - 12.2) / 2;
  const mod = Boolean(row.modified);
  const bg = mod ? COLORS.accentKeyBg : COLORS.surface;
  const border = mod ? COLORS.accentDeep : COLORS.keyBorder;
  const fg = mod ? COLORS.accentDeep : COLORS.ink;

  if (rightLayout.kind === 'note') {
    const x = rightX - rightLayout.width;
    drawRoundedRect(page, {
      x,
      topY: capTopY,
      w: rightLayout.width,
      h: 12.2,
      r: 2.5,
      color: COLORS.surface,
      pageH,
    });
    const tw = measureMixedText(row.note, 7.2, 'bodyBold', fonts);
    drawMixedText(page, row.note, {
      x: x + (rightLayout.width - tw) / 2,
      baselineTopY: capTopY + 12.2 * 0.5 + 7.2 * 0.34,
      size: 7.2,
      role: 'bodyBold',
      color: COLORS.sub,
      fonts,
      pageH,
    });
    return;
  }

  let cx = rightX - rightLayout.width;
  if (rightLayout.kind === 'groups') {
    rightLayout.groups.forEach((caps, idx) => {
      if (idx > 0) {
        const sw = measureMixedText(rightLayout.sep, 6.8, 'bodyBold', fonts);
        drawMixedText(page, rightLayout.sep, {
          x: cx + (rightLayout.sepW - sw) / 2,
          baselineTopY: capTopY + 12.2 * 0.5 + 6.8 * 0.34,
          size: 6.8,
          role: 'bodyBold',
          color: COLORS.sub,
          fonts,
          pageH,
        });
        cx += rightLayout.sepW;
      }
      caps.forEach((c, ci) => {
        drawKeycap(page, c.tok, {
          x: cx,
          topY: capTopY,
          w: c.w,
          h: 12.2,
          size: rightLayout.capSize,
          r: 2.6,
          bg,
          border,
          shadow: border,
          fg,
          fonts,
          pageH,
        });
        cx += c.w + (ci < caps.length - 1 ? rightLayout.capGap : 0);
      });
    });
    return;
  }

  if (rightLayout.prefix) {
    drawMixedText(page, rightLayout.prefix, {
      x: cx,
      baselineTopY: capTopY + 12.2 * 0.5 + 7.4 * 0.34,
      size: 7.4,
      role: 'bodyBold',
      color: COLORS.sub,
      fonts,
      pageH,
    });
    cx += rightLayout.prefixW;
  }

  rightLayout.caps.forEach((c, ci) => {
    drawKeycap(page, c.tok, {
      x: cx,
      topY: capTopY,
      w: c.w,
      h: 12.2,
      size: rightLayout.capSize,
      r: 2.6,
      bg,
      border,
      shadow: border,
      fg,
      fonts,
      pageH,
    });
    cx += c.w + (ci < rightLayout.caps.length - 1 ? rightLayout.capGap : 0);
  });
}

function drawRow(page, row, { x, topY, w, h = 15.6, showDivider = true, fonts, pageH }) {
  const rightLayout = measureRowRight(row, fonts);
  const mod = Boolean(row.modified);
  let labelX = x;
  if (mod) {
    page.drawCircle({
      x: x + 2.6,
      y: pageH - (topY + h / 2),
      size: 2.4,
      color: COLORS.accent,
    });
    labelX += 7.5;
  }

  const maxLabelW = Math.max(24, x + w - rightLayout.width - 6 - labelX);
  const labelText = truncateMixedText(row.label, maxLabelW, 8.3, mod ? 'bodyBold' : 'body', fonts);
  drawMixedText(page, labelText, {
    x: labelX,
    baselineTopY: topY + h * 0.5 + 8.3 * 0.34,
    size: 8.3,
    role: mod ? 'bodyBold' : 'body',
    color: mod ? COLORS.ink : COLORS.ink2,
    fonts,
    pageH,
  });

  drawRowRight(page, rightLayout, row, {
    rightX: x + w,
    rowTopY: topY,
    rowH: h,
    fonts,
    pageH,
  });

  if (showDivider) {
    const lineY = pageH - (topY + h);
    page.drawLine({
      start: { x, y: lineY },
      end: { x: x + w, y: lineY },
      thickness: 0.45,
      color: COLORS.hair,
    });
  }
}

function drawSectionHeader(page, { title, tag, x, topY, w, fonts, pageH }) {
  drawMixedText(page, title, {
    x,
    baselineTopY: topY + 9.8,
    size: 9.8,
    role: 'display',
    color: COLORS.ink,
    fonts,
    pageH,
  });

  if (tag) {
    const tagW = measureMixedText(tag, 7.1, 'bodyBold', fonts);
    drawMixedText(page, tag, {
      x: x + w - tagW,
      baselineTopY: topY + 9.5,
      size: 7.1,
      role: 'bodyBold',
      color: COLORS.sub,
      fonts,
      pageH,
    });
  }

  const lineY = pageH - (topY + 13.5);
  page.drawLine({
    start: { x, y: lineY },
    end: { x: x + w, y: lineY },
    thickness: 1.3,
    color: COLORS.ink,
  });

  return topY + 16;
}

function drawColorStrip(page, colors, { x, topY, w, fonts, pageH }) {
  const stripH = 31;
  drawRoundedRect(page, {
    x,
    topY,
    w,
    h: stripH,
    r: 4,
    color: COLORS.surface,
    borderColor: COLORS.surfaceBorder,
    borderWidth: 0.75,
    pageH,
  });

  const cellW = w / Math.max(1, colors.length);
  colors.forEach((c, idx) => {
    const cx = x + idx * cellW + cellW / 2;
    const dotR = 3.2;
    const nameW = measureMixedText(c.name, 7.1, 'bodyBold', fonts);
    const totalTopW = dotR * 2 + 3 + nameW;
    const startTopX = cx - totalTopW / 2;

    page.drawCircle({
      x: startTopX + dotR,
      y: pageH - (topY + 8.5),
      size: dotR,
      color: hex(c.hex),
      borderColor: rgb(0, 0, 0),
      borderWidth: 0.4,
    });

    drawMixedText(page, c.name, {
      x: startTopX + dotR * 2 + 3,
      baselineTopY: topY + 10.8,
      size: 7.1,
      role: 'bodyBold',
      color: COLORS.ink,
      fonts,
      pageH,
    });

    const tok = c.tokens?.[0] ?? '';
    if (tok) {
      const cw = measureKeycap(tok, { size: 7.4, minW: 13.5, padX: 3.5 }, fonts);
      const mod = Boolean(c.modified);
      drawKeycap(page, tok, {
        x: cx - cw / 2,
        topY: topY + 15.2,
        w: cw,
        h: 12,
        size: 7.4,
        r: 2.5,
        bg: mod ? COLORS.accentKeyBg : COLORS.white,
        border: mod ? COLORS.accentDeep : COLORS.keyBorder,
        shadow: mod ? COLORS.accentDeep : COLORS.keyBorder,
        fg: mod ? COLORS.accentDeep : COLORS.ink,
        fonts,
        pageH,
      });
    }
  });

  return topY + stripH + 4;
}

function wrapText(text, maxWidth, size, role, fonts) {
  const words = String(text ?? '').split(/\s+/).filter(Boolean);
  const lines = [];
  let line = '';
  for (const word of words) {
    const test = line ? `${line} ${word}` : word;
    if (measureMixedText(test, size, role, fonts) <= maxWidth) {
      line = test;
    } else {
      if (line) lines.push(line);
      line = word;
    }
  }
  if (line) lines.push(line);
  return lines;
}

function drawTipBox(page, tipText, { x, bottomTopY, w, fonts, pageH }) {
  const lines = wrapText(tipText, w - 14, 7.6, 'bodyBold', fonts);
  const boxH = Math.max(24, lines.length * 10.5 + 10);
  const topY = bottomTopY - boxH;
  drawRoundedRect(page, {
    x,
    topY,
    w,
    h: boxH,
    r: 4,
    color: COLORS.surface,
    borderColor: COLORS.ink,
    borderWidth: 1,
    pageH,
  });
  lines.forEach((line, idx) => {
    drawMixedText(page, line, {
      x: x + 7,
      baselineTopY: topY + 12.5 + idx * 10.5,
      size: 7.6,
      role: 'bodyBold',
      color: COLORS.ink,
      fonts,
      pageH,
    });
  });
}

function drawHeader(page, model, { marginX, topY, contentW, fonts, pageH }) {
  drawIcon(page, 'shortcuts', { x: marginX, topY: topY + 1, size: 15.5, color: COLORS.ink, pageH });

  const titleX = marginX + 20;
  const titleW = drawMixedText(page, model.header.title, {
    x: titleX,
    baselineTopY: topY + 13.5,
    size: 15,
    role: 'display',
    color: COLORS.ink,
    fonts,
    pageH,
  });

  const badgeX = titleX + titleW + 8;
  const badgeTextW = measureMixedText(model.header.badge, 7.6, 'display', fonts);
  const badgeW = badgeTextW + 12;
  drawRoundedRect(page, {
    x: badgeX,
    topY: topY + 1.5,
    w: badgeW,
    h: 15,
    r: 3,
    color: COLORS.ink,
    pageH,
  });
  drawMixedText(page, model.header.badge, {
    x: badgeX + 6,
    baselineTopY: topY + 11.4,
    size: 7.6,
    role: 'display',
    color: COLORS.white,
    fonts,
    pageH,
  });

  // 오른쪽 범례(● 내가 바꾼 키) + 메타
  const metaW = measureMixedText(model.header.meta, 8, 'mono', fonts);
  const metaX = marginX + contentW - metaW;
  drawMixedText(page, model.header.meta, {
    x: metaX,
    baselineTopY: topY + 12.5,
    size: 8,
    role: 'mono',
    color: COLORS.sub,
    fonts,
    pageH,
  });

  const legendW = measureMixedText(model.header.legendCustom, 8, 'bodyBold', fonts);
  const legendTextX = metaX - 12 - legendW;
  page.drawCircle({
    x: legendTextX - 6,
    y: pageH - (topY + 9.5),
    size: 2.7,
    color: COLORS.accent,
  });
  drawMixedText(page, model.header.legendCustom, {
    x: legendTextX,
    baselineTopY: topY + 12.5,
    size: 8,
    role: 'bodyBold',
    color: COLORS.ink,
    fonts,
    pageH,
  });

  const borderY = pageH - (topY + 21);
  page.drawLine({
    start: { x: marginX, y: borderY },
    end: { x: marginX + contentW, y: borderY },
    thickness: 1.6,
    color: COLORS.ink,
  });

  return topY + 26;
}

function drawKeyGuideStrip(page, keyGuide, { marginX, topY, contentW, fonts, pageH }) {
  const stripH = 16;
  drawRoundedRect(page, {
    x: marginX,
    topY,
    w: contentW,
    h: stripH,
    r: 3.5,
    color: COLORS.surface,
    borderColor: COLORS.hair,
    borderWidth: 0.7,
    pageH,
  });

  const labelW = measureMixedText(keyGuide.label, 7.2, 'bodyBold', fonts);
  drawRoundedRect(page, {
    x: marginX + 4,
    topY: topY + 2.5,
    w: labelW + 8,
    h: 11,
    r: 2.2,
    color: COLORS.ink,
    pageH,
  });
  drawMixedText(page, keyGuide.label, {
    x: marginX + 8,
    baselineTopY: topY + 10.4,
    size: 7.2,
    role: 'bodyBold',
    color: COLORS.white,
    fonts,
    pageH,
  });

  drawMixedText(page, keyGuide.text, {
    x: marginX + labelW + 18,
    baselineTopY: topY + 10.6,
    size: 7.6,
    role: 'bodyBold',
    color: COLORS.ink2,
    fonts,
    pageH,
  });

  return topY + stripH + 6;
}

function drawHeroSection(page, model, { marginX, topY, contentW, fonts, pageH }) {
  if (!model.hero || model.hero.length === 0) return topY;

  drawMixedText(page, model.heroHeading, {
    x: marginX,
    baselineTopY: topY + 7.2,
    size: 7.6,
    role: 'bodyBold',
    color: COLORS.sub,
    fonts,
    pageH,
  });

  const gridTopY = topY + 11;
  const gap = 9;
  const count = model.hero.length;
  const cardW = (contentW - gap * (count - 1)) / count;
  const cardH = 48;

  model.hero.forEach((h, idx) => {
    const cx = marginX + idx * (cardW + gap);
    const mod = Boolean(h.modified);

    drawRoundedRect(page, {
      x: cx,
      topY: gridTopY,
      w: cardW,
      h: cardH,
      r: 6,
      color: COLORS.heroBg,
      pageH,
    });

    // 상단 강조 바
    drawRoundedRect(page, {
      x: cx + 1,
      topY: gridTopY,
      w: cardW - 2,
      h: 2.4,
      r: 1.2,
      color: mod ? COLORS.accent : COLORS.heroTop,
      pageH,
    });

    drawIcon(page, h.icon, {
      x: cx + 8,
      topY: gridTopY + 7.5,
      size: 11.5,
      color: COLORS.white,
      pageH,
    });

    const maxTitleW = cardW - (mod ? 34 : 24);
    const title = truncateMixedText(h.title, maxTitleW, 9.6, 'display', fonts);
    drawMixedText(page, title, {
      x: cx + 22,
      baselineTopY: gridTopY + 16.5,
      size: 9.6,
      role: 'display',
      color: COLORS.white,
      fonts,
      pageH,
    });

    if (mod) {
      page.drawCircle({
        x: cx + cardW - 9,
        y: pageH - (gridTopY + 13),
        size: 2.6,
        color: COLORS.accent,
      });
    }

    // 히어로 키캡
    const capH = 18.5;
    const capY = gridTopY + 23.5;
    const capGap = 3.5;
    let kx = cx + 8;
    for (const tok of h.tokens) {
      const cw = measureKeycap(tok, { size: 10.2, minW: 20, padX: 5.5 }, fonts);
      drawKeycap(page, tok, {
        x: kx,
        topY: capY,
        w: cw,
        h: capH,
        size: 10.2,
        r: 3.5,
        bg: mod ? COLORS.heroModBg : COLORS.heroKeyBg,
        border: mod ? COLORS.accent : COLORS.heroKeyBorder,
        shadow: null,
        fg: mod ? COLORS.heroModText : COLORS.white,
        fonts,
        pageH,
      });
      kx += cw + capGap;
    }
  });

  return gridTopY + cardH + 10;
}

function drawFooter(page, model, pageNumber, { marginX, pageW, pageH, fonts }) {
  const contentW = pageW - marginX * 2;
  const lineTopY = pageH - 28;
  const lineY = pageH - lineTopY;

  page.drawLine({
    start: { x: marginX, y: lineY },
    end: { x: marginX + contentW, y: lineY },
    thickness: 0.9,
    color: COLORS.ink,
  });

  const rightText =
    model.pages.length > 1
      ? `${model.footer.right} · ${model.footer.pagePattern(pageNumber, model.pages.length)}`
      : model.footer.right;
  const rightW = measureMixedText(rightText, 7.2, 'mono', fonts);

  if (model.footer.left) {
    const leftText = truncateMixedText(model.footer.left, contentW - rightW - 16, 7.2, 'bodyBold', fonts);
    drawMixedText(page, leftText, {
      x: marginX,
      baselineTopY: lineTopY + 11.5,
      size: 7.2,
      role: 'bodyBold',
      color: COLORS.sub,
      fonts,
      pageH,
    });
  }

  drawMixedText(page, rightText, {
    x: marginX + contentW - rightW,
    baselineTopY: lineTopY + 11.5,
    size: 7.2,
    role: 'mono',
    color: COLORS.sub,
    fonts,
    pageH,
  });
}

function withTimeout(promise, ms, label) {
  let timer;
  const timeoutPromise = new Promise((_, reject) => {
    timer = setTimeout(() => {
      reject(new Error(`${label} 시간 초과 (${ms}ms)`));
    }, ms);
    timer.unref?.();
  });
  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timer !== undefined) clearTimeout(timer);
  });
}

/**
 * `InkState.shortcuts`에서 시안 4(책상 옆 한 장) 벡터 PDF 바이트(`Uint8Array`)를 생성한다.
 * @param {{
 *   shortcuts?: Array<{ id: string, scope?: string, group?: string, label?: string, combo?: string, modified?: boolean, windowsOnly?: boolean, later?: boolean }>,
 *   lang?: string,
 *   os?: string,
 *   date?: string,
 *   version?: string,
 *   region?: string,
 * }} [options]
 * @returns {Promise<Uint8Array>}
 */
export async function buildShortcutSheetPdf(options = {}) {
  const model = buildShortcutSheetModel(options);
  const rawFonts = await withTimeout(loadPdfFonts(), 15000, 'loadPdfFonts');

  const pdfDoc = await withTimeout(PDFDocument.create(), 15000, 'PDFDocument.create');
  pdfDoc.registerFontkit(fontkit);
  pdfDoc.setTitle(model.header.title);
  pdfDoc.setCreator('ShookShok');
  pdfDoc.setProducer('ShookShok PDF Generator');

  const fonts = {
    display: await withTimeout(pdfDoc.embedFont(rawFonts.display, { subset: true }), 15000, 'embedFont(display)'),
    body: await withTimeout(pdfDoc.embedFont(rawFonts.body, { subset: true }), 15000, 'embedFont(body)'),
    bodyBold: await withTimeout(pdfDoc.embedFont(rawFonts.bodyBold, { subset: true }), 15000, 'embedFont(bodyBold)'),
    mono: await withTimeout(pdfDoc.embedFont(rawFonts.mono, { subset: true }), 15000, 'embedFont(mono)'),
    symbols: await withTimeout(pdfDoc.embedFont(rawFonts.symbols, { subset: true }), 15000, 'embedFont(symbols)'),
  };

  const { width: pageW, height: pageH } = model.pageSize;
  const marginX = 28;
  const contentW = pageW - marginX * 2;
  const colGap = 15;
  const unit = (contentW - colGap * 2) / (0.98 + 1.28 + 0.94);
  const colWidths = [unit * 0.98, unit * 1.28, unit * 0.94];
  const colXs = [
    marginX,
    marginX + colWidths[0] + colGap,
    marginX + colWidths[0] + colGap + colWidths[1] + colGap,
  ];

  for (const pageModel of model.pages) {
    const page = pdfDoc.addPage([pageW, pageH]);
    // 흰 바탕
    page.drawRectangle({ x: 0, y: 0, width: pageW, height: pageH, color: COLORS.white });

    let cursorY = drawHeader(page, model, { marginX, topY: 18, contentW, fonts, pageH });
    cursorY = drawKeyGuideStrip(page, model.keyGuide, { marginX, topY: cursorY, contentW, fonts, pageH });
    if (pageModel.showHero) {
      cursorY = drawHeroSection(page, model, { marginX, topY: cursorY, contentW, fonts, pageH });
    } else {
      cursorY += 4;
    }

    for (let c = 0; c < 3; c += 1) {
      let y = cursorY;
      const cx = colXs[c];
      const cw = colWidths[c];
      for (const sec of pageModel.columns[c]) {
        y = drawSectionHeader(page, { title: sec.title, tag: sec.tag, x: cx, topY: y, w: cw, fonts, pageH });
        if (sec.kind === 'drawGrid') {
          if (sec.colors.length > 0) {
            y = drawColorStrip(page, sec.colors, { x: cx, topY: y, w: cw, fonts, pageH });
          }
          const subGap = 11;
          const subW = (cw - subGap) / 2;
          let ly = y;
          sec.leftRows.forEach((r, idx) => {
            drawRow(page, r, {
              x: cx,
              topY: ly,
              w: subW,
              showDivider: idx < sec.leftRows.length - 1,
              fonts,
              pageH,
            });
            ly += 15.6;
          });
          let ry = y;
          sec.rightRows.forEach((r, idx) => {
            drawRow(page, r, {
              x: cx + subW + subGap,
              topY: ry,
              w: subW,
              showDivider: idx < sec.rightRows.length - 1,
              fonts,
              pageH,
            });
            ry += 15.6;
          });
          y = Math.max(ly, ry) + 8;
        } else {
          const rows = sec.rows ?? [];
          rows.forEach((r, idx) => {
            drawRow(page, r, {
              x: cx,
              topY: y,
              w: cw,
              showDivider: idx < rows.length - 1,
              fonts,
              pageH,
            });
            y += 15.6;
          });
          y += 8;
        }
      }
    }

    if (pageModel.tip) {
      drawTipBox(page, pageModel.tip, {
        x: colXs[2],
        bottomTopY: pageH - 36,
        w: colWidths[2],
        fonts,
        pageH,
      });
    }

    drawFooter(page, model, pageModel.pageNumber, { marginX, pageW, pageH, fonts });
  }

  return withTimeout(pdfDoc.save(), 15000, 'pdfDoc.save');
}
