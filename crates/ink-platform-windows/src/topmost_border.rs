//! 창마다 메시지 루프 하나가 테두리·이벤트 훅·타이머를 함께 소유한다.
//! 이동만 하면 기존 그림을 옮기고 크기·배율·모서리·색이 달라질 때만 다시 그린다.

use crate::screen::{cloaked, frame_bounds};
use crate::topmost::{owner, release, wide};
use ink_app::ports::{BorderStyle, PinError};
use std::cell::Cell;
use std::ptr::{null, null_mut};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM},
    Graphics::{Dwm::*, Gdi::*},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
    UI::{Accessibility::*, HiDpi::*, WindowsAndMessaging::*},
};

const REFRESH: u32 = WM_APP + 1;
const STYLE: u32 = WM_APP + 2;
thread_local! { static TARGET: Cell<HWND> = const { Cell::new(null_mut()) }; }

unsafe extern "system" fn on_event(_: HWINEVENTHOOK, _: u32, hwnd: HWND, object: i32, child: i32, _: u32, _: u32) {
    if object == OBJID_WINDOW && child == 0 && TARGET.get() == hwnd {
        PostThreadMessageW(GetCurrentThreadId(), REFRESH, 0, 0);
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_ENDSESSION && w != 0 {
        let _ = release(TARGET.get(), std::process::id() as usize);
    }
    if msg == WM_NCHITTEST {
        return HTTRANSPARENT as isize;
    }
    DefWindowProcW(hwnd, msg, w, l)
}

pub(super) struct Border {
    thread: u32,
    join: Option<JoinHandle<()>>,
}

impl Border {
    pub fn start(handle: u64, pid: u32, style: BorderStyle) -> Result<Self, PinError> {
        let (ready, rx) = mpsc::sync_channel(1);
        let join = thread::Builder::new()
            .name("창 고정 테두리".into())
            .spawn(move || unsafe { run(handle as HWND, pid, style, ready) })
            .map_err(|e| PinError::Other(e.to_string()))?;
        match rx.recv() {
            Ok(Ok(thread)) => Ok(Self { thread, join: Some(join) }),
            res => {
                let _ = join.join();
                Err(res
                    .ok()
                    .and_then(Result::err)
                    .unwrap_or_else(|| PinError::Other("테두리 시작 실패".into())))
            }
        }
    }
    pub fn set_style(&self, style: BorderStyle) {
        unsafe {
            PostThreadMessageW(self.thread, STYLE, style.thickness_px as usize, style.color_rgb as isize);
        }
    }
}
impl Drop for Border {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            if !join.is_finished() {
                unsafe {
                    PostThreadMessageW(self.thread, WM_QUIT, 0, 0);
                }
            }
            let _ = join.join();
        }
    }
}

unsafe fn run(target: HWND, pid: u32, mut style: BorderStyle, ready: mpsc::SyncSender<Result<u32, PinError>>) {
    SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    TARGET.set(target);
    let inst = GetModuleHandleW(null());
    let class = wide("ScreenInkPinBorder");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(procedure),
        hInstance: inst,
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    RegisterClassW(&wc);
    let flags = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
    let hwnd = CreateWindowExW(flags, class.as_ptr(), null(), WS_POPUP, 0, 0, 0, 0, null_mut(), null_mut(), inst, null());
    if hwnd.is_null() {
        return;
    }
    let mut border = Surface {
        hwnd,
        target,
        previous: None,
    };
    let init = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) != 0 && border.refresh(style);
    if !init {
        let _ = ready.send(Err(PinError::Other("테두리를 준비할 수 없어요".into())));
        DestroyWindow(hwnd);
        return;
    }
    let hooks = [
        (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
        (EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE),
        (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
        (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
        (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
    ]
    .map(|(first, last)| {
        SetWinEventHook(
            first,
            last,
            null_mut(),
            Some(on_event),
            pid,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    });
    let timer = SetTimer(hwnd, 1, 1000, None);
    let mut msg: MSG = std::mem::zeroed();
    let started = timer != 0 && ready.send(Ok(GetCurrentThreadId())).is_ok();
    while started && GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        match msg.message {
            STYLE | REFRESH | WM_TIMER => {
                if owner(target) != std::process::id() as usize {
                    break;
                }
                if msg.message == STYLE {
                    style = BorderStyle {
                        thickness_px: msg.wParam as u32,
                        color_rgb: msg.lParam as u32,
                    };
                }
                border.refresh(style);
            }
            _ => {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
    for hook in hooks {
        if !hook.is_null() {
            UnhookWinEvent(hook);
        }
    }
    KillTimer(hwnd, timer);
    DestroyWindow(hwnd);
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Shape {
    width: i32,
    height: i32,
    thickness: i32,
    radius: i32,
    color: u32,
}

struct Surface {
    hwnd: HWND,
    target: HWND,
    previous: Option<Shape>,
}

impl Surface {
    unsafe fn refresh(&mut self, style: BorderStyle) -> bool {
        let target = self.target;
        if style.thickness_px == 0 || IsWindowVisible(target) == 0 || IsIconic(target) != 0 || cloaked(target) {
            ShowWindow(self.hwnd, SW_HIDE);
            return true;
        }
        let Some(r) = frame_bounds(target) else { return false };
        let dpi = GetDpiForWindow(target).max(96);
        let scaled = |px: u32| ((px * dpi + 48) / 96) as i32;
        let mut pref = 0i32;
        DwmGetWindowAttribute(target, DWMWA_WINDOW_CORNER_PREFERENCE as u32, (&mut pref as *mut i32).cast(), 4);
        let radius = if IsZoomed(target) != 0 || pref == DWMWCP_DONOTROUND {
            0
        } else if pref == DWMWCP_ROUNDSMALL {
            4
        } else {
            8
        };
        let shape = Shape {
            width: r.right - r.left,
            height: r.bottom - r.top,
            thickness: scaled(style.thickness_px),
            radius: scaled(radius),
            color: style.color_rgb,
        };
        if shape.width <= 0 || shape.height <= 0 {
            ShowWindow(self.hwnd, SW_HIDE);
            return true;
        }
        if self.previous != Some(shape) {
            if !paint(self.hwnd, POINT { x: r.left, y: r.top }, shape) {
                ShowWindow(self.hwnd, SW_HIDE);
                return false;
            }
            self.previous = Some(shape);
        }
        // 바로 앞 창 뒤에 놓아 다른 고정 창이나 캡처 오버레이를 덮지 않는다.
        let previous = GetWindow(target, GW_HWNDPREV);
        let mut flags = SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOSIZE;
        if previous == self.hwnd {
            flags |= SWP_NOZORDER;
        }
        let after = if previous.is_null() { HWND_TOPMOST } else { previous };
        SetWindowPos(self.hwnd, after, r.left, r.top, 0, 0, flags) != 0
    }
}

fn inset(radius: i32, depth: i32) -> usize {
    if depth >= radius {
        return 0;
    }
    let dy = (radius - depth) as f64 - 0.5;
    ((radius as f64 - ((radius as f64).powi(2) - dy * dy).sqrt()).round() as usize).min(radius as usize)
}

unsafe fn paint(hwnd: HWND, at: POINT, s: Shape) -> bool {
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: s.width,
        biHeight: -s.height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..std::mem::zeroed()
    };
    let dc = CreateCompatibleDC(null_mut());
    if dc.is_null() {
        return false;
    }
    let mut bits = null_mut();
    let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    if bitmap.is_null() {
        DeleteDC(dc);
        return false;
    }
    let old = SelectObject(dc, bitmap);
    let width = s.width as usize;
    let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, width * s.height as usize);
    pixels.fill(0);
    let radius = s.radius.min(s.width.min(s.height) / 2);
    let color = 0xff000000 | s.color;
    for (y, line) in pixels.chunks_exact_mut(width).enumerate() {
        let depth = (y as i32).min(s.height - 1 - y as i32);
        let edge = inset(radius, depth);
        if depth < s.thickness || s.width <= 2 * s.thickness {
            line[edge..width - edge].fill(color);
        } else {
            let end = (s.thickness as usize + inset((radius - s.thickness).max(0), depth - s.thickness)).min(width - edge);
            line[edge..end].fill(color);
            line[width - end..width - edge].fill(color);
        }
    }
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let size = SIZE { cx: s.width, cy: s.height };
    let ok = UpdateLayeredWindow(hwnd, null_mut(), &at, &size, dc, &POINT { x: 0, y: 0 }, 0, &blend, ULW_ALPHA) != 0;
    SelectObject(dc, old);
    DeleteObject(bitmap);
    DeleteDC(dc);
    ok
}
