# Windows 데스크톱을 맥 미니에서 다루게 하기 (2026-11-27)

목적: 슉쇽의 Windows 쪽(빌드·시험·MSIX·실기)을 맥 미니의 에이전트가 직접 하게 한다.
지금은 맥에서 Windows 코드를 컴파일 검사까지만 할 수 있고(`scripts/check-windows.sh`), 실제 빌드·실행·화면 확인은 못 한다.

## 무엇을 열면 무엇이 되나

| 여는 것 | 에이전트가 할 수 있게 되는 것 |
|---|---|
| **SSH (필수)** | 소스 받기·빌드(`cargo`·`npm`)·시험·MSIX 만들기·앱을 사용자 화면에 띄우기·기록 읽기·스크린숏 파일로 받기 |
| **화면 원격(선택, 권장)** | 트레이·단축키·혼합 배율·첫 실행 화면을 눈으로 확인하고 클릭(맥의 원격 창을 에이전트가 조작) |

화면 원격은 **실제 모니터 화면을 그대로 보여 주는 방식(RustDesk)** 을 권장한다. Windows 원격 데스크톱(RDP)은 접속하면 가상 화면으로 바뀌어 실제 모니터 배율·두 모니터 시험이 안 된다(RDP는 빌드·간단 확인용으로만).

## 1. Windows에서 (관리자 PowerShell, 한 번만)

```powershell
# OpenSSH 서버 켜기
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
Start-Service sshd
Set-Service -Name sshd -StartupType Automatic
# 접속하면 PowerShell이 뜨게
New-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name DefaultShell -Value "C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe" -PropertyType String -Force
# 이 PC의 주소와 이름 확인(맥 쪽에 알려 줄 값)
ipconfig | Select-String IPv4
$env:USERNAME; hostname
```

- 방화벽 규칙(포트 22, 같은 망만)은 설치할 때 자동으로 생긴다.
- **절전 끄기**: 설정 › 시스템 › 전원 › 화면·절전 → 절전 "안 함"(잠자면 접속이 끊긴다). 화면 끄기는 괜찮다.
- 공유기에서 이 PC 주소를 고정(DHCP 예약)해 두면 주소가 바뀌지 않는다.

## 2. 접속 키 등록 (비밀번호 없이 접속)

1. 맥에서 에이전트가 이 PC 전용 키를 만들고 **공개 키 한 줄**을 알려 준다(비밀 키는 맥 밖으로 나가지 않는다).
2. Windows 관리자 PowerShell에 붙여 넣는다:

```powershell
$k = '<에이전트가 준 공개 키 한 줄>'
Add-Content -Path C:\ProgramData\ssh\administrators_authorized_keys -Value $k
icacls C:\ProgramData\ssh\administrators_authorized_keys /inheritance:r /grant "Administrators:F" /grant "SYSTEM:F"
```

(관리자가 아닌 계정이면 `C:\Users\<이름>\.ssh\authorized_keys`에 넣는다.)

3. 맥 쪽 `~/.ssh/config`에 `Host win-desk`(주소·사용자·키)를 에이전트가 적고 `ssh win-desk hostname`으로 확인한다.

## 3. 화면 원격(선택)

- 두 쪽에 RustDesk를 설치하고 Windows 쪽 설정에서 "직접 IP 접속 허용"과 고정 비밀번호를 켠다(같은 망 직접 연결, 바깥 서버 안 거침).
- 맥에서 RustDesk 창을 띄워 두면 에이전트가 화면 제어 도구로 그 창을 조작한다(앱별 승인 1회).

## 4. 그다음은 에이전트가 SSH로 한다

- 개발 도구 설치(winget): Git, Rust(rustup, MSVC), Node LTS, Visual Studio Build Tools(C++), WebView2 확인.
- 저장소 받기 → `cargo test`(Windows 대상)·`npm test` → `cargo tauri build` → MSIX(`scripts/package-msix.ps1`).
- 앱을 **로그인한 사용자 화면**에 띄우기: SSH 세션은 화면이 없으므로 `schtasks /create ... /it` + `schtasks /run`으로 실행하고, 기록(`%LOCALAPPDATA%\...\Shookshok.log`)과 스크린숏 파일을 받아 본다.
- 남은 Windows 실기: 2차 확대(고른 배율·포인터 지도), 창 항상 위, 혼합 배율, MSIX 설치·업데이트(R2-OS-2, 11-distribution).

## 보안 메모

- 키 접속만 쓰고, 같은 집 망 안에서만 연다(공유기 포트 포워딩 하지 않음).
- 그만 쓰려면: `Stop-Service sshd; Set-Service sshd -StartupType Disabled`, `administrators_authorized_keys`에서 그 줄 삭제.
