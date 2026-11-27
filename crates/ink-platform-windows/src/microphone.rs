//! WASAPI 공유 모드 입력. 장치 식별자와 이름은 설정 화면에만 돌려주고 기록하지 않는다.

use crate::recorder::Timeline;
use ink_app::MicAccess;
use windows::core::{w, Result, HSTRING, PWSTR};
use windows::Devices::Enumeration::{DeviceAccessInformation, DeviceAccessStatus, DeviceClass};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{E_INVALIDARG, RPC_E_CHANGED_MODE};
use windows::Win32::Media::Audio::*;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ};
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

const RATE: u32 = 48_000;
const FRAME_BYTES: usize = 4;
const TICKS: i64 = 10_000_000;

struct Apartment(bool);

impl Apartment {
    fn enter() -> Result<Self> {
        // 호출자가 이미 STA이면 그 아파트에서 읽고, 직접 초기화한 횟수만 되돌린다.
        match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            Ok(()) => Ok(Self(true)),
            Err(e) if e.code() == RPC_E_CHANGED_MODE => Ok(Self(false)),
            Err(e) => Err(e),
        }
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { RoUninitialize() };
        }
    }
}

pub(crate) fn access() -> MicAccess {
    let _apartment = Apartment::enter();
    let status = DeviceAccessInformation::CreateFromDeviceClass(DeviceClass::AudioCapture).and_then(|info| info.CurrentStatus());
    if matches!(status, Ok(DeviceAccessStatus::DeniedByUser | DeviceAccessStatus::DeniedBySystem)) || desktop_access_denied() {
        MicAccess::Denied
    } else {
        MicAccess::Granted
    }
}

// 데스크톱 앱 전체에 적용되는 별도 스위치도 확인한다. 설정을 쓰거나 장치 기록을 열거하지 않는다.
fn desktop_access_denied() -> bool {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let microphone = w!(r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone");
    let desktop = w!(r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone\NonPackaged");
    [(HKEY_LOCAL_MACHINE, microphone), (HKEY_CURRENT_USER, desktop)].into_iter().any(|(root, key)| {
        let mut value = [0u16; 16];
        let mut bytes = std::mem::size_of_val(&value) as u32;
        let status = unsafe {
            RegGetValueW(
                root,
                key.as_ptr(),
                w!("Value").as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                value.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        status == 0 && value.split(|&c| c == 0).next().is_some_and(|s| s.iter().copied().eq("Deny".encode_utf16()))
    })
}

pub fn open_settings() -> std::result::Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            w!("open").as_ptr(),
            w!("ms-settings:privacy-microphone").as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize > 32 {
        Ok(())
    } else {
        Err("마이크 설정을 열 수 없어요".into())
    }
}

fn enumerator() -> Result<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

fn take_string(ptr: PWSTR) -> Result<String> {
    // GetId와 PropVariantToStringAlloc은 모두 COM 작업 할당자로 문자열을 넘긴다.
    let value = unsafe { ptr.to_string() };
    unsafe { CoTaskMemFree(Some(ptr.as_ptr().cast())) };
    Ok(value?)
}

fn device_info(device: IMMDevice) -> Result<(String, String)> {
    unsafe {
        let id = take_string(device.GetId()?)?;
        let props = device.OpenPropertyStore(STGM_READ)?;
        let mut value = props.GetValue(&PKEY_Device_FriendlyName)?;
        let name = PropVariantToStringAlloc(&value);
        let _ = PropVariantClear(&mut value);
        Ok((id, take_string(name?)?))
    }
}

pub(crate) fn inputs() -> Result<Vec<(String, String)>> {
    let _apartment = Apartment::enter()?;
    unsafe {
        let devices = enumerator()?.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
        Ok((0..devices.GetCount()?).filter_map(|i| devices.Item(i).and_then(device_info).ok()).collect())
    }
}

fn capture_format(native_channels: u16) -> Result<WAVEFORMATEX> {
    if native_channels == 0 {
        return Err(E_INVALIDARG.into());
    }
    let channels = native_channels.min(2);
    let align = channels * 2;
    Ok(WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: channels,
        nSamplesPerSec: RATE,
        nAvgBytesPerSec: RATE * u32::from(align),
        nBlockAlign: align,
        wBitsPerSample: 16,
        cbSize: 0,
    })
}

fn duration(frames: u32) -> i64 {
    i64::from(frames) * TICKS / i64::from(RATE)
}

/// 장치 QPC와 WGC의 SystemRelativeTime은 모두 100 ns 단위다.
/// 겹치는 패킷과 시작 전 패킷을 버리고, 시각 오류가 난 패킷은 직전 끝에 잇는다.
fn packet_time(timeline: Timeline, previous_end: Option<i64>, qpc: Option<i64>, now: i64, length: i64) -> Option<i64> {
    let ts = timeline.sample(qpc.unwrap_or(now - length))?;
    let ts = if qpc.is_none() { previous_end.unwrap_or(ts) } else { ts };
    if previous_end.is_some_and(|end| ts.saturating_add(length) <= end) {
        return None;
    }
    Some(ts.max(previous_end.unwrap_or(0)))
}

pub(crate) struct Microphone {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    channels: u16,
    buffer_frames: u32,
    previous_end: Option<i64>,
    pcm: Vec<u8>,
}

impl Microphone {
    // 녹화 스레드의 MTA 안에서만 만들고 읽고 해제한다.
    pub(crate) fn open(id: &str) -> Result<Self> {
        unsafe {
            let devices = enumerator()?;
            let device =
                if id.is_empty() { devices.GetDefaultAudioEndpoint(eCapture, eConsole)? } else { devices.GetDevice(&HSTRING::from(id))? };
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let mix = client.GetMixFormat()?;
            let channels = (*mix).nChannels;
            CoTaskMemFree(Some(mix.cast()));
            let format = capture_format(channels)?;
            // 공유 엔진이 장치 고유 표본률·부동소수점 형식을 48 kHz 정수 PCM으로 변환한다.
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                TICKS / 2,
                0,
                &format,
                None,
            )?;
            let capture = client.GetService()?;
            let buffer_frames = client.GetBufferSize()?;
            Ok(Self { client, capture, channels: format.nChannels, buffer_frames, previous_end: None, pcm: Vec::new() })
        }
    }

    pub(crate) fn start(&self) -> Result<()> {
        unsafe { self.client.Start() }
    }

    pub(crate) fn stop(&self) {
        unsafe {
            let _ = self.client.Stop();
        }
    }

    pub(crate) fn pause(&mut self) -> Result<()> {
        self.previous_end = None;
        unsafe {
            self.client.Stop()?;
            self.client.Reset()
        }
    }

    pub(crate) fn drain(&mut self, writer: &IMFSinkWriter, stream: u32, timeline: Timeline, now: i64) -> Result<()> {
        unsafe {
            // 인코더가 느릴 때 새로 들어오는 소리를 끝없이 따라가지 않고 영상·중지 처리로 돌아간다.
            let mut remaining = self.buffer_frames;
            while remaining > 0 && self.capture.GetNextPacketSize()? != 0 {
                let (mut ptr, mut frames, mut flags, mut qpc) = (std::ptr::null_mut(), 0, 0, 0);
                self.capture.GetBuffer(&mut ptr, &mut frames, &mut flags, None, Some(&mut qpc))?;
                if frames == 0 {
                    break;
                }
                let length = duration(frames);
                let valid = flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 == 0;
                let ts = packet_time(timeline, self.previous_end, valid.then_some(qpc as i64), now, length);
                if ts.is_some() {
                    self.pcm.clear();
                    if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || ptr.is_null() {
                        self.pcm.resize(frames as usize * FRAME_BYTES, 0);
                    } else {
                        let bytes = std::slice::from_raw_parts(ptr, frames as usize * self.channels as usize * 2);
                        if self.channels == 1 {
                            for sample in bytes.chunks_exact(2) {
                                self.pcm.extend_from_slice(sample);
                                self.pcm.extend_from_slice(sample);
                            }
                        } else {
                            self.pcm.extend_from_slice(bytes);
                        }
                    }
                }
                // 장치 버퍼를 반납한 다음에만 시간이 걸릴 수 있는 인코더를 부른다.
                self.capture.ReleaseBuffer(frames)?;
                remaining = remaining.saturating_sub(frames);
                if let Some(ts) = ts {
                    write_pcm(writer, stream, &self.pcm, ts, length)?;
                    self.previous_end = Some(ts + length);
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn add_stream(writer: &IMFSinkWriter) -> Result<u32> {
    unsafe {
        let audio = |subtype| -> Result<IMFMediaType> {
            let ty = MFCreateMediaType()?;
            ty.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            ty.SetGUID(&MF_MT_SUBTYPE, subtype)?;
            ty.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
            ty.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, RATE)?;
            ty.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
            Ok(ty)
        };
        let aac = audio(&MFAudioFormat_AAC)?;
        aac.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 128_000 / 8)?;
        let stream = writer.AddStream(&aac)?;
        let pcm = audio(&MFAudioFormat_PCM)?;
        pcm.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, FRAME_BYTES as u32)?;
        pcm.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, RATE * FRAME_BYTES as u32)?;
        writer.SetInputMediaType(stream, &pcm, None)?;
        Ok(stream)
    }
}

pub(crate) fn write_pcm(writer: &IMFSinkWriter, stream: u32, bytes: &[u8], ts: i64, length: i64) -> Result<()> {
    unsafe {
        let size = u32::try_from(bytes.len()).map_err(|_| E_INVALIDARG)?;
        let buf = MFCreateMemoryBuffer(size)?;
        let mut ptr = std::ptr::null_mut();
        buf.Lock(&mut ptr, None, None)?;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        buf.Unlock()?;
        buf.SetCurrentLength(size)?;
        let sample = MFCreateSample()?;
        sample.AddBuffer(&buf)?;
        sample.SetSampleTime(ts)?;
        sample.SetSampleDuration(length)?;
        writer.WriteSample(stream, &sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 공유_엔진에_요청하는_형식은_장치_채널에_맞춘_48khz_pcm() {
        for (native, channels) in [(1, 1), (2, 2), (6, 2)] {
            let f = capture_format(native).unwrap();
            let actual = (f.nChannels, f.nSamplesPerSec, f.nBlockAlign, f.nAvgBytesPerSec, f.wBitsPerSample);
            assert_eq!(actual, (channels, 48_000, channels * 2, u32::from(channels) * 96_000, 16));
        }
        assert!(capture_format(0).is_err());
    }

    #[test]
    fn 영상과_같은_원점에서_역행과_장치_시각_오류를_처리한다() {
        let len = duration(480);
        let origin = 5_000_000;
        let clock = Timeline::new(origin);
        assert_eq!(packet_time(clock, None, Some(origin - len), origin, len), None);
        assert_eq!(packet_time(clock, None, Some(origin + 200), origin + len, len), Some(200));
        assert_eq!(packet_time(clock, Some(len), Some(origin), origin + len, len), None);
        assert_eq!(packet_time(clock, Some(len), None, origin + 3 * len, len), Some(len));
        assert_eq!(packet_time(clock, Some(len), Some(origin + 3 * len), origin + 4 * len, len), Some(3 * len));
    }
}
