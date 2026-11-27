/**
 * 웹뷰가 보낸 읽기 전용 원본을 비트맵으로 옮긴 뒤 공유 메모리를 놓는다.
 * 이전 세션의 응답은 무시한다. 창이 닫히거나 구버전 웹뷰가 요청을 거절하면 이미지 주소로 돌아간다.
 * @param {number} session
 * @param {AbortSignal} signal
 * @param {() => Promise<unknown>} request
 * @returns {Promise<ImageBitmap | null>}
 */
export function readDrawFrame(session, signal, request) {
  const webview = /** @type {any} */ (globalThis).chrome?.webview;
  if (!webview?.releaseBuffer || !globalThis.createImageBitmap || signal.aborted) return Promise.resolve(null);
  return new Promise((resolve) => {
    let settled = false;
    function finish(image = /** @type {ImageBitmap | null} */ (null)) {
      if (settled) {
        image?.close();
        return;
      }
      settled = true;
      clearTimeout(timer);
      signal.removeEventListener('abort', cancel);
      webview.removeEventListener('sharedbufferreceived', receive);
      resolve(image);
    }
    const cancel = () => finish();
    /** @param {any} event */
    async function receive(event) {
      const meta = event.additionalData;
      if (meta?.session !== session) return;
      const buffer = event.getBuffer();
      try {
        const pixels = new ImageData(new Uint8ClampedArray(buffer), meta.width, meta.height);
        finish(await createImageBitmap(pixels));
      } catch {
        cancel();
      } finally {
        webview.releaseBuffer(buffer);
      }
    }
    const timer = setTimeout(cancel, 1000);
    webview.addEventListener('sharedbufferreceived', receive);
    signal.addEventListener('abort', cancel, { once: true });
    request().catch(cancel);
  });
}
