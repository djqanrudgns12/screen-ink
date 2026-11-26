//! 맥 F3 녹화(M15, 05 §8·R-REC-1·2): ScreenCaptureKit `SCStream` 프레임 → AVAssetWriter(H.264, mp4).
//! Windows `recorder.rs`(Media Foundation 직접)와 같은 포트·같은 규칙:
//! - 대상: 모니터 전체 · 범위(모니터 캡처의 `sourceRect`) · 창 추적(`desktopIndependentWindow` — 창이 움직여도 따라감, 사용자 결정 #6)
//! - 1080p30 5 Mbps 등 비트레이트는 서비스가 정한 값(`recording::bitrate`, 사용자 결정), 키프레임 2초마다
//! - 우리 섬·빨간 꺾쇠·정지 패널은 창 공유 끔(`NSWindowSharingNone`)이라 영상에 안 찍힌다. 판서·확대 오버레이는 찍힌다
//!   (녹화 중 Alt+1·Alt+2는 녹화를 끊지 않는다 — 수업 판서를 함께 남기려는 것)
//! - SCK는 화면이 바뀔 때만 완전한 프레임을 준다 → 완전한 프레임(`SCFrameStatus::Complete`)만 넣는다
//! - 마무리 = 스트림 정지 → 입력 끝 → `finishWriting`(moov). 실패하면 임시 파일은 남긴다(다음 시작 때 복구 제안)
//! - 마이크는 아직 없다(`mic_supported` false — 서비스가 끄고 한 번 알린다, Windows와 같음)

use crate::coords::Space;
use block2::RcBlock;
use ink_app::{RecStartOpts, RecStats, Recorder, SavedFile, REC_WINDOW_CLOSED};
use ink_domain::recording::{self, Leftover, RecTarget};
use ink_domain::naming;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_av_foundation::{
    AVAssetWriter, AVAssetWriterInput, AVAssetWriterStatus, AVFileTypeMPEG4, AVMediaTypeVideo, AVVideoAverageBitRateKey, AVVideoCodecKey, AVVideoCodecTypeH264,
    AVVideoCompressionPropertiesKey, AVVideoExpectedSourceFrameRateKey, AVVideoHeightKey, AVVideoMaxKeyFrameIntervalKey, AVVideoProfileLevelH264HighAutoLevel,
    AVVideoProfileLevelKey, AVVideoWidthKey,
};
use objc2_core_foundation::{CFArray, CGPoint, CGRect, CGSize};
use objc2_core_media::{CMSampleBuffer, CMTime};
use objc2_foundation::{NSArray, NSCopying, NSDictionary, NSError, NSNumber, NSObjectProtocol, NSString, NSURL};
use objc2_screen_capture_kit::{
    SCContentFilter, SCFrameStatus, SCStream, SCStreamConfiguration, SCStreamDelegate, SCStreamFrameInfoStatus, SCStreamOutput, SCStreamOutputType,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// 시작(스트림이 첫 프레임을 내기 시작함)·정지·마무리를 기다리는 시간.
const START_WAIT: Duration = Duration::from_secs(3);
const STOP_WAIT: Duration = Duration::from_secs(3);
const FINISH_WAIT: Duration = Duration::from_secs(15);
/// `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange`('420v') — H.264 인코더가 바로 먹는 형식.
const PIXEL_420V: u32 = u32::from_be_bytes(*b"420v");

/// SCK·AV 객체를 스레드 사이로 넘기는 껍데기. 작성기는 SCK 표본 큐(직렬) 하나와, 스트림을 멈춘 뒤 마무리하는 스레드만 쓴다.
struct Hold<T>(T);
// SAFETY: 위 규칙 — 같은 객체를 두 스레드가 동시에 만지지 않는다(정지 완료를 기다린 뒤 마무리).
unsafe impl<T> Send for Hold<T> {}
unsafe impl<T> Sync for Hold<T> {}

/// 표본 처리기와 녹화기가 나눠 쓰는 상태.
struct Sink {
    writer: Hold<Retained<AVAssetWriter>>,
    input: Hold<Retained<AVAssetWriterInput>>,
    started: AtomicBool,
    frames: AtomicU64,
    t0: Mutex<Option<Instant>>,
    /// 스트림이 스스로 멈춘 까닭(창 닫힘 등)
    failure: Mutex<Option<String>>,
    window_target: bool,
}

impl Sink {
    /// 완전한 화면 프레임 하나를 작성기에 넣는다(표본 큐에서).
    fn on_frame(&self, buf: &CMSampleBuffer) {
        // SAFETY: SCK가 넘긴 살아 있는 표본. 읽기 전용 함수들.
        unsafe {
            if !buf.is_valid() || !frame_complete(buf) {
                return;
            }
            let pts = buf.presentation_time_stamp();
            let writer = &self.writer.0;
            if !self.started.swap(true, Ordering::SeqCst) {
                if !writer.startWriting() {
                    self.fail(format!("writer start failed: {}", writer.error().map(|e| e.code()).unwrap_or(0)));
                    return;
                }
                writer.startSessionAtSourceTime(pts);
                *self.t0.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
            }
            if writer.status() != AVAssetWriterStatus::Writing {
                return;
            }
            let input = &self.input.0;
            if input.isReadyForMoreMediaData() && input.appendSampleBuffer(buf) {
                self.frames.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn fail(&self, why: String) {
        log::warn!("recorder: {why}");
        let mut f = self.failure.lock().unwrap_or_else(|e| e.into_inner());
        if f.is_none() {
            *f = Some(why);
        }
    }
}

/// 표본 첨부 사전의 `SCStreamFrameInfoStatus`가 완전한 프레임인지.
pub(crate) unsafe fn frame_complete(buf: &CMSampleBuffer) -> bool {
    // SAFETY: 살아 있는 표본. CFArray ↔ NSArray 무료 다리, 원소는 SCK가 만든 사전(NSString → NSNumber).
    unsafe {
        let Some(arr) = buf.sample_attachments_array(false) else { return false };
        let arr: &NSArray<NSDictionary<NSString, AnyObject>> = &*(&*arr as *const CFArray as *const NSArray<NSDictionary<NSString, AnyObject>>);
        let Some(d) = arr.iter().next() else { return false };
        d.objectForKey(SCStreamFrameInfoStatus).and_then(|v| v.downcast::<NSNumber>().ok()).is_some_and(|n| n.integerValue() == SCFrameStatus::Complete.0)
    }
}

define_class!(
    // SAFETY: NSObject는 하위 클래스 요구가 없고, 이 클래스는 Drop을 구현하지 않는다.
    #[unsafe(super(NSObject))]
    #[name = "ShookshokRecOutput"]
    #[ivars = Arc<Sink>]
    struct RecOutput;

    unsafe impl NSObjectProtocol for RecOutput {}

    unsafe impl SCStreamOutput for RecOutput {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        fn stream_did_output(&self, _stream: &SCStream, buf: &CMSampleBuffer, kind: SCStreamOutputType) {
            if kind == SCStreamOutputType::Screen {
                self.ivars().on_frame(buf);
            }
        }
    }

    unsafe impl SCStreamDelegate for RecOutput {
        #[unsafe(method(stream:didStopWithError:))]
        fn stream_did_stop(&self, _stream: &SCStream, error: &NSError) {
            let sink = self.ivars();
            // 창 추적 중 창이 닫히면 SCK가 스트림을 멈춘다 → 서비스는 "창이 닫혀 녹화를 멈췄어요"로 저장한다
            let why = if sink.window_target { REC_WINDOW_CLOSED.to_string() } else { format!("capture stopped: {}", error.code()) };
            sink.fail(why);
        }
    }
);

impl RecOutput {
    fn new(sink: Arc<Sink>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(sink);
        // SAFETY: NSObject init.
        unsafe { msg_send![super(this), init] }
    }
}

struct Live {
    stream: Hold<Retained<SCStream>>,
    /// 표본·중지 알림을 받는 객체 — 스트림은 약하게 잡으므로 녹화가 끝날 때까지 쥐고 있는다
    _output: Hold<Retained<RecOutput>>,
    sink: Arc<Sink>,
    temp: PathBuf,
    width: u32,
    height: u32,
}

/// `Recorder` 구현(맥). `temp_dir` = `$TMPDIR/Shookshok`, `default_folder` = 저장 폴더가 비었을 때(사진/Shookshok).
pub struct MacRecorder {
    temp_dir: PathBuf,
    default_folder: PathBuf,
    live: Option<Live>,
}

impl MacRecorder {
    pub fn new(temp_dir: PathBuf, default_folder: PathBuf) -> Self {
        MacRecorder { temp_dir, default_folder, live: None }
    }
}

/// 완료 처리기 하나를 기다린다(오류 코드 또는 성공).
fn wait_done(f: impl FnOnce(&block2::DynBlock<dyn Fn(*mut NSError)>), wait: Duration) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let block = RcBlock::new(move |err: *mut NSError| {
        // SAFETY: SCK가 넘긴 오류 포인터는 이 호출 동안 유효하다.
        let r = match unsafe { err.as_ref() } {
            Some(e) => Err(format!("{}", e.code())),
            None => Ok(()),
        };
        let _ = tx.send(r);
    });
    f(&block);
    rx.recv_timeout(wait).map_err(|_| "timed out".to_string())?
}

/// 녹화 대상 → SCK 필터와 (sourceRect 포인트, 영상 크기).
fn filter_for(target: &RecTarget, space: &Space) -> Result<(Retained<SCContentFilter>, Option<CGRect>, u32, u32), String> {
    let even = |v: i32| (v.max(2) as u32) & !1;
    match *target {
        RecTarget::Monitor { monitor_id, rect } | RecTarget::Region { monitor_id, rect } => {
            let s = space.screens.iter().find(|s| s.id == monitor_id).ok_or("monitor gone")?;
            let display = crate::capture::shareable_display(s.display)?;
            // SAFETY: 필터는 만든 뒤 SCK에 넘기기만 한다.
            let filter = unsafe { SCContentFilter::initWithDisplay_excludingWindows(SCContentFilter::alloc(), &display, &NSArray::new()) };
            let source = matches!(target, RecTarget::Region { .. }).then(|| {
                let p = space.rect_to_points(rect);
                CGRect::new(CGPoint::new(p.x - s.pt.x, p.y - s.pt.y), CGSize::new(p.w, p.h))
            });
            Ok((filter, source, even(rect.w), even(rect.h)))
        }
        RecTarget::Window { handle, rect } => {
            let w = crate::window_shot::find(handle as u32)?;
            // SAFETY: 위와 같음.
            let filter = unsafe { SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), &w) };
            Ok((filter, None, even(rect.w), even(rect.h)))
        }
    }
}

/// H.264 mp4 작성기와 영상 입력.
fn writer_for(path: &Path, w: u32, h: u32, fps: u32, bitrate: u32) -> Result<(Retained<AVAssetWriter>, Retained<AVAssetWriterInput>), String> {
    let num = |v: i64| NSNumber::numberWithLongLong(v);
    // SAFETY: AV 상수는 프레임워크가 쥔 정적 문자열. 사전·작성기는 여기서 만들어 돌려준다.
    unsafe {
        let key = |k: Option<&'static NSString>| k.ok_or("AVFoundation constant missing");
        let compression = NSDictionary::<NSString, AnyObject>::from_retained_objects(
            &[key(AVVideoAverageBitRateKey)?, key(AVVideoExpectedSourceFrameRateKey)?, key(AVVideoMaxKeyFrameIntervalKey)?, key(AVVideoProfileLevelKey)?],
            &[
                Retained::into_super(Retained::into_super(Retained::into_super(num(bitrate as i64)))),
                Retained::into_super(Retained::into_super(Retained::into_super(num(fps as i64)))),
                Retained::into_super(Retained::into_super(Retained::into_super(num(fps as i64 * 2)))),
                Retained::into_super(Retained::into_super(key(AVVideoProfileLevelH264HighAutoLevel)?.copy())),
            ],
        );
        let settings = NSDictionary::<NSString, AnyObject>::from_retained_objects(
            &[key(AVVideoCodecKey)?, key(AVVideoWidthKey)?, key(AVVideoHeightKey)?, key(AVVideoCompressionPropertiesKey)?],
            &[
                Retained::into_super(Retained::into_super(key(AVVideoCodecTypeH264)?.copy())),
                Retained::into_super(Retained::into_super(Retained::into_super(num(w as i64)))),
                Retained::into_super(Retained::into_super(Retained::into_super(num(h as i64)))),
                Retained::into_super(Retained::into_super(compression)),
            ],
        );
        let input = AVAssetWriterInput::initWithMediaType_outputSettings(AVAssetWriterInput::alloc(), key(AVMediaTypeVideo)?, Some(&settings));
        input.setExpectsMediaDataInRealTime(true);
        let url = NSURL::fileURLWithPath_isDirectory(&NSString::from_str(&path.to_string_lossy()), false);
        let writer = AVAssetWriter::initWithURL_fileType_error(AVAssetWriter::alloc(), &url, key(AVFileTypeMPEG4)?).map_err(|e| format!("writer: {}", e.code()))?;
        if !writer.canAddInput(&input) {
            return Err("writer refused the video input".into());
        }
        writer.addInput(&input);
        Ok((writer, input))
    }
}

impl MacRecorder {
    /// 스트림을 멈추고 작성기를 마무리한다. 프레임이 하나도 없으면 오류(임시 파일은 남김).
    fn halt(&mut self, finish: bool) -> Option<(PathBuf, Result<RecStats, String>)> {
        let live = self.live.take()?;
        // SAFETY: 살아 있는 스트림. 정지 완료를 기다린 뒤에는 표본 처리기가 더 불리지 않는다.
        let stopped = wait_done(|b| unsafe { live.stream.0.stopCaptureWithCompletionHandler(Some(b)) }, STOP_WAIT);
        if let Err(e) = &stopped {
            log::warn!("recorder stop: {e}");
        }
        let sink = &live.sink;
        let frames = sink.frames.load(Ordering::SeqCst);
        let elapsed_ms = sink.t0.lock().unwrap_or_else(|e| e.into_inner()).map(|t| t.elapsed().as_millis() as u64).unwrap_or(0);
        let writer = &sink.writer.0;
        if !finish {
            // SAFETY: 살아 있는 작성기.
            unsafe { writer.cancelWriting() };
            return Some((live.temp, Err("aborted".into())));
        }
        if !sink.started.load(Ordering::SeqCst) || frames == 0 {
            return Some((live.temp, Err("no frames".into())));
        }
        let (tx, rx) = mpsc::channel::<()>();
        let done = RcBlock::new(move || {
            let _ = tx.send(());
        });
        // SAFETY: 입력을 닫고 마무리(moov) — 완료 처리기는 AV 큐에서 돈다.
        unsafe {
            sink.input.0.markAsFinished();
            writer.finishWritingWithCompletionHandler(&done);
        }
        let r = match rx.recv_timeout(FINISH_WAIT) {
            // SAFETY: 마무리가 끝난 작성기의 상태 읽기.
            Ok(()) if unsafe { writer.status() } == AVAssetWriterStatus::Completed => {
                let bytes = std::fs::metadata(&live.temp).map(|m| m.len()).unwrap_or(0);
                Ok(RecStats { elapsed_ms, bytes, frames, width: live.width, height: live.height })
            }
            Ok(()) => Err(format!("finish failed: {}", unsafe { writer.error() }.map(|e| e.code()).unwrap_or(0))),
            Err(_) => Err("finish timed out".into()),
        };
        Some((live.temp, r))
    }
}

impl Recorder for MacRecorder {
    /// F3 고르기 중: 공유 가능한 내용 목록을 미리 받아 둔다(시작이 수십 ms 늦지 않게).
    fn prepare(&mut self) {
        crate::capture::warm();
    }

    fn start(&mut self, opts: RecStartOpts) -> Result<(), String> {
        if self.live.is_some() {
            return Err("already recording".into());
        }
        if !crate::permissions::screen() {
            return Err(ink_app::NO_SCREEN_PERMISSION.into());
        }
        if let Some(dir) = opts.temp_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("temp dir: {e}"))?;
        }
        let space = crate::space();
        let (filter, source, w, h) = filter_for(&opts.target, &space)?;
        let fps = opts.fps.clamp(1, 60);
        let (writer, input) = writer_for(&opts.temp_path, w, h, fps, opts.bitrate)?;
        let sink = Arc::new(Sink {
            writer: Hold(writer),
            input: Hold(input),
            started: AtomicBool::new(false),
            frames: AtomicU64::new(0),
            t0: Mutex::new(None),
            failure: Mutex::new(None),
            window_target: matches!(opts.target, RecTarget::Window { .. }),
        });
        let output = RecOutput::new(sink.clone());
        // SAFETY: 설정·스트림은 만든 뒤 SCK에 넘긴다. 표본 큐는 직렬(작성기를 한 곳에서만 만짐).
        let stream = unsafe {
            let config = SCStreamConfiguration::new();
            config.setWidth(w as usize);
            config.setHeight(h as usize);
            if let Some(r) = source {
                config.setSourceRect(r);
            }
            config.setMinimumFrameInterval(CMTime::new(1, fps as i32));
            config.setShowsCursor(opts.capture_cursor);
            config.setPixelFormat(PIXEL_420V);
            config.setQueueDepth(6);
            let delegate = ProtocolObject::<dyn SCStreamDelegate>::from_ref(&*output);
            let stream = SCStream::initWithFilter_configuration_delegate(SCStream::alloc(), &filter, &config, Some(delegate));
            let queue = dispatch2::DispatchQueue::new("kr.shookshok.rec", None);
            stream
                .addStreamOutput_type_sampleHandlerQueue_error(ProtocolObject::from_ref(&*output), SCStreamOutputType::Screen, Some(&queue))
                .map_err(|e| format!("stream output: {}", e.code()))?;
            stream
        };
        // SAFETY: 살아 있는 스트림.
        wait_done(|b| unsafe { stream.startCaptureWithCompletionHandler(Some(b)) }, START_WAIT).map_err(|e| format!("capture did not start: {e}"))?;
        log::info!("recorder started {w}x{h}@{fps} {} kbps", opts.bitrate / 1000);
        self.live = Some(Live { stream: Hold(stream), _output: Hold(output), sink, temp: opts.temp_path, width: w, height: h });
        Ok(())
    }

    fn stats(&self) -> RecStats {
        let Some(l) = &self.live else { return RecStats::default() };
        let elapsed_ms = l.sink.t0.lock().unwrap_or_else(|e| e.into_inner()).map(|t| t.elapsed().as_millis() as u64).unwrap_or(0);
        let bytes = std::fs::metadata(&l.temp).map(|m| m.len()).unwrap_or(0);
        RecStats { elapsed_ms, bytes, frames: l.sink.frames.load(Ordering::Relaxed), width: l.width, height: l.height }
    }

    fn stop(&mut self) -> Result<RecStats, String> {
        let (_, r) = self.halt(true).ok_or("not recording")?;
        r
    }

    fn abort(&mut self) {
        if let Some((temp, _)) = self.halt(false) {
            let _ = std::fs::remove_file(&temp);
        }
    }

    fn take_failure(&mut self) -> Option<String> {
        self.live.as_ref().and_then(|l| l.sink.failure.lock().unwrap_or_else(|e| e.into_inner()).take())
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

impl Drop for MacRecorder {
    fn drop(&mut self) {
        // 앱이 끝날 때 녹화 중이면 정상 마무리(실패하면 임시 파일은 다음 시작 때 복구 제안)
        let _ = self.halt(true);
    }
}

// ───────────── 파일(Windows `recorder.rs`와 같은 규칙) ─────────────

/// MP4 맨 위 상자 종류들(마무리 `moov`가 있는지).
fn mp4_kinds(path: &Path) -> Vec<[u8; 4]> {
    use std::io::{Read, Seek, SeekFrom};
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
    // 다른 볼륨: 복사 → 크기 확인 → 지움
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

/// 그 폴더(없으면 있는 조상)의 남은 바이트(`statvfs`).
pub fn free_bytes(dir: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let mut d = dir;
    while !d.exists() {
        d = d.parent()?;
    }
    let c = std::ffi::CString::new(d.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: 살아 있는 C 문자열과 출력 구조체 하나.
    (unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0).then(|| st.f_bavail as u64 * st.f_frsize as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 남은_공간은_있는_조상_폴더에서() {
        let free = free_bytes(Path::new("/tmp/shookshok-없는-폴더/아래")).expect("조상(/tmp)의 남은 공간");
        assert!(free > 0);
    }

    /// 실제 화면 녹화 2초(화면 기록 권한이 있는 셸에서 손으로). 영상은 임시 폴더에만 쓰고 바로 지운다(개인정보) —
    /// 크기·프레임 수·mp4 마무리(moov)만 본다.
    /// `cargo test -p ink-platform-macos recorder::tests::실제_모니터_녹화_2초 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn 실제_모니터_녹화_2초() {
        let space = crate::space();
        let m = space.layout().monitors.into_iter().find(|m| m.primary).expect("주 모니터");
        let dir = std::env::temp_dir().join("Shookshok-test");
        let temp = dir.join(recording::temp_name(1));
        let mut rec = MacRecorder::new(dir.clone(), dir.clone());
        let bitrate = recording::bitrate(m.bounds.w as u32, m.bounds.h as u32, 30);
        rec.start(RecStartOpts {
            target: RecTarget::Monitor { monitor_id: m.id, rect: m.bounds },
            temp_path: temp.clone(),
            fps: 30,
            bitrate,
            mic_device: None,
            capture_cursor: true,
        })
        .expect("녹화 시작");
        std::thread::sleep(Duration::from_millis(2000));
        let mid = rec.stats();
        let stats = rec.stop().expect("마무리");
        let kinds = mp4_kinds(&temp);
        eprintln!("{}x{} frames {} (중간 {}), {} bytes, boxes {:?}", stats.width, stats.height, stats.frames, mid.frames, stats.bytes, kinds.iter().map(|k| String::from_utf8_lossy(k).into_owned()).collect::<Vec<_>>());
        let _ = std::fs::remove_file(&temp);
        assert!(stats.frames >= 1, "최소 한 프레임");
        assert!(recording::mp4_playable(&kinds), "moov까지 마무리");
        assert_eq!((stats.width, stats.height), ((m.bounds.w as u32) & !1, (m.bounds.h as u32) & !1));
    }
}
