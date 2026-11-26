//! F3 녹화 어댑터(단계 6.3·6.5, spikes-windows §2): windows-capture 없이 `windows` 0.62로
//! WGC(모니터·창 추적) → 우리 텍스처 링(자르기·창 크기 변화는 VideoProcessor로 줄여 가운데) → Media Foundation
//! `IMFSinkWriter`(D3D 관리자 + 하드웨어 변환, H.264 MP4).
//!
//! - WGC 프레임 풀 텍스처를 그대로 넘기지 않는다: `WriteSample`이 비동기라 풀이 그 텍스처를 다시 쓴다(함정 4) → 링에 복사.
//! - 비트레이트는 `MF_MT_AVG_BITRATE`만으로 안 지켜졌다(8 → 13~14 Mbps, 함정 3) → 인코더 속성(`CODECAPI_*`)을 함께 준다.
//! - 시작 1초 안 중지도 손상 0(R-REC-1): 프레임이 없으면 400 ms까지 첫 프레임을 기다리고, 그래도 없으면 검은 장면 한 장.
//! - 화면이 그대로면 WGC는 프레임을 주지 않는다 → 1초마다·멈출 때 마지막 장면을 다시 넣어 영상 길이 = 실제 길이.
//! - 임시 파일(`%TEMP%\shookshok\rec-<id>.mp4`) → 마무리 뒤 저장 폴더로 옮김(다른 드라이브면 복사 → 크기 확인 → 지움).
//! - 로그에 창 제목·파일 이름을 쓰지 않는다(R-LOG-1).

use crate::gpu::{self, Gpu};
use ink_app::{RecStartOpts, RecStats, Recorder, SavedFile, REC_ENCODER_FAILED, REC_WINDOW_CLOSED};
use ink_domain::naming;
use ink_domain::recording::{self, Leftover, RecTarget};
use std::ffi::c_void;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows::core::{IInspectable, Interface, GUID, HSTRING};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_RATIONAL, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

/// 우리 텍스처 링 크기(spikes §2 함정 4).
const RING: usize = 8;
/// 고르기 화면(캡처 제외가 아닌 정지 화면·오버레이)을 숨긴 뒤 첫 프레임까지 최소 간격.
const SETTLE: Duration = Duration::from_millis(80);
/// 시작 1초 안 중지: 첫 프레임을 기다리는 최대 시간(시안 E §3).
const FIRST_FRAME_WAIT: Duration = Duration::from_millis(400);
/// 화면이 그대로일 때 마지막 장면을 다시 넣는 간격.
const REPEAT_EVERY: Duration = Duration::from_secs(1);
/// `start`가 캡처 준비(item·pool·session)를 기다리는 시간.
const START_TIMEOUT: Duration = Duration::from_secs(3);

fn hr(what: &str) -> impl Fn(windows::core::Error) -> String + '_ {
    move |e| format!("{what}: {:#010x}", e.code().0)
}

/// MF는 프로세스에 한 번 시작하고 끄지 않는다(시작 비용을 녹화마다 내지 않게).
fn ensure_mf() -> Result<(), String> {
    static MF: OnceLock<Result<(), String>> = OnceLock::new();
    MF.get_or_init(|| {
        // SAFETY: 프로세스 수명 동안 한 번.
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) }.map_err(hr("MFStartup"))
    })
    .clone()
}

#[derive(Default)]
struct Shared {
    frames: AtomicU64,
    width: AtomicU32,
    height: AtomicU32,
    t0: Mutex<Option<Instant>>,
    failure: Mutex<Option<String>>,
}

struct Live {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<RecStats, String>>>,
    temp: PathBuf,
}

/// `Recorder` 구현(Windows). `temp_dir` = `%TEMP%\shookshok`, `default_folder` = 저장 폴더가 비었을 때(사진\shookshok).
pub struct WinRecorder {
    temp_dir: PathBuf,
    default_folder: PathBuf,
    live: Option<Live>,
    shared: Arc<Shared>,
}

impl WinRecorder {
    pub fn new(temp_dir: PathBuf, default_folder: PathBuf) -> Self {
        WinRecorder { temp_dir, default_folder, live: None, shared: Arc::new(Shared::default()) }
    }

    fn halt(&mut self) -> Option<(PathBuf, Result<RecStats, String>)> {
        let mut live = self.live.take()?;
        live.stop.store(true, Ordering::SeqCst);
        let r = match live.thread.take() {
            Some(t) => t.join().unwrap_or_else(|_| Err("recorder thread panicked".into())),
            None => Err("recorder thread missing".into()),
        };
        Some((live.temp, r))
    }
}

impl Recorder for WinRecorder {
    /// F3 고르기 중: MF·장치·하드웨어 인코더를 미리 깨운다(준비 0.35~0.7 s를 숨김, 함정 6).
    fn prepare(&mut self) {
        static WARMED: AtomicBool = AtomicBool::new(false);
        if WARMED.swap(true, Ordering::SeqCst) {
            return;
        }
        let dir = self.temp_dir.clone();
        let _ = std::thread::Builder::new().name("shookshok-rec-warm".into()).spawn(move || {
            let t = Instant::now();
            // SAFETY: 이 스레드를 MTA로.
            unsafe {
                let _ = RoInitialize(RO_INIT_MULTITHREADED);
            }
            let warm = || -> Result<(), String> {
                ensure_mf()?;
                let gpu = gpu::shared()?;
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let path = dir.join(format!("warm-{}.mp4", std::process::id()));
                // 싱크 라이터 + 입력 형식까지(인코더 MFT를 실제로 띄움) 만들고 바로 버린다
                let r = Encoder::open(&gpu, &path, 1920, 1080, 30, recording::bitrate(1920, 1080, 30), false).map(drop);
                let _ = std::fs::remove_file(&path);
                r
            };
            match warm() {
                Ok(()) => log::info!("recorder warmed in {:.0} ms", t.elapsed().as_secs_f64() * 1000.0),
                Err(e) => log::info!("recorder warm-up failed: {e}"),
            }
        });
    }

    fn start(&mut self, opts: RecStartOpts) -> Result<(), String> {
        if self.live.is_some() {
            return Err("already recording".into());
        }
        if let Some(dir) = opts.temp_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("temp dir: {e}"))?;
        }
        let shared = Arc::new(Shared::default());
        self.shared = shared.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = channel();
        let temp = opts.temp_path.clone();
        let (stop2, shared2) = (stop.clone(), shared.clone());
        let thread = std::thread::Builder::new().name("shookshok-rec".into()).spawn(move || run(opts, stop2, shared2, ready_tx)).map_err(|e| e.to_string())?;
        match ready_rx.recv_timeout(START_TIMEOUT) {
            Ok(Ok(())) => {
                self.live = Some(Live { stop, thread: Some(thread), temp });
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                Err("capture did not start in time".into())
            }
        }
    }

    fn stats(&self) -> RecStats {
        let s = &self.shared;
        let elapsed_ms = s.t0.lock().unwrap_or_else(|e| e.into_inner()).map(|t| t.elapsed().as_millis() as u64).unwrap_or(0);
        let bytes = self.live.as_ref().and_then(|l| std::fs::metadata(&l.temp).ok()).map(|m| m.len()).unwrap_or(0);
        RecStats { elapsed_ms, bytes, frames: s.frames.load(Ordering::Relaxed), width: s.width.load(Ordering::Relaxed), height: s.height.load(Ordering::Relaxed) }
    }

    fn stop(&mut self) -> Result<RecStats, String> {
        let (temp, r) = self.halt().ok_or("not recording")?;
        let mut stats = r?;
        stats.bytes = std::fs::metadata(&temp).map(|m| m.len()).unwrap_or(0);
        Ok(stats)
    }

    fn abort(&mut self) {
        if let Some((temp, _)) = self.halt() {
            let _ = std::fs::remove_file(&temp);
        }
    }

    fn take_failure(&mut self) -> Option<String> {
        self.shared.failure.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    fn temp_dir(&self) -> Option<PathBuf> {
        Some(self.temp_dir.clone())
    }

    fn leftovers(&self) -> Vec<Leftover> {
        let active = self.live.as_ref().map(|l| l.temp.clone());
        scan_leftovers(&self.temp_dir, active.as_deref())
    }

    fn free_bytes(&self, folder: &str) -> Option<u64> {
        let dir = if folder.trim().is_empty() { self.default_folder.clone() } else { PathBuf::from(folder) };
        free_bytes(&dir)
    }

    fn keep_file(&mut self, temp: &Path, folder: &str, stem: &str) -> Result<SavedFile, String> {
        keep_file(temp, folder, stem, &self.default_folder)
    }

    fn delete_file(&mut self, temp: &Path) -> Result<(), String> {
        std::fs::remove_file(temp).map_err(|e| e.to_string())
    }
}

impl Drop for WinRecorder {
    fn drop(&mut self) {
        // 앱이 끝날 때 녹화 중이면 정상 마무리(임시 파일은 다음 시작 때 복구 제안)
        let _ = self.halt();
    }
}

// ───────────── 녹화 스레드 ─────────────

fn run(opts: RecStartOpts, stop: Arc<AtomicBool>, shared: Arc<Shared>, ready: Sender<Result<(), String>>) -> Result<RecStats, String> {
    // SAFETY: 이 스레드를 MTA로 — WGC·MF 객체는 이 스레드에서만 만든다.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let began = Instant::now();
    let src = match Source::open(&opts) {
        Ok(s) => s,
        Err(e) => {
            let _ = ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    let _ = ready.send(Ok(()));
    let result = record(&opts, &src, &stop, &shared, began);
    src.close();
    if let Err(e) = &result {
        let mut f = shared.failure.lock().unwrap_or_else(|e| e.into_inner());
        if f.is_none() && !stop.load(Ordering::SeqCst) {
            *f = Some(format!("{REC_ENCODER_FAILED}: {e}"));
        }
    }
    result
}

/// WGC 쪽: 항목·풀·세션 + 창 닫힘·프레임 도착 신호.
struct Source {
    gpu: Arc<Gpu>,
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    closed: Arc<AtomicBool>,
    arrived: Receiver<()>,
    size: SizeInt32,
    /// 모니터·범위 녹화: 모니터 텍스처에서 자를 상자(x, y, w, h). 창 추적이면 `None`
    crop: Option<(u32, u32, u32, u32)>,
    /// 영상 크기(짝수)
    video: (u32, u32),
}

impl Source {
    fn open(opts: &RecStartOpts) -> Result<Source, String> {
        let gpu = gpu::shared()?;
        if !gpu.video {
            return Err("d3d11 device has no video support".into());
        }
        ensure_mf()?;
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(hr("interop"))?;
        let (item, crop): (GraphicsCaptureItem, Option<PhysBox>) = match &opts.target {
            RecTarget::Window { handle, .. } => {
                // SAFETY: 창 핸들 하나로 캡처 항목(창이 사라졌으면 오류).
                let item = unsafe { interop.CreateForWindow(HWND(*handle as usize as *mut c_void)) }.map_err(hr("CreateForWindow"))?;
                (item, None)
            }
            RecTarget::Monitor { rect, .. } | RecTarget::Region { rect, .. } => {
                let center = POINT { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
                // SAFETY: 읽기 전용 질의, 구조체 크기를 채운다.
                let (hmon, origin) = unsafe {
                    let hmon = MonitorFromPoint(center, MONITOR_DEFAULTTONEAREST);
                    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                    let _ = GetMonitorInfoW(hmon, &mut mi);
                    (hmon, (mi.rcMonitor.left, mi.rcMonitor.top))
                };
                // SAFETY: 모니터 핸들 하나로 캡처 항목.
                let item = unsafe { interop.CreateForMonitor(hmon) }.map_err(hr("CreateForMonitor"))?;
                (item, Some(PhysBox { x: rect.x - origin.0, y: rect.y - origin.1, w: rect.w, h: rect.h }))
            }
        };
        let size = item.Size().map_err(hr("Size"))?;
        let (crop, video) = match crop {
            Some(b) => {
                let x = b.x.clamp(0, size.Width.max(0)) as u32 & !1;
                let y = b.y.clamp(0, size.Height.max(0)) as u32 & !1;
                let (w, h) = recording::even_size(b.w.max(0) as u32, b.h.max(0) as u32);
                let w = w.min((size.Width.max(0) as u32).saturating_sub(x) & !1);
                let h = h.min((size.Height.max(0) as u32).saturating_sub(y) & !1);
                (Some((x, y, w, h)), (w, h))
            }
            None => (None, recording::even_size(size.Width.max(0) as u32, size.Height.max(0) as u32)),
        };
        if video.0 < 16 || video.1 < 16 {
            return Err("capture area too small".into());
        }
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&gpu.winrt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 3, size).map_err(hr("FramePool"))?;
        let session = pool.CreateCaptureSession(&item).map_err(hr("Session"))?;
        let _ = session.SetIsBorderRequired(false);
        let _ = session.SetIsCursorCaptureEnabled(opts.capture_cursor);
        // Win11: 초당 장면 수 위로는 DWM이 프레임을 만들지 않게(없는 OS면 무시)
        let _ = session.SetMinUpdateInterval(windows::Foundation::TimeSpan { Duration: 10_000_000 / opts.fps.max(1) as i64 });
        let closed = Arc::new(AtomicBool::new(false));
        let flag = closed.clone();
        let _ = item.Closed(&TypedEventHandler::<GraphicsCaptureItem, IInspectable>::new(move |_, _| {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        }));
        let (tx, arrived) = sync_channel::<()>(1);
        let _ = pool.FrameArrived(&TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |_, _| {
            let _ = tx.try_send(());
            Ok(())
        }));
        // 항목은 세션이 쥐고 있다(Closed 처리기도 그 수명 동안)
        drop(item);
        Ok(Source { gpu, pool, session, closed, arrived, size, crop, video })
    }

    fn close(&self) {
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}

#[derive(Clone, Copy)]
struct PhysBox {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn frame_texture(f: &Direct3D11CaptureFrame) -> Result<ID3D11Texture2D, String> {
    let surface = f.Surface().map_err(hr("Surface"))?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(hr("cast"))?;
    // SAFETY: 프레임이 살아 있는 동안 그 텍스처를 빌린다.
    unsafe { access.GetInterface() }.map_err(hr("GetInterface"))
}

fn record(opts: &RecStartOpts, src: &Source, stop: &AtomicBool, shared: &Shared, began: Instant) -> Result<RecStats, String> {
    let (vw, vh) = src.video;
    let t = Instant::now();
    let enc = Encoder::open(&src.gpu, &opts.temp_path, vw, vh, opts.fps, opts.bitrate, true)?;
    log::info!("encoder ready in {:.0} ms ({vw}x{vh} {}fps, {} kbps)", t.elapsed().as_secs_f64() * 1000.0, opts.fps, opts.bitrate / 1000);
    shared.width.store(vw, Ordering::Relaxed);
    shared.height.store(vh, Ordering::Relaxed);
    let ring = Ring::new(&src.gpu, vw, vh)?;
    let mut fit = Fitter::default();
    if let Some(left) = SETTLE.checked_sub(began.elapsed()) {
        std::thread::sleep(left);
    }
    src.session.StartCapture().map_err(hr("StartCapture"))?;
    let t0 = Instant::now();
    *shared.t0.lock().unwrap_or_else(|e| e.into_inner()) = Some(t0);
    let dur = 10_000_000i64 / opts.fps.max(1) as i64;
    let ts_now = || (t0.elapsed().as_nanos() / 100) as i64;
    let mut size = src.size;
    let mut slot = 0usize;
    let mut last: Option<(usize, i64, Instant)> = None;
    let mut frames = 0u64;
    let write = |slot: usize, ts: i64, last: &mut Option<(usize, i64, Instant)>, frames: &mut u64| -> Result<(), String> {
        // 표본 시각은 늘 앞으로만
        let ts = last.map_or(ts, |(_, l, _)| ts.max(l + 1));
        enc.write(&ring.tex[slot], ts, dur)?;
        *last = Some((slot, ts, Instant::now()));
        *frames += 1;
        shared.frames.store(*frames, Ordering::Relaxed);
        Ok(())
    };

    let mut grab = |f: Direct3D11CaptureFrame, slot: usize, size: &mut SizeInt32| -> Result<(), String> {
        let cs = f.ContentSize().map_err(hr("ContentSize"))?;
        let tex = frame_texture(&f)?;
        fit.copy(&src.gpu, &ring, slot, &tex, src.crop, (cs.Width.max(0) as u32, cs.Height.max(0) as u32), opts.fps);
        let _ = f.Close();
        // 창 크기가 바뀌면 풀도 따라간다(영상 크기는 그대로, 줄여서 가운데)
        if src.crop.is_none() && (cs.Width, cs.Height) != (size.Width, size.Height) && cs.Width > 0 && cs.Height > 0 {
            *size = cs;
            let _ = src.pool.Recreate(&src.gpu.winrt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 3, cs);
        }
        Ok(())
    };

    while !stop.load(Ordering::SeqCst) {
        if src.closed.load(Ordering::SeqCst) {
            *shared.failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(REC_WINDOW_CLOSED.into());
            break;
        }
        let _ = src.arrived.recv_timeout(Duration::from_millis(100));
        // 밀린 프레임은 버리고 가장 새것만
        let mut newest: Option<Direct3D11CaptureFrame> = None;
        while let Ok(f) = src.pool.TryGetNextFrame() {
            if let Some(old) = newest.replace(f) {
                let _ = old.Close();
            }
        }
        let ts = ts_now();
        let Some(frame) = newest else {
            if let Some((s, _, at)) = last {
                if at.elapsed() >= REPEAT_EVERY {
                    write(s, ts, &mut last, &mut frames)?;
                }
            }
            continue;
        };
        // 초당 장면 수 맞춤(모니터는 60 Hz로 올 수 있다)
        if last.is_some_and(|(_, l, _)| ts - l < dur * 9 / 10) {
            let _ = frame.Close();
            continue;
        }
        grab(frame, slot, &mut size)?;
        write(slot, ts, &mut last, &mut frames)?;
        slot = (slot + 1) % RING;
    }

    // 시작 1초 안 중지: 첫 프레임을 조금 기다리고, 없으면 검은 장면(링은 검정으로 시작)
    if frames == 0 {
        let deadline = Instant::now() + FIRST_FRAME_WAIT;
        while Instant::now() < deadline {
            if let Ok(f) = src.pool.TryGetNextFrame() {
                grab(f, 0, &mut size)?;
                break;
            }
            let _ = src.arrived.recv_timeout(Duration::from_millis(20));
        }
        write(0, 0, &mut last, &mut frames)?;
    }
    // 끝 장면을 멈춘 순간에 한 번 더 — 정지 화면이 길었어도 영상 길이 = 실제 길이
    if let Some((s, _, _)) = last {
        write(s, ts_now(), &mut last, &mut frames)?;
    }
    enc.finalize()?;
    Ok(RecStats { elapsed_ms: t0.elapsed().as_millis() as u64, bytes: 0, frames, width: vw, height: vh })
}

// ───────────── 텍스처 링 · 맞춤 ─────────────

pub(crate) struct Ring {
    pub(crate) tex: Vec<ID3D11Texture2D>,
    rtv: Vec<ID3D11RenderTargetView>,
    w: u32,
    h: u32,
}

impl Ring {
    /// 검정으로 채운 BGRA 텍스처 `RING`장(렌더 대상 — 지우기·VideoProcessor 출력).
    pub(crate) fn new(gpu: &Gpu, w: u32, h: u32) -> Result<Ring, String> {
        Self::with_pixels(gpu, w, h, &[])
    }

    /// 시험용: 장마다 처음 화소(없으면 검정).
    pub(crate) fn with_pixels(gpu: &Gpu, w: u32, h: u32, pixels: &[Vec<u8>]) -> Result<Ring, String> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: w,
            Height: h,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let black: Vec<u8> = (0..w as usize * h as usize).flat_map(|_| [0u8, 0, 0, 255]).collect();
        let (mut tex, mut rtv) = (Vec::new(), Vec::new());
        let n = RING.max(pixels.len());
        for i in 0..n {
            let px = pixels.get(i).map(|p| p.as_slice()).unwrap_or(&black);
            let init = D3D11_SUBRESOURCE_DATA { pSysMem: px.as_ptr() as *const c_void, SysMemPitch: w * 4, SysMemSlicePitch: 0 };
            let mut t = None;
            let mut v = None;
            // SAFETY: 설명·처음 화소(w×h×4)를 넘겨 텍스처와 그 렌더 대상 보기를 만든다.
            unsafe {
                gpu.device.CreateTexture2D(&desc, Some(&init), Some(&mut t)).map_err(hr("ring texture"))?;
                let t = t.ok_or("ring texture missing")?;
                gpu.device.CreateRenderTargetView(&t, None, Some(&mut v)).map_err(hr("ring view"))?;
                tex.push(t);
                rtv.push(v.ok_or("ring view missing")?);
            }
        }
        Ok(Ring { tex, rtv, w, h })
    }
}

/// 프레임 → 링 한 장: 모니터는 상자 자르기, 창은 같은 크기면 복사, 다르면 VideoProcessor로 줄여 가운데(검은 바탕).
#[derive(Default)]
struct Fitter {
    scaler: Option<Scaler>,
    /// VideoProcessor가 안 되면(드라이버) 왼쪽 위 자르기로 — 한 번만 알린다
    no_scaler: bool,
}

impl Fitter {
    #[allow(clippy::too_many_arguments)]
    fn copy(&mut self, gpu: &Gpu, ring: &Ring, slot: usize, src: &ID3D11Texture2D, crop: Option<(u32, u32, u32, u32)>, content: (u32, u32), fps: u32) {
        let (vw, vh) = (ring.w, ring.h);
        let mut sd = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: 출력 구조체 하나.
        unsafe { src.GetDesc(&mut sd) };
        let (cw, ch) = (content.0.min(sd.Width), content.1.min(sd.Height));
        let dst = &ring.tex[slot];
        let boxed = |x: u32, y: u32, w: u32, h: u32| D3D11_BOX { left: x, top: y, front: 0, right: x + w, bottom: y + h, back: 1 };
        // SAFETY: 두 텍스처 모두 같은 장치·같은 형식, 상자는 원본 안으로 가뒀다.
        unsafe {
            if let Some((x, y, w, h)) = crop {
                let w = w.min(sd.Width.saturating_sub(x)).min(vw);
                let h = h.min(sd.Height.saturating_sub(y)).min(vh);
                gpu.context.CopySubresourceRegion(dst, 0, 0, 0, 0, src, 0, Some(&boxed(x, y, w, h)));
                return;
            }
            // 짝수로 내린 1 px 차이는 그냥 자른다
            if cw >= vw && ch >= vh && cw - vw <= 1 && ch - vh <= 1 {
                gpu.context.CopySubresourceRegion(dst, 0, 0, 0, 0, src, 0, Some(&boxed(0, 0, vw, vh)));
                return;
            }
            let fit = recording::fit_inside(cw, ch, vw, vh);
            gpu.context.ClearRenderTargetView(&ring.rtv[slot], &[0.0, 0.0, 0.0, 1.0]);
            if !self.no_scaler {
                let ok = self.scaler_for(gpu, (cw, ch), (vw, vh), fps).and_then(|s| s.blt(src, (cw, ch), dst, fit));
                match ok {
                    Ok(()) => return,
                    Err(e) => {
                        log::info!("video processor unavailable ({e}), cropping instead");
                        self.no_scaler = true;
                    }
                }
            }
            let (w, h) = (cw.min(vw), ch.min(vh));
            let (dx, dy) = ((vw - w) / 2, (vh - h) / 2);
            gpu.context.CopySubresourceRegion(dst, 0, dx, dy, 0, src, 0, Some(&boxed(0, 0, w, h)));
        }
    }

    fn scaler_for(&mut self, gpu: &Gpu, input: (u32, u32), output: (u32, u32), fps: u32) -> Result<&Scaler, String> {
        if self.scaler.as_ref().map(|s| (s.input, s.output)) != Some((input, output)) {
            self.scaler = Some(Scaler::new(gpu, input, output, fps)?);
        }
        self.scaler.as_ref().ok_or_else(|| "scaler missing".to_string())
    }
}

/// D3D11 VideoProcessor(BGRA → BGRA 크기 바꾸기, GPU).
struct Scaler {
    vdev: ID3D11VideoDevice,
    vctx: ID3D11VideoContext,
    en: ID3D11VideoProcessorEnumerator,
    vp: ID3D11VideoProcessor,
    input: (u32, u32),
    output: (u32, u32),
}

impl Scaler {
    fn new(gpu: &Gpu, input: (u32, u32), output: (u32, u32), fps: u32) -> Result<Scaler, String> {
        let vdev: ID3D11VideoDevice = gpu.device.cast().map_err(hr("video device"))?;
        let vctx: ID3D11VideoContext = gpu.context.cast().map_err(hr("video context"))?;
        let rate = DXGI_RATIONAL { Numerator: fps.max(1), Denominator: 1 };
        let desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
            InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            InputFrameRate: rate,
            InputWidth: input.0,
            InputHeight: input.1,
            OutputFrameRate: rate,
            OutputWidth: output.0,
            OutputHeight: output.1,
            Usage: D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
        };
        // SAFETY: 설명대로 열거자·처리기를 만든다.
        unsafe {
            let en = vdev.CreateVideoProcessorEnumerator(&desc).map_err(hr("VP enumerator"))?;
            let vp = vdev.CreateVideoProcessor(&en, 0).map_err(hr("VP"))?;
            Ok(Scaler { vdev, vctx, en, vp, input, output })
        }
    }

    fn blt(&self, src: &ID3D11Texture2D, content: (u32, u32), dst: &ID3D11Texture2D, fit: ink_domain::geometry::PhysRect) -> Result<(), String> {
        let in_desc = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
            FourCC: 0,
            ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_VPIV { MipSlice: 0, ArraySlice: 0 } },
        };
        let out_desc = D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
            ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_VPOV { MipSlice: 0 } },
        };
        let src_rect = RECT { left: 0, top: 0, right: content.0 as i32, bottom: content.1 as i32 };
        let dst_rect = RECT { left: fit.x, top: fit.y, right: fit.x + fit.w, bottom: fit.y + fit.h };
        let out_rect = RECT { left: 0, top: 0, right: self.output.0 as i32, bottom: self.output.1 as i32 };
        // SAFETY: 보기는 이 호출 안에서만 쓰고, 처리기·장치·텍스처는 모두 같은 장치 것이다.
        unsafe {
            let mut iv = None;
            self.vdev.CreateVideoProcessorInputView(src, &self.en, &in_desc, Some(&mut iv)).map_err(hr("VP input view"))?;
            let mut ov = None;
            self.vdev.CreateVideoProcessorOutputView(dst, &self.en, &out_desc, Some(&mut ov)).map_err(hr("VP output view"))?;
            let ov = ov.ok_or("VP output view missing")?;
            self.vctx.VideoProcessorSetStreamSourceRect(&self.vp, 0, true, Some(&src_rect));
            self.vctx.VideoProcessorSetStreamDestRect(&self.vp, 0, true, Some(&dst_rect));
            self.vctx.VideoProcessorSetOutputTargetRect(&self.vp, true, Some(&out_rect));
            let stream = D3D11_VIDEO_PROCESSOR_STREAM {
                Enable: true.into(),
                OutputIndex: 0,
                InputFrameOrField: 0,
                PastFrames: 0,
                FutureFrames: 0,
                ppPastSurfaces: std::ptr::null_mut(),
                pInputSurface: std::mem::ManuallyDrop::new(iv),
                ppFutureSurfaces: std::ptr::null_mut(),
                ppPastSurfacesRight: std::ptr::null_mut(),
                pInputSurfaceRight: std::mem::ManuallyDrop::new(None),
                ppFutureSurfacesRight: std::ptr::null_mut(),
            };
            let streams = [stream];
            let r = self.vctx.VideoProcessorBlt(&self.vp, &ov, 0, &streams).map_err(hr("VideoProcessorBlt"));
            let [stream] = streams;
            drop(std::mem::ManuallyDrop::into_inner(stream.pInputSurface));
            r
        }
    }
}

// ───────────── Media Foundation 싱크 라이터 ─────────────

pub(crate) struct Encoder {
    writer: IMFSinkWriter,
    stream: u32,
    _manager: IMFDXGIDeviceManager,
}

// CODECAPI 값(codecapi.h): 속도 조절 방식 = 최대 제한 VBR(평균 = 목표, 최대 = 2배).
const RATE_CONTROL_PEAK_CONSTRAINED_VBR: u32 = 1;
/// H.264 Main 프로필(eAVEncH264VProfile_Main = 77) — 학교 PC 기본 재생기 호환.
const H264_PROFILE_MAIN: u32 = 77;

fn video_type(subtype: &GUID, w: u32, h: u32, fps: u32) -> windows::core::Result<IMFMediaType> {
    // SAFETY: 새 미디어 형식 객체에 값만 넣는다.
    unsafe {
        let t = MFCreateMediaType()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        t.SetGUID(&MF_MT_SUBTYPE, subtype)?;
        t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        t.SetUINT64(&MF_MT_FRAME_SIZE, ((w as u64) << 32) | h as u64)?;
        t.SetUINT64(&MF_MT_FRAME_RATE, ((fps as u64) << 32) | 1)?;
        t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1)?;
        Ok(t)
    }
}

impl Encoder {
    /// 싱크 라이터를 만들고 입력 형식까지 정한다. `begin`이면 쓰기를 시작한다(아니면 미리 깨우기용).
    pub(crate) fn open(gpu: &Gpu, path: &Path, w: u32, h: u32, fps: u32, bitrate: u32, begin: bool) -> Result<Encoder, String> {
        ensure_mf()?;
        // SAFETY: MF 객체를 만들고 값만 넣는다. 장치 관리자는 인코더 수명 동안 쥔다.
        unsafe {
            let mut token = 0u32;
            let mut mgr = None;
            MFCreateDXGIDeviceManager(&mut token, &mut mgr).map_err(hr("DXGI device manager"))?;
            let mgr = mgr.ok_or("device manager missing")?;
            mgr.ResetDevice(&gpu.device, token).map_err(hr("ResetDevice"))?;
            let mut attrs = None;
            MFCreateAttributes(&mut attrs, 4).map_err(hr("attributes"))?;
            let attrs = attrs.ok_or("attributes missing")?;
            attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1).map_err(hr("hw transforms"))?;
            attrs.SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4).map_err(hr("container"))?;
            attrs.SetUnknown(&MF_SINK_WRITER_D3D_MANAGER, &mgr).map_err(hr("d3d manager"))?;
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(path.as_os_str()), None, &attrs).map_err(hr("MFCreateSinkWriterFromURL"))?;
            let out_t = video_type(&MFVideoFormat_H264, w, h, fps).map_err(hr("output type"))?;
            out_t.SetUINT32(&MF_MT_AVG_BITRATE, bitrate).map_err(hr("bitrate"))?;
            let _ = out_t.SetUINT32(&MF_MT_MPEG2_PROFILE, H264_PROFILE_MAIN);
            let stream = writer.AddStream(&out_t).map_err(hr("AddStream"))?;
            let in_t = video_type(&MFVideoFormat_RGB32, w, h, fps).map_err(hr("input type"))?;
            // 양수 보폭 = 위에서 아래
            in_t.SetUINT32(&MF_MT_DEFAULT_STRIDE, w * 4).map_err(hr("stride"))?;
            // 인코더 설정(속도 조절·평균·최대·키 장면 간격 2초) — 인코더가 모르면 무시된다
            let mut params = None;
            MFCreateAttributes(&mut params, 4).map_err(hr("params"))?;
            let params = params.ok_or("params missing")?;
            let _ = params.SetUINT32(&CODECAPI_AVEncCommonRateControlMode, RATE_CONTROL_PEAK_CONSTRAINED_VBR);
            let _ = params.SetUINT32(&CODECAPI_AVEncCommonMeanBitRate, bitrate);
            let _ = params.SetUINT32(&CODECAPI_AVEncCommonMaxBitRate, bitrate.saturating_mul(2));
            let _ = params.SetUINT32(&CODECAPI_AVEncMPVGOPSize, fps * 2);
            writer.SetInputMediaType(stream, &in_t, &params).map_err(hr("SetInputMediaType"))?;
            if begin {
                writer.BeginWriting().map_err(hr("BeginWriting"))?;
            }
            Ok(Encoder { writer, stream, _manager: mgr })
        }
    }

    /// 텍스처 한 장(`ts`·`dur` = 100 ns). 비동기 — 텍스처는 링이 쥐고 있어야 한다.
    pub(crate) fn write(&self, tex: &ID3D11Texture2D, ts: i64, dur: i64) -> Result<(), String> {
        // SAFETY: 살아 있는 텍스처를 감싼 버퍼·표본을 만들어 넘긴다.
        unsafe {
            let buf = MFCreateDXGISurfaceBuffer(&ID3D11Texture2D::IID, tex, 0, false).map_err(hr("surface buffer"))?;
            let len = buf.cast::<IMF2DBuffer>().and_then(|b| b.GetContiguousLength()).map_err(hr("buffer length"))?;
            buf.SetCurrentLength(len).map_err(hr("SetCurrentLength"))?;
            let s = MFCreateSample().map_err(hr("sample"))?;
            s.AddBuffer(&buf).map_err(hr("AddBuffer"))?;
            s.SetSampleTime(ts).map_err(hr("SetSampleTime"))?;
            s.SetSampleDuration(dur).map_err(hr("SetSampleDuration"))?;
            self.writer.WriteSample(self.stream, &s).map_err(hr("WriteSample"))
        }
    }

    /// 인코더 비우기 + moov 기록.
    pub(crate) fn finalize(&self) -> Result<(), String> {
        // SAFETY: 쓰기를 마무리한다(한 번).
        unsafe { self.writer.Finalize() }.map_err(hr("Finalize"))
    }
}

// ───────────── 파일: 남은 녹화 · 옮기기 · 남은 공간 ─────────────

/// 맨 위 MP4 상자 종류(최대 64개) — 재생 가능 판정(`recording::mp4_playable`)용.
pub fn mp4_kinds(path: &Path) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    let Ok(mut f) = std::fs::File::open(path) else { return out };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mut off = 0u64;
    while out.len() < 64 && off + 8 <= len {
        let mut head = [0u8; 16];
        if f.seek(SeekFrom::Start(off)).is_err() {
            break;
        }
        let n = f.read(&mut head).unwrap_or(0);
        let Some((size, kind)) = recording::mp4_box(&head[..n]) else { break };
        out.push(kind);
        if size < 8 {
            break;
        }
        off += size;
    }
    out
}

/// 임시 폴더의 `rec-*.mp4`(지금 녹화 중인 것 빼고).
pub fn scan_leftovers(dir: &Path, active: Option<&Path>) -> Vec<Leftover> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Leftover> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().and_then(recording::parse_temp_name).is_some())
        .map(|e| e.path())
        .filter(|p| Some(p.as_path()) != active)
        .map(|p| {
            let bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            let playable = recording::mp4_playable(&mp4_kinds(&p));
            Leftover { path: p.to_string_lossy().into_owned(), bytes, playable }
        })
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn move_into(temp: &Path, dir: &Path, stem: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let dir_s = dir.to_string_lossy();
    let stem = naming::fit_path(&dir_s, stem, "mp4");
    let name = naming::unique_file_name(&stem, "mp4", |n| dir.join(n).exists());
    let dest = dir.join(name);
    if std::fs::rename(temp, &dest).is_ok() {
        return Ok(dest);
    }
    // 다른 드라이브: 복사 → 크기 확인 → 지움
    std::fs::copy(temp, &dest)?;
    if std::fs::metadata(&dest)?.len() != std::fs::metadata(temp)?.len() {
        let _ = std::fs::remove_file(&dest);
        return Err(std::io::Error::other("copy size mismatch"));
    }
    std::fs::remove_file(temp)?;
    Ok(dest)
}

/// 저장 폴더로(빈 값·쓸 수 없는 폴더면 기본 폴더 + `fell_back`). 오류 문구에 경로를 넣지 않는다.
pub fn keep_file(temp: &Path, folder: &str, stem: &str, default_folder: &Path) -> Result<SavedFile, String> {
    let wanted = (!folder.trim().is_empty()).then(|| PathBuf::from(folder.trim()));
    let (path, fell_back) = match wanted.map(|d| move_into(temp, &d, stem)) {
        Some(Ok(p)) => (p, false),
        Some(Err(e)) => {
            log::warn!("record folder unusable ({e}), using the default folder");
            (move_into(temp, default_folder, stem).map_err(|e| e.to_string())?, true)
        }
        None => (move_into(temp, default_folder, stem).map_err(|e| e.to_string())?, true),
    };
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(SavedFile { name, path: path.to_string_lossy().into_owned(), fell_back })
}

/// 그 폴더(없으면 있는 조상)의 남은 바이트.
pub fn free_bytes(dir: &Path) -> Option<u64> {
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut d = dir;
    while !d.exists() {
        d = d.parent()?;
    }
    let mut free = 0u64;
    // SAFETY: 출력 하나, 경로는 살아 있는 HSTRING.
    unsafe { GetDiskFreeSpaceExW(&HSTRING::from(d.as_os_str()), Some(&mut free), None, None) }.ok()?;
    Some(free)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 화면이 아닌 합성 프레임 — `D:\dev-cache\screen-ink\out\`에만 쓴다(일꾼 규칙).
    fn out_dir() -> PathBuf {
        let d = PathBuf::from(r"D:\dev-cache\screen-ink\out\window-record");
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn synth(w: u32, h: u32, n: u32) -> Vec<u8> {
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let bar = (x + n * 24) % w < 96;
                px[i] = if bar { 255 } else { (40 + ((x / 16 + y / 16 + n) % 2) * 120) as u8 };
                px[i + 1] = (y * 255 / h) as u8;
                px[i + 2] = (x * 255 / w) as u8;
                px[i + 3] = 255;
            }
        }
        px
    }

    #[test]
    fn 녹화기_합성_프레임_3초_mp4는_재생되고_비트레이트_근처() {
        let gpu = gpu::shared().expect("d3d11");
        assert!(gpu.video, "VIDEO_SUPPORT 장치");
        let (w, h, fps) = (1280u32, 720u32, 30u32);
        let bitrate = recording::bitrate(w, h, fps);
        let pixels: Vec<Vec<u8>> = (0..RING as u32).map(|n| synth(w, h, n)).collect();
        let ring = Ring::with_pixels(&gpu, w, h, &pixels).expect("ring");
        let path = out_dir().join("synth-3s.mp4");
        let _ = std::fs::remove_file(&path);
        let t = Instant::now();
        let enc = Encoder::open(&gpu, &path, w, h, fps, bitrate, true).expect("encoder");
        let setup = t.elapsed();
        let dur = 10_000_000 / fps as i64;
        for n in 0..(3 * fps) as usize {
            enc.write(&ring.tex[n % RING], n as i64 * dur, dur).expect("write");
        }
        enc.finalize().expect("finalize");
        let bytes = std::fs::metadata(&path).unwrap().len();
        let mbps = bytes as f64 * 8.0 / 3.0 / 1e6;
        eprintln!("setup {setup:?} · {bytes} bytes · {mbps:.2} Mbps (target {:.2})", bitrate as f64 / 1e6);
        assert!(recording::mp4_playable(&mp4_kinds(&path)), "ftyp … moov");
        assert!(bytes > 50_000, "{bytes}");
        // [확인 필요] 하드웨어 인코더가 목표를 얼마나 지키는지 — 3배를 넘으면 실패로 본다
        assert!(mbps < bitrate as f64 / 1e6 * 3.0, "{mbps:.2} Mbps");
    }

    #[test]
    fn 녹화기_한_장만_쓰고_멈춰도_손상_없음() {
        let gpu = gpu::shared().expect("d3d11");
        let ring = Ring::new(&gpu, 640, 360).expect("ring");
        let path = out_dir().join("synth-one-frame.mp4");
        let _ = std::fs::remove_file(&path);
        let enc = Encoder::open(&gpu, &path, 640, 360, 30, 1_000_000, true).expect("encoder");
        enc.write(&ring.tex[0], 0, 333_333).expect("write");
        enc.write(&ring.tex[0], 3_000_000, 333_333).expect("write last");
        enc.finalize().expect("finalize");
        assert!(recording::mp4_playable(&mp4_kinds(&path)));
    }

    #[test]
    fn 녹화기_남은_녹화_찾기_옮기기_이름_겹침() {
        let root = out_dir().join("files");
        let _ = std::fs::remove_dir_all(&root);
        let temp = root.join("temp");
        std::fs::create_dir_all(&temp).unwrap();
        // 마무리된 것(ftyp + moov) · 강제 종료된 것(moov 없음) · 다른 파일
        let mut good = Vec::new();
        good.extend_from_slice(&[0, 0, 0, 16, b'f', b't', b'y', b'p', 0, 0, 0, 0, 0, 0, 0, 0]);
        good.extend_from_slice(&[0, 0, 0, 12, b'm', b'd', b'a', b't', 1, 2, 3, 4]);
        good.extend_from_slice(&[0, 0, 0, 8, b'm', b'o', b'o', b'v']);
        std::fs::write(temp.join("rec-1.mp4"), &good).unwrap();
        std::fs::write(temp.join("rec-2.mp4"), &good[..28]).unwrap();
        std::fs::write(temp.join("warm-9.mp4"), b"x").unwrap();
        std::fs::write(temp.join("rec-3.mp4"), &good).unwrap();
        let found = scan_leftovers(&temp, Some(&temp.join("rec-3.mp4")));
        assert_eq!(found.len(), 2, "rec-* 만, 녹화 중인 것 빼고");
        assert!(found[0].playable && !found[1].playable);
        assert_eq!(found[0].bytes, good.len() as u64);

        let out = root.join("videos");
        let a = keep_file(&temp.join("rec-1.mp4"), &out.to_string_lossy(), "shookshok_x", &root.join("default")).unwrap();
        assert_eq!((a.name.as_str(), a.fell_back), ("shookshok_x.mp4", false));
        let b = keep_file(&temp.join("rec-2.mp4"), &out.to_string_lossy(), "shookshok_x", &root.join("default")).unwrap();
        assert_eq!(b.name, "shookshok_x (2).mp4", "같은 이름이면 (2)");
        assert!(!temp.join("rec-1.mp4").exists() && !temp.join("rec-2.mp4").exists());
        let c = keep_file(&temp.join("rec-3.mp4"), "", "shookshok_y", &root.join("default")).unwrap();
        assert!(c.fell_back, "빈 폴더 = 기본 폴더");
        assert!(root.join("default").join("shookshok_y.mp4").exists());
        assert!(free_bytes(&root.join("없는").join("폴더")).is_some_and(|b| b > 0), "있는 조상 폴더로");
    }

    #[test]
    #[ignore = "실제 화면 필요 — 메인이 실행"]
    fn 주_모니터_3초_녹화() {
        let mut r = WinRecorder::new(out_dir().join("temp"), out_dir());
        let m = crate::screen::monitor_layout();
        let rect = m.primary().unwrap().bounds;
        let opts = RecStartOpts {
            target: RecTarget::Monitor { monitor_id: 0, rect },
            temp_path: out_dir().join("temp").join("rec-1.mp4"),
            fps: 30,
            bitrate: recording::bitrate(rect.w as u32, rect.h as u32, 30),
            mic_device: None,
            capture_cursor: true,
        };
        let t = Instant::now();
        r.start(opts).expect("start");
        eprintln!("start {:?}", t.elapsed());
        std::thread::sleep(Duration::from_secs(3));
        let s = r.stop().expect("stop");
        eprintln!("{s:?}");
        assert!(s.frames > 0);
    }
}
