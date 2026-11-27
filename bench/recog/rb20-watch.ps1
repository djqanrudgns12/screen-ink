<#
.SYNOPSIS
  RB-20 멈춤 시험(08 §5.1·D17·D19, C §4 "rb20_멈춘앱_UI_굳음_0회") — 메인이 실제 화면에서 돌린다.
.DESCRIPTION
  rb20-frozen(멈춘 시험 앱) 위에서 screen-ink F1 → 커서 이동 N번 → Esc 하는 동안,
  screen-ink UI 스레드(모든 최상위 창)에 SendMessageTimeout(WM_NULL, 100 ms)을 IntervalMs마다 보내 응답 시간을 잰다.
  응답이 100 ms를 넘은 횟수 = UI 굳음 수. 합격 = 0회.
  - 웹뷰(WebView2) 그리기는 다른 프로세스라 여기서는 안 잰다 — 앱 로그 'selection probe'의 moves max도 함께 본다.
  - 창 제목은 읽지 않는다(프로세스 번호로만 창을 찾음).

  순서: rb20-frozen을 먼저 띄우고(예: --mode hang-location), screen-ink를 띄운 뒤 이 스크립트를 돌린다.
  스크립트가 커서를 RB-20 창 가운데로 옮기고 F1을 누른다(-Hotkey 생략 시 F1).

.EXAMPLE
  cargo build --release --manifest-path bench\recog\tools\rb20-frozen\Cargo.toml
  Start-Process "$env:CARGO_TARGET_DIR\release\rb20-frozen.exe" -ArgumentList '--mode','hang-location','--life','90'
  npm run app:dev    # 다른 창에서
  powershell -ExecutionPolicy Bypass -File bench\recog\rb20-watch.ps1 -Moves 50
#>
param(
  [int]$Moves = 50,
  [int]$IntervalMs = 16,
  [int]$MoveGapMs = 60,
  [int]$HotkeyVk = 0x70,
  [switch]$ExitWithHotkey,
  [string]$AppProcess = 'screen-ink',
  [string]$FrozenProcess = 'rb20-frozen'
)
$ErrorActionPreference = 'Stop'

$app = @(Get-Process -Name $AppProcess -ErrorAction SilentlyContinue)
if ($app.Count -eq 0) { throw "$AppProcess 가 꺼져 있어요" }
$frozen = @(Get-Process -Name $FrozenProcess -ErrorAction SilentlyContinue)
if ($frozen.Count -eq 0) { throw "$FrozenProcess 가 꺼져 있어요(먼저 띄워요)" }
if ($app.Count -ne 1 -or $frozen.Count -ne 1) { throw '시험 대상이 여럿이에요. 사본에 고유한 프로세스 이름을 써요.' }

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;
public static class Rb20Watch {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern IntPtr WindowFromPhysicalPoint(POINT p);
  [StructLayout(LayoutKind.Sequential)] struct POINT { public int X, Y; }
  [DllImport("user32.dll")] static extern IntPtr SendMessageTimeout(IntPtr h, uint msg, UIntPtr w, IntPtr l, uint flags, uint timeout, out UIntPtr result);
  [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr ctx);
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }

  public static List<IntPtr> WindowsOf(uint pid, bool visibleOnly) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && (!visibleOnly || IsWindowVisible(h))) list.Add(h); return true; }, IntPtr.Zero);
    return list;
  }
  public static int[] RectOf(IntPtr h) {
    IntPtr old = SetThreadDpiAwarenessContext(new IntPtr(-4));
    try { RECT r; GetWindowRect(h, out r); return new[] { r.Left, r.Top, r.Right - r.Left, r.Bottom - r.Top }; }
    finally { if (old != IntPtr.Zero) SetThreadDpiAwarenessContext(old); }
  }
  public static void Move(int x, int y) {
    IntPtr old = SetThreadDpiAwarenessContext(new IntPtr(-4));
    try { SetCursorPos(x, y); } finally { if (old != IntPtr.Zero) SetThreadDpiAwarenessContext(old); }
  }
  public static void Key(byte vk) { keybd_event(vk, 0, 0, UIntPtr.Zero); Thread.Sleep(30); keybd_event(vk, 0, 2, UIntPtr.Zero); }
  public static uint OwnerAt(int x, int y) {
    uint pid; GetWindowThreadProcessId(WindowFromPhysicalPoint(new POINT { X=x, Y=y }), out pid);
    return pid;
  }

  static volatile bool running;
  static Thread worker;
  static List<double> lat = new List<double>();
  static int timeouts;
  // 200 ms마다 창 목록을 새로 읽어(F1 뒤 생기는 오버레이 포함) 모든 창에 WM_NULL
  public static void Start(uint pid, int intervalMs) {
    running = true; lat.Clear(); timeouts = 0;
    worker = new Thread(() => {
      var sw = new Stopwatch(); var refresh = Stopwatch.StartNew();
      var hs = WindowsOf(pid, false);
      while (running) {
        if (refresh.ElapsedMilliseconds > 200) { hs = WindowsOf(pid, false); refresh.Restart(); }
        double worst = 0;
        foreach (var h in hs) {
          UIntPtr res; sw.Restart();
          IntPtr ok = SendMessageTimeout(h, 0, UIntPtr.Zero, IntPtr.Zero, 0x0002, 100, out res);
          double ms = sw.Elapsed.TotalMilliseconds;
          if (ok == IntPtr.Zero && ms >= 99) { Interlocked.Increment(ref timeouts); }
          if (ms > worst) worst = ms;
        }
        lock (lat) lat.Add(worst);
        Thread.Sleep(intervalMs);
      }
    });
    worker.IsBackground = true; worker.Start();
  }
  public static double[] Stop() {
    running = false; worker.Join();
    lock (lat) {
      var a = lat.ToArray(); Array.Sort(a);
      int over = 0; foreach (var v in a) if (v > 100) over++;
      double p95 = a.Length == 0 ? 0 : a[Math.Min(a.Length - 1, (int)Math.Ceiling(a.Length * 0.95) - 1)];
      double max = a.Length == 0 ? 0 : a[a.Length - 1];
      return new double[] { a.Length, p95, max, over, timeouts };
    }
  }
}
'@

$fw = [Rb20Watch]::WindowsOf([uint32]$frozen[0].Id, $true)
if ($fw.Count -eq 0) { throw 'RB-20 창이 보이지 않아요' }
$r = [Rb20Watch]::RectOf($fw[0])
$cx = $r[0] + [int]($r[2] / 2); $cy = $r[1] + [int]($r[3] / 2)
if ([Rb20Watch]::OwnerAt($cx, $cy) -ne $frozen[0].Id) { throw '시험 창 가운데가 다른 창에 가려졌어요.' }
$before = @([Rb20Watch]::WindowsOf([uint32]$app[0].Id, $true))

[Rb20Watch]::Move($cx, $cy); Start-Sleep -Milliseconds 200
[Rb20Watch]::Start([uint32]$app[0].Id, $IntervalMs)
Start-Sleep -Milliseconds 300
[Rb20Watch]::Key([byte]$HotkeyVk)   # F1 진입
Start-Sleep -Milliseconds 400
$overlays = @([Rb20Watch]::WindowsOf([uint32]$app[0].Id, $true) | Where-Object {
  $box = [Rb20Watch]::RectOf($_)
  $_ -notin $before -and $cx -ge $box[0] -and $cx -lt ($box[0]+$box[2]) -and $cy -ge $box[1] -and $cy -lt ($box[1]+$box[3])
})
if ($overlays.Count -eq 0) {
  [void][Rb20Watch]::Stop()
  throw '단축키를 보냈지만 시험 앱의 캡처 창이 나타나지 않았어요.'
}
for ($k = 0; $k -lt $Moves; $k++) {
  # RB-20 창 안을 지그재그로
  $fx = 0.15 + 0.7 * (($k * 7) % 10) / 9.0
  $fy = 0.15 + 0.7 * (($k * 3) % 10) / 9.0
  [Rb20Watch]::Move($r[0] + [int]($r[2] * $fx), $r[1] + [int]($r[3] * $fy))
  Start-Sleep -Milliseconds $MoveGapMs
}
$exitKey = if ($ExitWithHotkey) { $HotkeyVk } else { 0x1B }
[Rb20Watch]::Key([byte]$exitKey)
Start-Sleep -Milliseconds 400
$s = [Rb20Watch]::Stop()
$stillOpen = @([Rb20Watch]::WindowsOf([uint32]$app[0].Id, $true) | Where-Object { $_ -in $overlays })

$verdict = if ($s[3] -eq 0 -and $stillOpen.Count -eq 0) { '통과' } else { '실패' }
Write-Host ("RB-20 UI 응답: 잰 횟수 {0} · p95 {1:N1} ms · 최대 {2:N1} ms · 100 ms 초과 {3}회(시간 초과 {4}) → {5}" -f $s[0], $s[1], $s[2], $s[3], $s[4], $verdict)
Write-Host "앱 로그에서도 확인: 'selection probe' moves max < 100 ms, 'msaa deadline … exceeded'(hang-location 모드면 1번 나와야 정상)"
if ($stillOpen.Count -gt 0) { throw '측정한 캡처 창이 종료 키를 보낸 뒤에도 열려 있어요.' }
if ($s[3] -ne 0) { exit 1 }
