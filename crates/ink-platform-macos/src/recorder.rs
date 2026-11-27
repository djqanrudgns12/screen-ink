//! 맥 F3 녹화(M15, 05 §8·R-REC-1·2): ScreenCaptureKit `SCStream` 프레임 → AVAssetWriter(H.264, mp4).
//! Windows `recorder.rs`(Media Foundation 직접)와 같은 포트·같은 규칙:
//! - 대상: 모니터 전체 · 범위(모니터 캡처의 `sourceRect`) · 창 추적(`desktopIndependentWindow` — 창이 움직여도 따라감, 사용자 결정 #6)
//! - 1080p30 5 Mbps 등 비트레이트는 서비스가 정한 값(`recording::bitrate`, 사용자 결정), 키프레임 2초마다
//! - 우리 섬·빨간 꺾쇠·정지 패널은 창 공유 끔(`NSWindowSharingNone`)이라 영상에 안 찍힌다. 판서·확대 오버레이는 찍힌다
//!   (녹화 중 Alt+1·Alt+2는 녹화를 끊지 않는다 — 수업 판서를 함께 남기려는 것)
//! - SCK는 화면이 바뀔 때만 완전한 프레임을 준다 → 완전한 프레임(`SCFrameStatus::Complete`)만 넣는다
//! - 마무리 = 스트림 정지 → 입력 끝 → `finishWriting`(moov). 실패하면 임시 파일은 남긴다(다음 시작 때 복구 제안)
//! - 마이크(단계 6.5): macOS 15+ SCK `captureMicrophone` + `SCStreamOutputType::Microphone` → AVAssetWriter AAC 트랙(영상·소리 같은 시계).
//!   권한 없음·장치 없음·시작 실패 때는 소리 없이 영상만 녹화를 이어 간다. 마이크 장치 이름·ID는 로그에 남기지 않는다.
//! - 창 추적(단계 6.4): 대상 창 위치·크기 변화와 숨김(최소화·다른 Space)을 감시해 빨간 꺾쇠 갱신 사건을 낸다.

use crate::coords::Space;
use block2::RcBlock;
use ink_app::{MicAccess, RecStartOpts, RecStats, Recorder, SavedFile, REC_WINDOW_CLOSED};
use ink_domain::geometry::PhysRect;
use ink_domain::naming;
use ink_domain::recording::{self, Leftover, RecTarget};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObject, ProtocolObject};
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_av_foundation::{
    AVAssetWriter, AVAssetWriterInput, AVAssetWriterStatus, AVFileTypeMPEG4, AVMediaTypeAudio, AVMediaTypeVideo, AVVideoAverageBitRateKey, AVVideoCodecKey,
    AVVideoCodecTypeH264, AVVideoCompressionPropertiesKey, AVVideoExpectedSourceFrameRateKey, AVVideoHeightKey, AVVideoMaxKeyFrameIntervalKey,
    AVVideoProfileLevelH264HighAutoLevel, AVVideoProfileLevelKey, AVVideoWidthKey,
};
use objc2_core_foundation::{CFArray, CGPoint, CGRect, CGSize};
use objc2_core_media::{CMSampleBuffer, CMTime};
use objc2_foundation::{NSArray, NSCopying, NSDictionary, NSError, NSNumber, NSObjectProtocol, NSProcessInfo, NSString, NSURL};
use objc2_screen_capture_kit::{
    SCContentFilter, SCFrameStatus, SCStream, SCStreamConfiguration, SCStreamDelegate, SCStreamFrameInfoStatus, SCStreamOutput, SCStreamOutputType,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// 시작(스트림이 첫 프레임을 내기 시작함)·정지·마무리를 기다리는 시간.
const START_WAIT: Duration = Duration::from_secs(3);
const STOP_WAIT: Duration = Duration::from_secs(3);
const FINISH_WAIT: Duration = Duration::from_secs(15);
/// 창 추적 녹화 중 대상 창 위치 확인 주기(6.4).
const TRACK_INTERVAL: Duration = Duration::from_millis(50);
/// `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange`('420v') — H.264 인코더가 바로 먹는 형식.
const PIXEL_420V: u32 = u32::from_be_bytes(*b"420v");
/// AAC 오디오 설정(48 kHz · 스테레오 · 128 kbps).
const FORMAT_AAC: u32 = u32::from_be_bytes(*b"aac ");
const AUDIO_SAMPLE_RATE: f64 = 48_000.0;
const AUDIO_CHANNELS: i64 = 2;
const AUDIO_BITRATE: i64 = 128_000;

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVCaptureDeviceTypeMicrophone: &'static NSString;
    static AVCaptureDeviceTypeExternal: &'static NSString;
}

#[link(name = "AVFAudio", kind = "framework")]
extern "C" {
    static AVFormatIDKey: &'static NSString;
    static AVSampleRateKey: &'static NSString;
    static AVNumberOfChannelsKey: &'static NSString;
    static AVEncoderBitRateKey: &'static NSString;
}

/// SCK 마이크 캡처 지원 OS 판정(macOS 15.0+).
pub fn os_supports_sck_mic(major: isize) -> bool {
    major >= 15
}

fn os_major_version() -> isize {
    NSProcessInfo::processInfo().operatingSystemVersion().majorVersion
}

/// `CMTime`을 나노초로 환산한다(유효하지 않거나 timescale <= 0이면 `None`).
pub fn pts_to_ns(value: i64, timescale: i32, valid: bool) -> Option<i64> {
    if !valid || timescale <= 0 {
        return None;
    }
    let ns = (value as i128).checked_mul(1_000_000_000)? / (timescale as i128);
    i64::try_from(ns).ok()
}

/// 영상·소리 표본 시각 맞춤(단조 증가·세션 시작 뒤 오디오 허용·일시 정지 오프셋 보정).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncTimeline {
    start_ns: Option<i64>,
    last_video_ns: Option<i64>,
    last_audio_ns: Option<i64>,
    paused_offset_ns: i64,
    pause_started_ns: Option<i64>,
}

impl SyncTimeline {
    /// 영상 프레임을 받아 `(보정 나노초, 세션 시작 여부)`를 돌려준다.
    pub fn accept_video(&mut self, raw_ns: i64) -> Option<(i64, bool)> {
        if self.pause_started_ns.is_some() {
            return None;
        }
        let adj = raw_ns.checked_sub(self.paused_offset_ns)?;
        match self.last_video_ns {
            None => {
                self.start_ns = Some(adj);
                self.last_video_ns = Some(adj);
                Some((adj, true))
            }
            Some(last) if adj > last => {
                self.last_video_ns = Some(adj);
                Some((adj, false))
            }
            Some(_) => None,
        }
    }

    /// 오디오 표본을 받아 보정 나노초를 돌려준다(첫 영상 프레임 전·역행 표본은 드롭).
    pub fn accept_audio(&mut self, raw_ns: i64) -> Option<i64> {
        if self.pause_started_ns.is_some() {
            return None;
        }
        let start = self.start_ns?;
        let adj = raw_ns.checked_sub(self.paused_offset_ns)?;
        if adj < start {
            return None;
        }
        if self.last_audio_ns.is_some_and(|last| adj < last) {
            return None;
        }
        self.last_audio_ns = Some(adj);
        Some(adj)
    }

    /// 일시 정지 시작 시각 기록.
    pub fn pause(&mut self, at_ns: i64) {
        if self.pause_started_ns.is_none() {
            self.pause_started_ns = Some(at_ns);
        }
    }

    /// 재개 시각을 받아 일시 정지 길이를 누적한다.
    pub fn resume(&mut self, at_ns: i64) {
        if let Some(p0) = self.pause_started_ns.take() {
            self.paused_offset_ns = self.paused_offset_ns.saturating_add(at_ns.saturating_sub(p0));
        }
    }
}

/// SCK·AV 객체를 스레드 사이로 넘기는 껍데기. 작성기는 SCK 표본 큐(직렬) 하나와, 스트림을 멈춘 뒤 마무리하는 스레드만 쓴다.
struct Hold<T>(T);
// SAFETY: 위 규칙 — 같은 객체를 두 스레드가 동시에 만지지 않는다(정지 완료를 기다린 뒤 마무리).
unsafe impl<T> Send for Hold<T> {}
unsafe impl<T> Sync for Hold<T> {}

/// 표본 처리기와 녹화기가 나눠 쓰는 상태.
struct Sink {
    writer: Hold<Retained<AVAssetWriter>>,
    input: Hold<Retained<AVAssetWriterInput>>,
    audio_input: Option<Hold<Retained<AVAssetWriterInput>>>,
    sync: Mutex<SyncTimeline>,
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
            let Some(raw_ns) = pts_to_ns(pts.value, pts.timescale, (pts.flags.0 & 1) != 0) else {
                return;
            };
            let start_session = {
                let mut tl = self.sync.lock().unwrap_or_else(|e| e.into_inner());
                let Some((_, start_session)) = tl.accept_video(raw_ns) else {
                    return;
                };
                start_session
            };
            let writer = &self.writer.0;
            if start_session {
                if !writer.startWriting() {
                    self.fail(format!("writer start failed: {}", writer.error().map(|e| e.code()).unwrap_or(0)));
                    return;
                }
                writer.startSessionAtSourceTime(pts);
                self.started.store(true, Ordering::SeqCst);
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

    /// 마이크 표본 하나를 작성기에 넣는다(표본 큐에서).
    fn on_audio(&self, buf: &CMSampleBuffer) {
        let Some(audio_input) = &self.audio_input else {
            return;
        };
        // SAFETY: SCK가 넘긴 살아 있는 표본.
        unsafe {
            if !buf.is_valid() || !buf.data_is_ready() || !self.started.load(Ordering::SeqCst) {
                return;
            }
            let pts = buf.presentation_time_stamp();
            let Some(raw_ns) = pts_to_ns(pts.value, pts.timescale, (pts.flags.0 & 1) != 0) else {
                return;
            };
            {
                let mut tl = self.sync.lock().unwrap_or_else(|e| e.into_inner());
                if tl.accept_audio(raw_ns).is_none() {
                    return;
                }
            }
            let writer = &self.writer.0;
            if writer.status() != AVAssetWriterStatus::Writing {
                return;
            }
            let input = &audio_input.0;
            if input.isReadyForMoreMediaData() {
                let _ = input.appendSampleBuffer(buf);
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
            match kind {
                SCStreamOutputType::Screen => self.ivars().on_frame(buf),
                SCStreamOutputType::Microphone => self.ivars().on_audio(buf),
                _ => {}
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
    mic_active: bool,
    pending_rect: Arc<Mutex<Option<Option<PhysRect>>>>,
    track_stop: Arc<AtomicBool>,
    track_thread: Option<JoinHandle<()>>,
}

/// `Recorder` 구현(맥). `temp_dir` = `$TMPDIR/Shookshok`, `default_folder` = 저장 폴더가 비었을 때(사진/Shookshok).
pub struct MacRecorder {
    temp_dir: PathBuf,
    default_folder: PathBuf,
    live: Option<Live>,
    on_window_rect: Option<Arc<dyn Fn(Option<PhysRect>) + Send + Sync>>,
}

impl MacRecorder {
    pub fn new(temp_dir: PathBuf, default_folder: PathBuf) -> Self {
        MacRecorder { temp_dir, default_folder, live: None, on_window_rect: None }
    }

    /// 창 추적 녹화 중 대상 창의 위치·크기·숨김 변화를 즉시 알릴 콜백을 붙인다.
    pub fn with_window_rect_callback(mut self, cb: impl Fn(Option<PhysRect>) + Send + Sync + 'static) -> Self {
        self.on_window_rect = Some(Arc::new(cb));
        self
    }
}

/// 마이크 입력 장치 목록 `(uniqueID, localizedName)`(로그·기록에 남기지 않는다).
fn list_audio_inputs() -> Vec<(String, String)> {
    let Some(cls) = AnyClass::get(c"AVCaptureDeviceDiscoverySession") else {
        return Vec::new();
    };
    let Some(media) = (unsafe { AVMediaTypeAudio }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    // SAFETY: AVFoundation 정적 장치 유형 + AVCaptureDeviceDiscoverySession 공개 클래스 메서드.
    unsafe {
        let types: Retained<NSArray<NSString>> = NSArray::from_slice(&[AVCaptureDeviceTypeMicrophone, AVCaptureDeviceTypeExternal]);
        let session: Retained<AnyObject> = msg_send![
            cls,
            discoverySessionWithDeviceTypes: &*types,
            mediaType: media,
            position: 0isize
        ];
        let devices: Retained<NSArray<AnyObject>> = msg_send![&*session, devices];
        for dev in devices.iter() {
            let uid: Retained<NSString> = msg_send![&*dev, uniqueID];
            let name: Retained<NSString> = msg_send![&*dev, localizedName];
            let id = uid.to_string();
            let label = name.to_string();
            if !id.is_empty() && !label.is_empty() && seen.insert(id.clone()) {
                out.push((id, label));
            }
        }
    }
    out
}

/// 요청한 마이크 설정을 실제 캡처 설정으로 확인한다.
/// 반환: `None` = 사용 안 함/장치 없음, `Some(None)` = 시스템 기본 마이크, `Some(Some(id))` = 지정 마이크.
fn resolve_mic_device(requested: Option<&str>) -> Option<Option<String>> {
    let req = requested?;
    // 기본 마이크: 장치를 미리 찾지 않고 SCK에 맡긴다(`microphoneCaptureDeviceID` = nil이면 시스템 기본 입력).
    // 기본 입력이 정말 없으면 스트림 시작이 실패하고, 부르는 쪽이 영상만으로 다시 시작한다.
    if req.is_empty() {
        return Some(None);
    }
    let found = AnyClass::get(c"AVCaptureDevice").zip(unsafe { AVMediaTypeAudio }).and_then(|(cls, _)| {
        let ns_id = NSString::from_str(req);
        // SAFETY: `deviceWithUniqueID:`는 연결된 장치가 없으면 nil.
        let dev: Option<Retained<AnyObject>> = unsafe { msg_send![cls, deviceWithUniqueID: &*ns_id] };
        dev
    });
    // 고른 마이크가 빠져 있으면 녹음을 포기하지 않고 기본 마이크로
    Some(found.map(|_| req.to_string()))
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

struct WriterPair {
    writer: Retained<AVAssetWriter>,
    video: Retained<AVAssetWriterInput>,
    audio: Option<Retained<AVAssetWriterInput>>,
}

/// H.264(+선택적 AAC 오디오) mp4 작성기와 입력들.
fn writer_for(path: &Path, w: u32, h: u32, fps: u32, bitrate: u32, with_audio: bool) -> Result<WriterPair, String> {
    let num = |v: i64| NSNumber::numberWithLongLong(v);
    let fnum = |v: f64| NSNumber::numberWithDouble(v);
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
        let audio_input = if with_audio {
            let audio_settings = NSDictionary::<NSString, AnyObject>::from_retained_objects(
                &[AVFormatIDKey, AVSampleRateKey, AVNumberOfChannelsKey, AVEncoderBitRateKey],
                &[
                    Retained::into_super(Retained::into_super(Retained::into_super(num(FORMAT_AAC as i64)))),
                    Retained::into_super(Retained::into_super(Retained::into_super(fnum(AUDIO_SAMPLE_RATE)))),
                    Retained::into_super(Retained::into_super(Retained::into_super(num(AUDIO_CHANNELS)))),
                    Retained::into_super(Retained::into_super(Retained::into_super(num(AUDIO_BITRATE)))),
                ],
            );
            let a = AVAssetWriterInput::initWithMediaType_outputSettings(AVAssetWriterInput::alloc(), key(AVMediaTypeAudio)?, Some(&audio_settings));
            a.setExpectsMediaDataInRealTime(true);
            if !writer.canAddInput(&a) {
                return Err("writer refused the audio input".into());
            }
            writer.addInput(&a);
            Some(a)
        } else {
            None
        };
        Ok(WriterPair { writer, video: input, audio: audio_input })
    }
}

struct StreamSpec<'a> {
    filter: &'a SCContentFilter,
    source: Option<CGRect>,
    w: u32,
    h: u32,
    fps: u32,
    capture_cursor: bool,
    mic: Option<Option<&'a str>>,
}

/// SCK 스트림 생성·출력 등록·캡처 시작.
fn open_stream(spec: &StreamSpec<'_>, output: &RecOutput) -> Result<Retained<SCStream>, String> {
    // SAFETY: 설정·스트림은 만든 뒤 SCK에 넘긴다. 표본 큐는 직렬(작성기를 한 곳에서만 만짐).
    let stream = unsafe {
        let config = SCStreamConfiguration::new();
        config.setWidth(spec.w as usize);
        config.setHeight(spec.h as usize);
        if let Some(r) = spec.source {
            config.setSourceRect(r);
        }
        config.setMinimumFrameInterval(CMTime::new(1, spec.fps as i32));
        config.setShowsCursor(spec.capture_cursor);
        config.setPixelFormat(PIXEL_420V);
        config.setQueueDepth(6);
        if let Some(dev_id) = spec.mic {
            config.setCaptureMicrophone(true);
            if let Some(id) = dev_id {
                config.setMicrophoneCaptureDeviceID(Some(&NSString::from_str(id)));
            }
        }
        let delegate = ProtocolObject::<dyn SCStreamDelegate>::from_ref(output);
        let stream = SCStream::initWithFilter_configuration_delegate(SCStream::alloc(), spec.filter, &config, Some(delegate));
        let queue = dispatch2::DispatchQueue::new("kr.shookshok.rec", None);
        stream
            .addStreamOutput_type_sampleHandlerQueue_error(ProtocolObject::from_ref(output), SCStreamOutputType::Screen, Some(&queue))
            .map_err(|e| format!("stream output: {}", e.code()))?;
        if spec.mic.is_some() {
            stream
                .addStreamOutput_type_sampleHandlerQueue_error(ProtocolObject::from_ref(output), SCStreamOutputType::Microphone, Some(&queue))
                .map_err(|e| format!("mic output: {}", e.code()))?;
        }
        stream
    };
    // SAFETY: 살아 있는 스트림.
    wait_done(|b| unsafe { stream.startCaptureWithCompletionHandler(Some(b)) }, START_WAIT).map_err(|e| format!("capture did not start: {e}"))?;
    Ok(stream)
}

impl MacRecorder {
    /// 스트림을 멈추고 작성기를 마무리한다. 프레임이 하나도 없으면 오류(임시 파일은 남김).
    fn halt(&mut self, finish: bool) -> Option<(PathBuf, Result<RecStats, String>)> {
        let mut live = self.live.take()?;
        live.track_stop.store(true, Ordering::SeqCst);
        if let Some(t) = live.track_thread.take() {
            let _ = t.join();
        }
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
            if let Some(ref a) = sink.audio_input {
                a.0.markAsFinished();
            }
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

    fn mic_supported(&self) -> bool {
        os_supports_sck_mic(os_major_version())
    }

    fn mic_access(&self) -> MicAccess {
        crate::permissions::microphone()
    }

    fn request_mic_access(&mut self) {
        crate::permissions::request_microphone();
    }

    fn mic_active(&self) -> Option<bool> {
        self.live.as_ref().map(|l| l.mic_active)
    }

    fn audio_inputs(&self) -> Vec<(String, String)> {
        if self.mic_supported() {
            list_audio_inputs()
        } else {
            Vec::new()
        }
    }

    fn take_window_rect(&mut self) -> Option<Option<PhysRect>> {
        self.live.as_ref().and_then(|l| l.pending_rect.lock().unwrap_or_else(|e| e.into_inner()).take())
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
        let resolved_mic = if self.mic_supported() { resolve_mic_device(opts.mic_device.as_deref()) } else { None };
        let mut mic_active = resolved_mic.is_some();

        let make_sink = |with_audio: bool| -> Result<(Arc<Sink>, Retained<RecOutput>), String> {
            let pair = writer_for(&opts.temp_path, w, h, fps, opts.bitrate, with_audio)?;
            let sink = Arc::new(Sink {
                writer: Hold(pair.writer),
                input: Hold(pair.video),
                audio_input: pair.audio.map(Hold),
                sync: Mutex::new(SyncTimeline::default()),
                started: AtomicBool::new(false),
                frames: AtomicU64::new(0),
                t0: Mutex::new(None),
                failure: Mutex::new(None),
                window_target: matches!(opts.target, RecTarget::Window { .. }),
            });
            let output = RecOutput::new(sink.clone());
            Ok((sink, output))
        };

        let (mut sink, mut output) = make_sink(mic_active)?;
        let spec = StreamSpec {
            filter: &filter,
            source,
            w,
            h,
            fps,
            capture_cursor: opts.capture_cursor,
            mic: resolved_mic.as_ref().map(|o| o.as_deref()),
        };
        let stream = match open_stream(&spec, &output) {
            Ok(s) => s,
            Err(e) if mic_active => {
                // 마이크 장치 시작 실패 시(권한·장치 분리 등) 임시 파일을 비우고 영상 전용으로 계속한다.
                log::warn!("recorder mic failed ({e}), continuing video only");
                let _ = std::fs::remove_file(&opts.temp_path);
                mic_active = false;
                let pair = make_sink(false)?;
                sink = pair.0;
                output = pair.1;
                let fallback = StreamSpec { mic: None, ..spec };
                open_stream(&fallback, &output)?
            }
            Err(e) => return Err(e),
        };

        let pending_rect = Arc::new(Mutex::new(None));
        let track_stop = Arc::new(AtomicBool::new(false));
        let track_thread = if let RecTarget::Window { handle, .. } = opts.target {
            let win_id = handle as u32;
            let initial = crate::windows::visible_rect(win_id, &space);
            *pending_rect.lock().unwrap_or_else(|e| e.into_inner()) = Some(initial);
            if let Some(cb) = &self.on_window_rect {
                cb(initial);
            }
            let stop_flag = track_stop.clone();
            let pending = pending_rect.clone();
            let cb = self.on_window_rect.clone();
            std::thread::Builder::new()
                .name("shookshok-rec-track".into())
                .spawn(move || {
                    let mut last = initial;
                    while !stop_flag.load(Ordering::SeqCst) {
                        std::thread::sleep(TRACK_INTERVAL);
                        if stop_flag.load(Ordering::SeqCst) {
                            break;
                        }
                        let cur = crate::windows::visible_rect(win_id, &crate::space());
                        if cur != last {
                            last = cur;
                            *pending.lock().unwrap_or_else(|e| e.into_inner()) = Some(cur);
                            if let Some(cb) = &cb {
                                cb(cur);
                            }
                        }
                    }
                })
                .ok()
        } else {
            None
        };

        log::info!("recorder started {w}x{h}@{fps} {} kbps mic={mic_active}", opts.bitrate / 1000);
        self.live = Some(Live {
            stream: Hold(stream),
            _output: Hold(output),
            sink,
            temp: opts.temp_path,
            width: w,
            height: h,
            mic_active,
            pending_rect,
            track_stop,
            track_thread,
        });
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

    #[test]
    fn os_버전에_따른_마이크_지원_판정() {
        assert!(!os_supports_sck_mic(13));
        assert!(!os_supports_sck_mic(14));
        assert!(os_supports_sck_mic(15));
        assert!(os_supports_sck_mic(27));
    }

    #[test]
    fn cmtime_나노초_환산과_무효_거부() {
        assert_eq!(pts_to_ns(300, 600, true), Some(500_000_000));
        assert_eq!(pts_to_ns(1, 1000, true), Some(1_000_000));
        assert_eq!(pts_to_ns(10, 0, true), None);
        assert_eq!(pts_to_ns(10, -1, true), None);
        assert_eq!(pts_to_ns(10, 1000, false), None);
    }

    #[test]
    fn 영상_소리_시각_맞춤_및_일시정지_재개() {
        let mut tl = SyncTimeline::default();
        // 첫 영상 프레임 전 오디오는 드롭
        assert_eq!(tl.accept_audio(900), None);
        // 첫 영상 프레임이 세션을 연다
        assert_eq!(tl.accept_video(1_000), Some((1_000, true)));
        // 세션 시작 이전 오디오와 역행 영상은 거부
        assert_eq!(tl.accept_audio(999), None);
        assert_eq!(tl.accept_video(1_000), None);
        assert_eq!(tl.accept_video(990), None);
        // 정상 진행
        assert_eq!(tl.accept_audio(1_000), Some(1_000));
        assert_eq!(tl.accept_audio(1_020), Some(1_020));
        assert_eq!(tl.accept_audio(1_010), None);
        assert_eq!(tl.accept_video(1_033), Some((1_033, false)));

        // 일시 정지 중 모두 드롭, 재개 후 같은 오프셋으로 이어짐
        tl.pause(1_100);
        assert_eq!(tl.accept_video(1_200), None);
        assert_eq!(tl.accept_audio(1_200), None);
        tl.resume(2_100);
        // paused_offset_ns = 1_000 → 2_133 - 1_000 = 1_133 (> 1_033)
        assert_eq!(tl.accept_video(2_133), Some((1_133, false)));
        assert_eq!(tl.accept_audio(2_135), Some(1_135));
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
