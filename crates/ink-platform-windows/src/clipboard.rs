//! 클립보드(05 §3): 이미지 = CF_DIB(24bpp, 아래→위 — 한글·PPT·그림판이 다 읽음) + "PNG"(새 앱), 글 = CF_UNICODETEXT.
//! 잠겨 있으면 50 ms 간격으로 3번 다시 해 본다. `owner`는 우리 창(소유 창이 없으면 SetClipboardData가 실패).

use std::ptr::copy_nonoverlapping;
use windows_sys::Win32::Foundation::{GlobalFree, HANDLE, HWND};
use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;

/// BGRA(위→아래) → CF_DIB 덩어리(BITMAPINFOHEADER + 24bpp 아래→위, 줄은 4바이트 맞춤).
pub fn dib24(w: u32, h: u32, bgra: &[u8]) -> Vec<u8> {
    let stride = ((w * 3 + 3) & !3) as usize;
    let mut out = vec![0u8; 40 + stride * h as usize];
    out[0..4].copy_from_slice(&40u32.to_le_bytes());
    out[4..8].copy_from_slice(&(w as i32).to_le_bytes());
    out[8..12].copy_from_slice(&(h as i32).to_le_bytes()); // 양수 = 아래→위
    out[12..14].copy_from_slice(&1u16.to_le_bytes());
    out[14..16].copy_from_slice(&24u16.to_le_bytes());
    out[20..24].copy_from_slice(&((stride * h as usize) as u32).to_le_bytes());
    for y in 0..h as usize {
        let src = &bgra[y * w as usize * 4..(y + 1) * w as usize * 4];
        let row = 40 + (h as usize - 1 - y) * stride;
        for (x, px) in src.chunks_exact(4).enumerate() {
            out[row + x * 3..row + x * 3 + 3].copy_from_slice(&px[..3]);
        }
    }
    out
}

fn global(bytes: &[u8]) -> Option<HANDLE> {
    // SAFETY: 크기만큼 잡고 복사한 뒤 잠금을 푼다. 실패하면 돌려준다.
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if h.is_null() {
            return None;
        }
        let p = GlobalLock(h);
        if p.is_null() {
            GlobalFree(h);
            return None;
        }
        copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
        GlobalUnlock(h);
        Some(h)
    }
}

fn open(owner: isize) -> bool {
    for i in 0..4 {
        // SAFETY: 우리 창을 소유자로 연다.
        if unsafe { OpenClipboard(owner as HWND) } != 0 {
            return true;
        }
        if i < 3 {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    false
}

/// 형식 여러 개를 한 번에 올린다(성공한 개수).
fn put(owner: isize, items: &[(u32, Vec<u8>)]) -> Result<usize, String> {
    if !open(owner) {
        return Err("clipboard busy".into());
    }
    let mut ok = 0;
    // SAFETY: 연 클립보드를 비우고, 넘긴 핸들은 시스템이 갖는다(실패한 것만 우리가 푼다).
    unsafe {
        EmptyClipboard();
        for (fmt, bytes) in items {
            if let Some(h) = global(bytes) {
                if SetClipboardData(*fmt, h).is_null() {
                    GlobalFree(h);
                } else {
                    ok += 1;
                }
            }
        }
        CloseClipboard();
    }
    if ok == 0 {
        Err("SetClipboardData failed".into())
    } else {
        Ok(ok)
    }
}

/// 이미지: CF_DIB(필수) + PNG(있으면).
pub fn set_image(owner: isize, w: u32, h: u32, bgra: &[u8], png: Option<&[u8]>) -> Result<(), String> {
    let mut items = vec![(CF_DIB, dib24(w, h, bgra))];
    if let Some(p) = png {
        let name: Vec<u16> = "PNG\0".encode_utf16().collect();
        // SAFETY: 이름 버퍼는 NUL로 끝난다.
        let fmt = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
        if fmt != 0 {
            items.push((fmt, p.to_vec()));
        }
    }
    put(owner, &items).map(|_| ())
}

pub fn set_text(owner: isize, text: &str) -> Result<(), String> {
    let bytes: Vec<u8> = text.encode_utf16().chain(std::iter::once(0)).flat_map(|u| u.to_le_bytes()).collect();
    put(owner, &[(CF_UNICODETEXT, bytes)]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dib는_아래에서_위로_줄_맞춤() {
        // 2×2: 위 줄 빨강·초록, 아래 줄 파랑·흰
        let bgra = [0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];
        let d = dib24(2, 2, &bgra);
        assert_eq!(u32::from_le_bytes(d[0..4].try_into().unwrap()), 40);
        assert_eq!(d.len(), 40 + 8 * 2, "줄 6바이트 → 8바이트 맞춤");
        assert_eq!(&d[40..43], &[255, 0, 0], "첫 줄 = 아래 줄(파랑)");
        assert_eq!(&d[48..51], &[0, 0, 255], "둘째 줄 = 위 줄(빨강)");
    }
}
