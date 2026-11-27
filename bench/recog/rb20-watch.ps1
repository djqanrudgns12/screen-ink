<#
.SYNOPSIS
  RB-20 멈춤 시험(08 §5.1·D17·D19, C §4 "rb20_멈춘앱_UI_굳음_0회") — 메인이 실제 화면에서 돌린다.
.DESCRIPTION
  rb20-frozen(멈춘 시험 앱) 위에서 screen-ink F1 → 커서 이동 N번 → Esc 하는 동안,
  screen-ink UI 스레드(모든 최상위 창)에 SendMessageTimeout(WM_NULL, 100 ms)을 IntervalMs마다 보내 응답 시간을 잰다.
  한 차례에 검사한 창 중 가장 오래 걸린 호출이 한 표본이다. 진입·이동·종료 경계 양쪽의 구간과
  최대 호출의 스레드·창 클래스를 남긴다. 합격은 133표본 이상, 100 ms 초과와 메시지 시간 초과 모두 0회다.
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
  [ValidateRange(1, 2147483647)][int]$Moves = 50,
  [ValidateRange(1, 2147483647)][int]$IntervalMs = 16,
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
using System.Text;
using System.Threading;
public static class Rb20Watch {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder name, int count);
  [DllImport("user32.dll")] static extern IntPtr WindowFromPhysicalPoint(POINT p);
  [StructLayout(LayoutKind.Sequential)] struct POINT { public int X, Y; }
  [DllImport("user32.dll", EntryPoint="SendMessageTimeoutW", ExactSpelling=true, SetLastError=true)] static extern IntPtr SendMessageTimeout(IntPtr h, uint msg, UIntPtr w, IntPtr l, uint flags, uint timeout, out UIntPtr result);
  [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr ctx);
  [DllImport("user32.dll")] static extern bool GetPhysicalCursorPos(out POINT p);
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
  public static int[] Cursor() {
    POINT p;
    return GetPhysicalCursorPos(out p) ? new[] { p.X, p.Y } : null;
  }
  public static void Key(byte vk) { keybd_event(vk, 0, 0, UIntPtr.Zero); Thread.Sleep(30); keybd_event(vk, 0, 2, UIntPtr.Zero); }
  public static uint OwnerAt(int x, int y) {
    uint pid; GetWindowThreadProcessId(WindowFromPhysicalPoint(new POINT { X=x, Y=y }), out pid);
    return pid;
  }

  static volatile bool running;
  static Thread worker;
  static readonly Stopwatch clock = new Stopwatch();
  static volatile string phase;
  public static double ElapsedMs { get { return clock.Elapsed.TotalMilliseconds; } }
  public static readonly List<string> SlowCalls = new List<string>();
  static readonly List<Sample> samples = new List<Sample>();
  public class Sample {
    public string From, To, WindowClass;
    public uint ThreadId;
    public double StartMs, Ms;
    public int Timeouts, Failures;
  }
  public static void Mark(string next) { phase = next; }
  // 200 ms마다 창 목록을 새로 읽어(F1 뒤 생기는 오버레이 포함) 모든 창에 WM_NULL
  public static void Start(uint pid, int intervalMs) {
    running = true; samples.Clear(); SlowCalls.Clear(); phase = "대기"; clock.Restart();
    worker = new Thread(() => {
      var sw = new Stopwatch(); var refresh = Stopwatch.StartNew();
      var hs = WindowsOf(pid, false);
      while (running) {
        if (refresh.ElapsedMilliseconds > 200) { hs = WindowsOf(pid, false); refresh.Restart(); }
        var sample = new Sample();
        IntPtr worst = IntPtr.Zero;
        foreach (var h in hs) {
          uint owner;
          uint tid = GetWindowThreadProcessId(h, out owner);
          if (owner != pid) continue;
          string from = phase;
          double start = clock.Elapsed.TotalMilliseconds;
          UIntPtr res; sw.Restart();
          IntPtr ok = SendMessageTimeout(h, 0, UIntPtr.Zero, IntPtr.Zero, 0x0002, 100, out res);
          int error = ok == IntPtr.Zero ? Marshal.GetLastWin32Error() : 0;
          double ms = sw.Elapsed.TotalMilliseconds;
          string to = phase;
          bool timeout = false;
          if (ok == IntPtr.Zero) {
            timeout = error == 1460 || ms >= 99;
            if (timeout) sample.Timeouts++;
            else {
              // 열거 뒤 사라진 창만 건너뛴다. 살아 있는 창의 즉시 반환 실패도 남긴다.
              if (GetWindowThreadProcessId(h, out owner) != tid || owner != pid) continue;
              sample.Failures++;
            }
          }
          if (ms > 100 || ok == IntPtr.Zero) {
            var name = new StringBuilder(128);
            GetClassName(h, name, name.Capacity);
            SlowCalls.Add(String.Format("{0}→{1} +{2:F1}~{3:F1} ms · 응답 {4:F1} ms · 스레드 {5}, 클래스 {6}, {7}",
              from, to, start, start + ms, ms, tid, name, timeout ? "시간 초과" : ok == IntPtr.Zero ? "호출 실패" : "응답 도착"));
          }
          if (worst == IntPtr.Zero || ms > sample.Ms) {
            worst = h; sample.ThreadId = tid; sample.StartMs = start; sample.Ms = ms;
            sample.From = from; sample.To = to;
          }
        }
        if (worst != IntPtr.Zero) {
          var name = new StringBuilder(128);
          GetClassName(worst, name, name.Capacity);
          sample.WindowClass = name.ToString();
          samples.Add(sample);
        }
        Thread.Sleep(intervalMs);
      }
    });
    worker.IsBackground = true; worker.Start();
  }
  public static Sample[] Stop() {
    running = false; worker.Join();
    return samples.ToArray();
  }
}
'@

$fw = [Rb20Watch]::WindowsOf([uint32]$frozen[0].Id, $true)
if ($fw.Count -eq 0) { throw 'RB-20 창이 보이지 않아요' }
$r = [Rb20Watch]::RectOf($fw[0])
$cx = $r[0] + [int]($r[2] / 2); $cy = $r[1] + [int]($r[3] / 2)
if ([Rb20Watch]::OwnerAt($cx, $cy) -ne $frozen[0].Id) { throw '시험 창 가운데가 다른 창에 가려졌어요.' }
$before = @([Rb20Watch]::WindowsOf([uint32]$app[0].Id, $true))

$cursor = [Rb20Watch]::Cursor()
$watching = $false
try {
  [Rb20Watch]::Move($cx, $cy); Start-Sleep -Milliseconds 200
  [Rb20Watch]::Start([uint32]$app[0].Id, $IntervalMs)
  $watching = $true
  Start-Sleep -Milliseconds 300
  [Rb20Watch]::Mark('진입')
  $entryMs = [Rb20Watch]::ElapsedMs
  [Rb20Watch]::Key([byte]$HotkeyVk)   # F1 진입
  Start-Sleep -Milliseconds 400
  $overlays = @([Rb20Watch]::WindowsOf([uint32]$app[0].Id, $true) | Where-Object {
    $box = [Rb20Watch]::RectOf($_)
    $_ -notin $before -and $cx -ge $box[0] -and $cx -lt ($box[0]+$box[2]) -and $cy -ge $box[1] -and $cy -lt ($box[1]+$box[3])
  })
  if ($overlays.Count -eq 0) {
    throw '단축키를 보냈지만 시험 앱의 캡처 창이 나타나지 않았어요.'
  }
  [Rb20Watch]::Mark('이동')
  $moveMs = [Rb20Watch]::ElapsedMs
  for ($k = 0; $k -lt $Moves; $k++) {
    # RB-20 창 안을 지그재그로
    $fx = 0.15 + 0.7 * (($k * 7) % 10) / 9.0
    $fy = 0.15 + 0.7 * (($k * 3) % 10) / 9.0
    [Rb20Watch]::Move($r[0] + [int]($r[2] * $fx), $r[1] + [int]($r[3] * $fy))
    Start-Sleep -Milliseconds $MoveGapMs
  }
  $exitKey = if ($ExitWithHotkey) { $HotkeyVk } else { 0x1B }
  [Rb20Watch]::Mark('종료')
  $exitMs = [Rb20Watch]::ElapsedMs
  [Rb20Watch]::Key([byte]$exitKey)
  Start-Sleep -Milliseconds 400
} finally {
  if ($watching) { $s = [Rb20Watch]::Stop() }
  if ($cursor) { [Rb20Watch]::Move($cursor[0], $cursor[1]) }
}
$stillOpen = @([Rb20Watch]::WindowsOf([uint32]$app[0].Id, $true) | Where-Object { $_ -in $overlays })

if (!$s.Count) { throw '응답을 잰 창이 없어요.' }
$sorted = @($s | Sort-Object Ms)
$over = @($s | Where-Object Ms -gt 100).Count
$timeouts = ($s | Measure-Object -Property Timeouts -Sum).Sum
$failures = ($s | Measure-Object -Property Failures -Sum).Sum
$verdict = if ($s.Count -lt 133) { '표본 부족' } elseif (!$over -and !$timeouts -and !$failures -and !$stillOpen.Count) { '통과' } else { '실패' }
Write-Host ("감시 시작 기준 진입 +{0:N1} ms · 이동 +{1:N1} ms · 종료 +{2:N1} ms" -f $entryMs, $moveMs, $exitMs)
foreach ($call in [Rb20Watch]::SlowCalls) { Write-Host $call }
foreach ($group in ($s | Group-Object { if ($_.From -eq $_.To) { $_.From } else { "$($_.From)→$($_.To)" } })) {
  $peak = $group.Group | Sort-Object Ms -Descending | Select-Object -First 1
  Write-Host ("{0} · {1}표본 · 최대 {2:N1} ms (감시 시작 +{3:N1} ms, 스레드 {4}, 클래스 {5}) · 100 ms 초과 {6}회" -f $group.Name, $group.Count, $peak.Ms, $peak.StartMs, $peak.ThreadId, $peak.WindowClass, @($group.Group | Where-Object Ms -gt 100).Count)
}
$p95 = $sorted[[int][Math]::Ceiling($s.Count * 0.95) - 1].Ms
Write-Host ("RB-20 UI 응답: 잰 횟수 {0} · p95 {1:N1} ms · 최대 {2:N1} ms · 100 ms 초과 {3}회(시간 초과 {4}, 그 밖의 호출 실패 {5}) → {6}" -f $s.Count, $p95, $sorted[-1].Ms, $over, $timeouts, $failures, $verdict)
Write-Host "앱 로그에서도 확인: 'selection probe' moves max < 100 ms, 'msaa deadline … exceeded'(hang-location 모드면 1번 나와야 정상)"
if ($stillOpen.Count -gt 0) { throw '측정한 캡처 창이 종료 키를 보낸 뒤에도 열려 있어요.' }
if ($verdict -ne '통과') { exit 1 }
