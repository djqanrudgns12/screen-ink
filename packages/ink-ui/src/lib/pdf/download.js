// 단축키 PDF 파일명 생성 및 저장(Tauri savePdf 우선, 없으면 브라우저 Blob 내려받기)
import { normalizeLang } from '../i18n/core.js';
import { buildShortcutSheetPdf } from './renderPdf.js';
import { localDateStamp } from './layoutModel.js';

/**
 * 날짜와 언어에 따른 제안 PDF 파일명을 반환한다.
 * @param {string} [lang='ko']
 * @param {string} [date]
 */
export function suggestedShortcutPdfName(lang = 'ko', date) {
  const l = normalizeLang(lang);
  const d = date || localDateStamp();
  return l === 'en' ? `shookshok-shortcuts-${d}.pdf` : `슉쇽-단축키-${d}.pdf`;
}

/**
 * PDF를 생성해 `client.savePdf`가 있으면 네이티브 저장 대화상자로, 없으면 Blob 내려받기로 저장한다.
 * @param {import('@screen-ink/client').InkClient | null | undefined} client
 * @param {{
 *   shortcuts?: any[],
 *   lang?: string,
 *   os?: string,
 *   date?: string,
 *   version?: string,
 *   region?: string,
 * }} [options]
 * @returns {Promise<{ ok: boolean, name: string }>}
 */
export async function exportShortcutSheetPdf(client, options = {}) {
  const date = options.date || localDateStamp();
  const name = suggestedShortcutPdfName(options.lang, date);
  const bytes = await buildShortcutSheetPdf({ ...options, date });

  if (typeof client?.savePdf === 'function') {
    const saved = await client.savePdf(bytes, name);
    return { ok: Boolean(saved), name };
  }

  if (typeof document !== 'undefined' && typeof URL !== 'undefined') {
    const blob = new Blob([bytes], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    try {
      const a = document.createElement('a');
      a.href = url;
      a.download = name;
      a.style.display = 'none';
      document.body.appendChild(a);
      a.click();
      a.remove();
      return { ok: true, name };
    } finally {
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    }
  }

  return { ok: false, name };
}
