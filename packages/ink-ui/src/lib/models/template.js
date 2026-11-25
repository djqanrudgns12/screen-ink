// 파일 이름 템플릿 검사·미리보기 (S03 캡처·녹화)

const FORBIDDEN_RE = /[\\/:*?"<>|]/;

/**
 * Windows 파일 이름 금지 글자 검사.
 * @param {string} template
 * @returns {string | null} 오류 문구 또는 null
 */
export function validateFilenameTemplate(template) {
  if (!template || !template.trim()) return '파일 이름을 입력하세요';
  if (FORBIDDEN_RE.test(template)) return '\\ / : * ? " < > | 는 쓸 수 없어요';
  return null;
}

/**
 * 템플릿 토큰({yyyy}, {MM}, {dd}, {HH}, {mm}, {ss}, {n})을 미리보기 파일 이름으로 바꾼다.
 * @param {string} template
 * @param {Date} [now]
 * @param {number} [seq=1]
 * @returns {string}
 */
export function renderFilenamePreview(template, now = new Date(2026, 10, 25, 10, 31, 0), seq = 1) {
  const pad2 = (n) => String(n).padStart(2, '0');
  const pad3 = (n) => String(n).padStart(3, '0');
  const name = (template || 'capture')
    .replaceAll('{yyyy}', String(now.getFullYear()))
    .replaceAll('{MM}', pad2(now.getMonth() + 1))
    .replaceAll('{dd}', pad2(now.getDate()))
    .replaceAll('{HH}', pad2(now.getHours()))
    .replaceAll('{mm}', pad2(now.getMinutes()))
    .replaceAll('{ss}', pad2(now.getSeconds()))
    .replaceAll('{n}', pad3(seq));
  return `${name}.png`;
}

/**
 * 긴 저장 경로 앞부분을 줄여 표시한다.
 * @param {string} path
 * @param {number} [maxLen=28]
 * @returns {string}
 */
export function shortenPath(path, maxLen = 28) {
  if (!path || path.length <= maxLen) return path || '';
  const tail = path.slice(-(maxLen - 1));
  const sepIdx = tail.search(/[\\/]/);
  return sepIdx > 0 ? `…${tail.slice(sepIdx)}` : `…${tail}`;
}
