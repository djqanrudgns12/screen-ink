//! 공용 D3D11 장치(단계 6): F2 창 캡처(WGC)와 F3 녹화(WGC + Media Foundation)가 같은 장치를 쓴다(spikes §2 추천 3).
//!
//! - 한 번 만들어 재사용(spikes §1: 만들기 40~75 ms, 드물게 195 ms) — 장치가 제거되면(드라이버 갱신·절전) 다시 만든다.
//! - `BGRA_SUPPORT | VIDEO_SUPPORT` + `SetMultithreadProtected(true)`: MF가 다른 스레드에서 같은 즉시 컨텍스트를 쓴다(spikes §2 함정 5).

use std::sync::{Arc, Mutex};
use windows::core::Interface;
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::{E_FAIL, HMODULE};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D10::ID3D10Multithread;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_FLAG, D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;

pub struct Gpu {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    /// WGC 프레임 풀이 받는 WinRT 장치
    pub winrt: IDirect3DDevice,
    /// 영상 처리(VIDEO_SUPPORT) 장치인지 — 아니면 녹화 D3D 경로를 쓸 수 없다
    pub video: bool,
}

// SAFETY: D3D11 장치는 자유 스레드 객체이고, 즉시 컨텍스트는 `SetMultithreadProtected(true)`로 D3D가 내부 잠금으로
// 직렬화한다(MF 싱크 라이터와 함께 쓰는 문서 규약). WinRT `IDirect3DDevice`는 agile이다.
unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}

static SHARED: Mutex<Option<Arc<Gpu>>> = Mutex::new(None);

/// 공용 장치(없거나 제거됐으면 새로).
pub fn shared() -> Result<Arc<Gpu>, String> {
    let mut slot = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(gpu) = slot.as_ref() {
        // SAFETY: 읽기 전용 질의.
        if unsafe { gpu.device.GetDeviceRemovedReason() }.is_ok() {
            return Ok(gpu.clone());
        }
        log::warn!("d3d11 device removed, recreating");
    }
    let t = std::time::Instant::now();
    let gpu = Arc::new(create().map_err(|e| format!("d3d11 device: {e}"))?);
    log::info!("d3d11 device ready in {:.0} ms (video {})", t.elapsed().as_secs_f64() * 1000.0, gpu.video);
    *slot = Some(gpu.clone());
    Ok(gpu)
}

fn create_with(flags: D3D11_CREATE_DEVICE_FLAG) -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext)> {
    let mut device = None;
    let mut context = None;
    // SAFETY: 출력 두 개를 넘기고, 성공하면 둘 다 채워진다.
    unsafe {
        D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), flags, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context))?;
    }
    match (device, context) {
        (Some(d), Some(c)) => Ok((d, c)),
        _ => Err(E_FAIL.into()),
    }
}

fn create() -> windows::core::Result<Gpu> {
    // 영상 처리 지원이 없는 드라이버(기본 디스플레이 등)면 창 캡처만이라도 되게 빼고 다시
    let (device, context, video) = match create_with(D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT) {
        Ok((d, c)) => (d, c, true),
        Err(_) => {
            let (d, c) = create_with(D3D11_CREATE_DEVICE_BGRA_SUPPORT)?;
            (d, c, false)
        }
    };
    let mt: ID3D10Multithread = device.cast()?;
    // SAFETY: 장치 하나의 다중 스레드 보호를 켠다(이전 값은 버림).
    unsafe {
        let _ = mt.SetMultithreadProtected(true);
    }
    let dxgi: IDXGIDevice = device.cast()?;
    // SAFETY: 살아 있는 DXGI 장치를 WinRT로 감싼다.
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
    let winrt: IDirect3DDevice = inspectable.cast()?;
    Ok(Gpu { device, context, winrt, video })
}
