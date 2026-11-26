//! F2 창 캡처 어댑터(단계 6.1, spikes-windows §1): `windows` 0.62로 WGC를 직접 쓴다(windows-capture 안 씀).
//!
//! - WGC 객체는 전용 일꾼 스레드(MTA) 하나가 쥔다. 메인 스레드는 "준비"를 던지고(기다리지 않음), "찍기"만 답을 기다린다.
//! - F2 고르기 중 커서 아래 창이 바뀌면 item·pool·session을 미리 만든다(≈15 ms 숨김). 찍기 = StartCapture → 첫 프레임 150 ms.
//! - 테두리 끄기(`SetIsBorderRequired(false)`), 커서 빼기. 프레임 풀 텍스처는 바로 스테이징으로 복사해 돌려준다.
//! - 판정(`windowshot::after_primary`): 내용 → 씀, 검정·시간 초과 + 표시 선호도 → `WINDOW_PROTECTED`, 그 밖 → 서비스가 스냅숏 자르기.
//! - 창 제목은 읽지 않는다. 로그는 방식·크기·시간만(R-LOG-1).

use crate::gpu::{self, Gpu};
use crate::screen::WindowsScreen;
use ink_app::{Frame, ScreenSource, WindowShot, WINDOW_PROTECTED};
use ink_domain::geometry::{MonitorLayout, PhysRect};
use ink_domain::image::blankness;
use ink_domain::windowshot::{self, ShotNext, FIRST_FRAME_MS};
use std::ffi::c_void;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::{Interface, HSTRING};
use windows::Graphics::Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS, DWMWINDOWATTRIBUTE};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{GetWindowDisplayAffinity, GetWindowRect, IsZoomed};

/// `DWMWA_WINDOW_CORNER_PREFERENCE`(Win11, 33).
const DWMWA_WINDOW_CORNER_PREFERENCE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(33);
/// 찍기 답을 기다리는 안전 시간(첫 프레임 150 ms + 준비·읽기).
const REPLY_TIMEOUT: Duration = Duration::from_millis(2_000);

enum Job {
    Prepare(Option<u64>),
    Shoot { window: u64, reply: Sender<Result<WindowShot, String>> },
}

/// `ScreenSource` = GDI 정지 캡처(`WindowsScreen`) + WGC 창 캡처.
pub struct WgcScreen {
    base: WindowsScreen,
    jobs: Option<Sender<Job>>,
}

impl Default for WgcScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl WgcScreen {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let spawned = std::thread::Builder::new().name("shookshok-wgc-window".into()).spawn(move || worker(rx)).is_ok();
        if !spawned {
            log::warn!("window capture worker could not start");
        }
        WgcScreen { base: WindowsScreen, jobs: spawned.then_some(tx) }
    }
}

impl ScreenSource for WgcScreen {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
        self.base.capture(layout)
    }

    fn cursor_sprite(&mut self) -> Option<ink_domain::image::CursorSprite> {
        self.base.cursor_sprite()
    }

    fn prepare_window(&mut self, window: Option<u64>) {
        if let Some(tx) = &self.jobs {
            let _ = tx.send(Job::Prepare(window));
        }
    }

    fn capture_window(&mut self, window: u64, _bounds: PhysRect) -> Result<WindowShot, String> {
        let tx = self.jobs.as_ref().ok_or("window capture worker missing")?;
        let (reply, rx) = channel();
        tx.send(Job::Shoot { window, reply }).map_err(|_| "window capture worker stopped".to_string())?;
        rx.recv_timeout(REPLY_TIMEOUT).map_err(|_| "window capture timed out".to_string())?
    }
}

/// 미리 만든 캡처(아직 StartCapture 전).
struct Prepared {
    window: u64,
    gpu: Arc<Gpu>,
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
}

impl Prepared {
    fn close(self) {
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}

fn hwnd(window: u64) -> HWND {
    HWND(window as usize as *mut c_void)
}

fn worker(rx: Receiver<Job>) {
    // SAFETY: 이 스레드를 MTA로 — WinRT 객체는 이 스레드에서만 만든다.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    log_border_access();
    let mut ready: Option<Prepared> = None;
    for job in rx {
        match job {
            Job::Prepare(None) => {
                if let Some(p) = ready.take() {
                    p.close();
                }
            }
            Job::Prepare(Some(w)) => {
                if ready.as_ref().map(|p| p.window) == Some(w) {
                    continue;
                }
                if let Some(p) = ready.take() {
                    p.close();
                }
                match prepare(w) {
                    Ok(p) => ready = Some(p),
                    Err(e) => log::debug!("window capture prepare failed: {e}"),
                }
            }
            Job::Shoot { window, reply } => {
                let p = match ready.take() {
                    Some(p) if p.window == window => Ok(p),
                    other => {
                        if let Some(p) = other {
                            p.close();
                        }
                        prepare(window)
                    }
                };
                let _ = reply.send(p.and_then(shoot));
            }
        }
    }
}

/// 패키지 없이 실행하면 Allowed(4)였다(spikes §1). 다르면 노란 테두리가 그려질 수 있어 한 번 남긴다.
fn log_border_access() {
    use windows::Security::Authorization::AppCapabilityAccess::AppCapability;
    match AppCapability::Create(&HSTRING::from("graphicsCaptureWithoutBorder")).and_then(|c| c.CheckAccess()) {
        Ok(s) if s.0 == 4 => {}
        Ok(s) => log::info!("borderless capture access status {}", s.0),
        Err(e) => log::info!("borderless capture access unknown ({:#010x})", e.code().0),
    }
}

fn prepare(window: u64) -> Result<Prepared, String> {
    let gpu = gpu::shared()?;
    let err = |what: &str, e: windows::core::Error| format!("{what}: {:#010x}", e.code().0);
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(|e| err("interop", e))?;
    // SAFETY: 창 핸들 하나로 캡처 항목을 만든다(창이 사라졌으면 오류).
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd(window)) }.map_err(|e| err("CreateForWindow", e))?;
    let size = item.Size().map_err(|e| err("Size", e))?;
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&gpu.winrt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size).map_err(|e| err("FramePool", e))?;
    let session = pool.CreateCaptureSession(&item).map_err(|e| err("Session", e))?;
    // Win10 2104(20348)+. 실패해도 계속(테두리가 생길 수 있음 — 위 접근 상태 로그로 진단)
    let _ = session.SetIsBorderRequired(false);
    let _ = session.SetIsCursorCaptureEnabled(false);
    Ok(Prepared { window, gpu, pool, session })
}

fn shoot(p: Prepared) -> Result<WindowShot, String> {
    let h = hwnd(p.window);
    let t = Instant::now();
    let started = p.session.StartCapture();
    let frame = match started {
        Ok(()) => first_frame(&p.pool, Duration::from_millis(FIRST_FRAME_MS)),
        Err(e) => {
            log::info!("StartCapture failed: {:#010x}", e.code().0);
            None
        }
    };
    let waited = t.elapsed();
    let pixels = match frame.as_ref().map(|f| read_frame(&p.gpu, f, h)) {
        Some(Ok(px)) => Some(px),
        Some(Err(e)) => {
            log::info!("window frame read failed: {e}");
            None
        }
        None => None,
    };
    if let Some(f) = frame {
        let _ = f.Close();
    }
    p.close();
    let affinity = display_affinity(h);
    let kind = pixels.as_ref().map(|(_, _, px)| blankness(px));
    log::debug!("wgc window first frame {:.1} ms, {:?}, affinity {affinity}", waited.as_secs_f64() * 1000.0, kind);
    match (windowshot::after_primary(kind, affinity), pixels) {
        (ShotNext::Protected, _) => Err(format!("{WINDOW_PROTECTED}: display affinity {affinity}")),
        (_, Some((w, hgt, bgra))) => Ok(WindowShot { width: w, height: hgt, bgra, method: "wgc", corner_radius: corner_radius(h) }),
        (_, None) => Err(format!("no first frame in {FIRST_FRAME_MS} ms")),
    }
}

fn first_frame(pool: &Direct3D11CaptureFramePool, limit: Duration) -> Option<Direct3D11CaptureFrame> {
    let deadline = Instant::now() + limit;
    loop {
        if let Ok(f) = pool.TryGetNextFrame() {
            return Some(f);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_micros(500));
    }
}

/// 창 바깥 보이지 않는 크기 조절 테두리를 뺀 자리(프레임이 GetWindowRect 크기면 DWM 경계로 자른다).
fn visible_part(h: HWND, cw: u32, ch: u32) -> (u32, u32, u32, u32) {
    let mut wr = RECT::default();
    let mut dr = RECT::default();
    // SAFETY: 읽기 전용 질의, RECT 크기를 넘긴다.
    let ok = unsafe { GetWindowRect(h, &mut wr).is_ok() && DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, &mut dr as *mut RECT as *mut c_void, std::mem::size_of::<RECT>() as u32).is_ok() };
    if ok {
        let (ww, wh) = ((wr.right - wr.left) as u32, (wr.bottom - wr.top) as u32);
        let (dw, dh) = ((dr.right - dr.left).max(0) as u32, (dr.bottom - dr.top).max(0) as u32);
        let (ox, oy) = ((dr.left - wr.left).max(0) as u32, (dr.top - wr.top).max(0) as u32);
        if (cw, ch) == (ww, wh) && dw > 0 && dh > 0 && (dw < ww || dh < wh) && ox + dw <= cw && oy + dh <= ch {
            return (ox, oy, dw, dh);
        }
    }
    (0, 0, cw, ch)
}

/// 프레임 → 스테이징 복사 → BGRA(위→아래, 줄 사이 틈 없이).
fn read_frame(gpu: &Gpu, f: &Direct3D11CaptureFrame, h: HWND) -> Result<(u32, u32, Vec<u8>), String> {
    let e = |what: &str, e: windows::core::Error| format!("{what}: {:#010x}", e.code().0);
    let cs = f.ContentSize().map_err(|x| e("ContentSize", x))?;
    let surface = f.Surface().map_err(|x| e("Surface", x))?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(|x| e("cast", x))?;
    // SAFETY: 프레임이 살아 있는 동안 그 텍스처를 빌린다.
    let tex: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(|x| e("GetInterface", x))?;
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: 출력 구조체 하나.
    unsafe { tex.GetDesc(&mut desc) };
    let cw = (cs.Width.max(0) as u32).min(desc.Width);
    let ch = (cs.Height.max(0) as u32).min(desc.Height);
    if cw == 0 || ch == 0 {
        return Err("empty frame".into());
    }
    let (ox, oy, w, hgt) = visible_part(h, cw, ch);
    let staging_desc = D3D11_TEXTURE2D_DESC {
        Width: w,
        Height: hgt,
        MipLevels: 1,
        ArraySize: 1,
        Format: desc.Format,
        SampleDesc: desc.SampleDesc,
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: 0,
    };
    let mut staging = None;
    // SAFETY: 설명대로 텍스처를 만들고, 프레임 텍스처의 상자 하나를 복사한 뒤 읽기로 연다.
    unsafe {
        gpu.device.CreateTexture2D(&staging_desc, None, Some(&mut staging)).map_err(|x| e("CreateTexture2D", x))?;
        let staging = staging.ok_or("staging texture missing")?;
        let bx = D3D11_BOX { left: ox, top: oy, front: 0, right: ox + w, bottom: oy + hgt, back: 1 };
        gpu.context.CopySubresourceRegion(&staging, 0, 0, 0, 0, &tex, 0, Some(&bx));
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        gpu.context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut m)).map_err(|x| e("Map", x))?;
        let row = w as usize * 4;
        let mut out = vec![0u8; row * hgt as usize];
        for y in 0..hgt as usize {
            let src = std::slice::from_raw_parts((m.pData as *const u8).add(y * m.RowPitch as usize), row);
            out[y * row..(y + 1) * row].copy_from_slice(src);
        }
        gpu.context.Unmap(&staging, 0);
        Ok((w, hgt, out))
    }
}

/// `GetWindowDisplayAffinity`(남의 창도 읽힌다, spikes §1). 못 읽으면 0.
fn display_affinity(h: HWND) -> u32 {
    let mut v = 0u32;
    // SAFETY: 출력 DWORD 하나.
    if unsafe { GetWindowDisplayAffinity(h, &mut v) }.is_ok() {
        v
    } else {
        0
    }
}

/// Win11 둥근 모서리 반지름(물리 px). Win10은 속성을 못 읽어 0.
fn corner_radius(h: HWND) -> u32 {
    let mut pref = 0u32;
    // SAFETY: 출력 DWORD 하나, 크기를 넘긴다.
    let read = unsafe { DwmGetWindowAttribute(h, DWMWA_WINDOW_CORNER_PREFERENCE, &mut pref as *mut u32 as *mut c_void, 4) }.is_ok();
    // SAFETY: 읽기 전용 질의.
    let (maximized, dpi) = unsafe { (IsZoomed(h).as_bool(), GetDpiForWindow(h)) };
    windowshot::corner_radius(read.then_some(pref), maximized, if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ink_domain::image::Blankness;

    #[test]
    #[ignore = "실제 화면 필요 — 메인이 실행"]
    fn 지금_활성_창을_wgc로_한_장() {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        // SAFETY: 읽기 전용.
        let fg = unsafe { GetForegroundWindow() };
        let mut s = WgcScreen::new();
        s.prepare_window(Some(fg.0 as usize as u64));
        let t = Instant::now();
        let shot = s.capture_window(fg.0 as usize as u64, PhysRect::default());
        eprintln!("capture_window {:?} in {:?}", shot.as_ref().map(|s| (s.width, s.height, s.method, s.corner_radius)), t.elapsed());
        assert!(shot.is_ok() || shot.as_ref().err().is_some_and(|e| e.starts_with(WINDOW_PROTECTED)));
        if let Ok(s) = shot {
            assert_ne!(blankness(&s.bgra), Blankness::Unpainted);
        }
    }
}
