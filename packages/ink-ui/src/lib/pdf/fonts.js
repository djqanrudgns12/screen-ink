// PDF 생성용 내장 TTF 글꼴 로더 (네트워크 요청 없음 — 로컬 파일/번들 에셋만 사용)

const PDF_FONT_FILES = {
  display: 'Paperlogy-6SemiBold.ttf',
  body: 'Freesentation-4Regular.ttf',
  bodyBold: 'Freesentation-6SemiBold.ttf',
  mono: 'IBMPlexMono-Medium.ttf',
  symbols: 'Pretendard-Symbols.ttf',
};

/** @type {Promise<Record<keyof typeof PDF_FONT_FILES, Uint8Array>> | null} */
let cachedFontsPromise = null;

/**
 * PDF 임베딩용 글꼴이 WOFF2가 아닌 TrueType/OpenType 바이트인지 확인한다.
 * fontkit에 WOFF2를 넘기면 TTFSubset 단계에서 예외 후 내부 큐가 고착되는 문제를 원천 차단한다.
 * @param {Uint8Array} bytes
 * @param {string} filename
 */
export function assertTrueTypeBytes(bytes, filename) {
  if (!(bytes instanceof Uint8Array) || bytes.length < 12) {
    throw new Error(`PDF 글꼴 바이트가 비정상입니다: ${filename}`);
  }
  const tag =
    ((bytes[0] << 24) | (bytes[1] << 16) | (bytes[2] << 8) | bytes[3]) >>> 0;
  if (tag === 0x774f4632 || tag === 0x774f4646) {
    throw new Error(`PDF 글꼴이 WOFF/WOFF2 형식입니다(TTF 필요): ${filename}`);
  }
  const validSfnt = tag === 0x00010000 || tag === 0x74727565 || tag === 0x4f54544f;
  if (!validSfnt) {
    throw new Error(`알 수 없는 PDF 글꼴 헤더(0x${tag.toString(16)}): ${filename}`);
  }
  return bytes;
}

async function readFontAsset(filename) {
  const url = new URL(`../fonts/pdf/${filename}`, import.meta.url);
  if (url.protocol === 'file:' && typeof process !== 'undefined' && process.versions?.node) {
    const [{ readFile }, { fileURLToPath }] = await Promise.all([
      import('node:fs/promises'),
      import('node:url'),
    ]);
    const buf = await readFile(fileURLToPath(url));
    return assertTrueTypeBytes(new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength), filename);
  }
  const res = await fetch(url.href);
  if (!res.ok) {
    throw new Error(`PDF 글꼴 로드 실패: ${filename} (${res.status})`);
  }
  return assertTrueTypeBytes(new Uint8Array(await res.arrayBuffer()), filename);
}

/**
 * 5종 TTF 부분 집합 바이트를 로드해 메모리에 캐시한다.
 * @returns {Promise<{ display: Uint8Array, body: Uint8Array, bodyBold: Uint8Array, mono: Uint8Array, symbols: Uint8Array }>}
 */
export function loadPdfFonts() {
  if (!cachedFontsPromise) {
    cachedFontsPromise = (async () => {
      const entries = await Promise.all(
        Object.entries(PDF_FONT_FILES).map(async ([role, file]) => [role, await readFontAsset(file)]),
      );
      return Object.fromEntries(entries);
    })().catch((err) => {
      cachedFontsPromise = null;
      throw err;
    });
  }
  return cachedFontsPromise;
}
