<#
.SYNOPSIS
  Snipaste 대조 하네스(08 §5.4, C §2.3 3f-snipaste) — 메인이 실제 화면에서 돌린다.
.DESCRIPTION
  사례 JSON의 정답 점마다: 커서를 그 점으로 → 화면 찍기(전) → Snipaste 단축키(F1) → 커서 1 px 흔들기 → 찍기(후) → Esc.
  전·후 PNG를 snipaste-diff(ink-vision::diff_rect, 단위 시험 있음)로 판독해 Snipaste 첫 강조를 얻고,
  같은 점의 screen-ink 재생 결과(recog-replay CSV)와 나란히 CSV + 요약(md)으로 쓴다.

  지킬 것
  - 화면에는 시험 문서만(개인정보 없는 화면). 사례를 뜬 그 화면 그대로(창을 옮기지 않음).
  - screen-ink는 끈다(F1이 겹침). Snipaste 2.11.3 기본 설정, 창·요소 감지 켬.
  - 전·후 PNG는 임시 폴더(%TEMP%\screen-ink-snipaste)에만 쓰고, -Keep이 없으면 끝날 때 지운다(저장소에 넣지 않음).
  - 점 하나에 약 1.3초. 도는 동안 마우스·키보드를 만지지 않는다.

.PARAMETER Case       사례 JSON(bench\recog\cases\*.json). 그 점들과 모니터 배치를 쓴다.
.PARAMETER ReplayCsv  같은 사례의 recog-replay CSV(기본 bench\results\recog-replay.csv). 없으면 Snipaste만 적는다.
.PARAMETER Out        결과 CSV(기본 bench\results\recog-<날짜>-snipaste-<사례 id>.csv). 같은 이름 .md에 요약.
.PARAMETER Inset      Snipaste 테두리가 사각형 안쪽에 그려지는 두께(px). -Calibrate로 한 번 잰다.
.PARAMETER Calibrate  창처럼 크고 확실한 정답 점(RB-04 '제목 표시줄'·'창 빈 곳')이 있는 사례로 어긋남을 재서 -Inset 값을 알려 준다. CSV는 쓰지 않는다.
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File bench\recog\compare-snipaste.ps1 -Case bench\recog\cases\case-20261126-101500-1.json -Calibrate
  powershell -ExecutionPolicy Bypass -File bench\recog\compare-snipaste.ps1 -Case bench\recog\cases\case-20261126-101500-1.json -Inset 1
#>
param(
  [Parameter(Mandatory = $true)][string]$Case,
  [string]$ReplayCsv = "",
  [string]$Out = "",
  [int]$Inset = 0,
  [int]$DelayMs = 450,
  [int]$HotkeyVk = 0x70,
  [switch]$Calibrate,
  [switch]$Keep
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$HIT = 2   # 맞음 = 네 변 모두 ±2 px(replay::HIT_TOLERANCE와 같음)

if (Get-Process -Name 'screen-ink' -ErrorAction SilentlyContinue) { throw 'screen-ink가 켜져 있어요 — F1이 겹치니 끄고 다시 돌려요.' }
if (-not (Get-Process -Name 'Snipaste' -ErrorAction SilentlyContinue)) { throw 'Snipaste가 꺼져 있어요 — 켜고(기본 설정) 다시 돌려요.' }

# 판독 도구 빌드(릴리스)
$manifest = Join-Path $root 'bench\recog\tools\snipaste-diff\Cargo.toml'
& cargo build --release --quiet --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { throw 'snipaste-diff 빌드 실패' }
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'bench\recog\tools\snipaste-diff\target' }
$diffExe = Join-Path $targetDir 'release\snipaste-diff.exe'
if (-not (Test-Path $diffExe)) { throw "판독 도구가 없어요: $diffExe" }

Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
public static class SiHarness {
  [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr ctx);
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  static readonly IntPtr PMV2 = new IntPtr(-4);
  // 물리 px로 커서 옮기기·찍기(이 스레드만 잠깐 Per-Monitor V2)
  public static void Move(int x, int y) {
    IntPtr old = SetThreadDpiAwarenessContext(PMV2);
    try { SetCursorPos(x, y); } finally { if (old != IntPtr.Zero) SetThreadDpiAwarenessContext(old); }
  }
  public static void Capture(int x, int y, int w, int h, string path) {
    IntPtr old = SetThreadDpiAwarenessContext(PMV2);
    try {
      using (var bmp = new Bitmap(w, h, PixelFormat.Format24bppRgb))
      using (var g = Graphics.FromImage(bmp)) {
        g.CopyFromScreen(x, y, 0, 0, new Size(w, h), CopyPixelOperation.SourceCopy);
        bmp.Save(path, ImageFormat.Png);
      }
    } finally { if (old != IntPtr.Zero) SetThreadDpiAwarenessContext(old); }
  }
  public static void Key(byte vk) {
    keybd_event(vk, 0, 0, UIntPtr.Zero);
    System.Threading.Thread.Sleep(30);
    keybd_event(vk, 0, 2, UIntPtr.Zero);
  }
}
'@

$caseObj = [IO.File]::ReadAllText((Resolve-Path $Case).Path, [Text.Encoding]::UTF8) | ConvertFrom-Json
$caseId = $caseObj.id
$monitors = @($caseObj.monitors)
$points = @($caseObj.points)
if ($points.Count -eq 0) { throw '사례에 정답 점이 없어요(recog-truth 또는 rb04-win32 merge로 먼저 채워요).' }

# screen-ink 재생 결과(같은 사례·같은 점)
if (-not $ReplayCsv) { $ReplayCsv = Join-Path $root 'bench\results\recog-replay.csv' }
$replay = @{}
if (Test-Path $ReplayCsv) {
  foreach ($r in (Import-Csv -Path $ReplayCsv -Encoding UTF8)) {
    if ($r.case -eq $caseId) { $replay["$($r.x),$($r.y)"] = $r }
  }
} else { Write-Warning "재생 CSV가 없어요: $ReplayCsv — Snipaste만 적어요" }

$tmp = Join-Path $env:TEMP 'screen-ink-snipaste'
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

function Get-EdgeError($a, $t) {
  $e = @([math]::Abs($a.x - $t.x), [math]::Abs($a.y - $t.y), [math]::Abs(($a.x + $a.w) - ($t.x + $t.w)), [math]::Abs(($a.y + $a.h) - ($t.y + $t.h)))
  return ($e | Measure-Object -Maximum).Maximum
}

$rows = New-Object System.Collections.Generic.List[object]
$i = 0
foreach ($p in $points) {
  $i++
  $x = [int]$p.at.x; $y = [int]$p.at.y
  $m = $monitors | Where-Object { $x -ge $_.bounds.x -and $x -lt ($_.bounds.x + $_.bounds.w) -and $y -ge $_.bounds.y -and $y -lt ($_.bounds.y + $_.bounds.h) } | Select-Object -First 1
  if (-not $m) { Write-Warning "점 ($x,$y)이 모니터 밖 — 건너뜀"; continue }
  $b = $m.bounds
  $before = Join-Path $tmp ("{0:D3}-before.png" -f $i)
  $after = Join-Path $tmp ("{0:D3}-after.png" -f $i)

  [SiHarness]::Move($x, $y); Start-Sleep -Milliseconds 150
  [SiHarness]::Capture($b.x, $b.y, $b.w, $b.h, $before)
  [SiHarness]::Key([byte]$HotkeyVk); Start-Sleep -Milliseconds $DelayMs
  [SiHarness]::Move($x + 1, $y); Start-Sleep -Milliseconds 60
  [SiHarness]::Move($x, $y); Start-Sleep -Milliseconds 150
  [SiHarness]::Capture($b.x, $b.y, $b.w, $b.h, $after)
  [SiHarness]::Key([byte]0x1B); Start-Sleep -Milliseconds 300

  $line = & $diffExe $before $after ($x - $b.x) ($y - $b.y) --inset $Inset
  $sn = $null
  if ($LASTEXITCODE -eq 0 -and $line -ne 'none') {
    $v = $line.Split(',') | ForEach-Object { [int]$_ }
    $sn = [pscustomobject]@{ x = $v[0] + $b.x; y = $v[1] + $b.y; w = $v[2]; h = $v[3] }
  }
  $edge = if ($sn) { Get-EdgeError $sn $p.truth } else { $null }
  $r = $replay["$x,$y"]
  $siSteps = if ($r) { $r.steps } else { '' }
  $rows.Add([pscustomobject]@{
      case = $caseId; label = $p.label; x = $x; y = $y
      truth_x = $p.truth.x; truth_y = $p.truth.y; truth_w = $p.truth.w; truth_h = $p.truth.h
      snipaste_x = $(if ($sn) { $sn.x } else { '' }); snipaste_y = $(if ($sn) { $sn.y } else { '' })
      snipaste_w = $(if ($sn) { $sn.w } else { '' }); snipaste_h = $(if ($sn) { $sn.h } else { '' })
      snipaste_edge = $(if ($null -ne $edge) { $edge } else { '' })
      snipaste_top1 = [int]($null -ne $edge -and $edge -le $HIT)
      screenink_steps = $siSteps
      screenink_top1 = [int]($siSteps -eq '0')
      screenink_top3 = [int]($siSteps -in @('0', '1', '2'))
      d_left = $(if ($sn) { $sn.x - $p.truth.x } else { '' }); d_top = $(if ($sn) { $sn.y - $p.truth.y } else { '' })
    })
  $snText = if ($null -ne $edge) { "경계 $edge px" } else { '못 읽음' }
  $siText = if ($siSteps -ne '') { "칸 $siSteps" } else { '-' }
  Write-Host ("[{0}/{1}] {2} — Snipaste {3} · screen-ink {4}" -f $i, $points.Count, $p.label, $snText, $siText)
}

if (-not $Keep) { Remove-Item -Recurse -Force $tmp }

if ($Calibrate) {
  # 정답 근처(경계 6 px 안)를 잡은 점들의 왼쪽·위 어긋남 중앙값 = 테두리가 안쪽으로 그려진 두께
  $near = @($rows | Where-Object { $_.snipaste_edge -ne '' -and [int]$_.snipaste_edge -le 6 })
  if ($near.Count -eq 0) { throw '보정에 쓸 점이 없어요(Snipaste가 정답 근처를 하나도 못 잡음)' }
  $ds = @($near | ForEach-Object { [int]$_.d_left; [int]$_.d_top } | Sort-Object)
  $median = $ds[[int][math]::Floor($ds.Count / 2)]
  Write-Host "보정: 점 $($near.Count)개, 왼쪽·위 어긋남 중앙값 $median px → 다음 실행에 -Inset $median"
  return
}

$date = Get-Date -Format 'yyyy-MM-dd'
if (-not $Out) { $Out = Join-Path $root "bench\results\recog-$date-snipaste-$caseId.csv" }
$csv = ($rows | Select-Object case, label, x, y, truth_x, truth_y, truth_w, truth_h, snipaste_x, snipaste_y, snipaste_w, snipaste_h, snipaste_edge, snipaste_top1, screenink_steps, screenink_top1, screenink_top3 | ConvertTo-Csv -NoTypeInformation) -join "`r`n"
[IO.File]::WriteAllText($Out, $csv + "`r`n", (New-Object Text.UTF8Encoding $false))

$n = $rows.Count
$snTop1 = @($rows | Where-Object { $_.snipaste_top1 -eq 1 }).Count
$joined = @($rows | Where-Object { $_.screenink_steps -ne '' })
$siTop1 = @($joined | Where-Object { $_.screenink_top1 -eq 1 }).Count
$siTop3 = @($joined | Where-Object { $_.screenink_top3 -eq 1 }).Count
$snFail = @($joined | Where-Object { $_.snipaste_top1 -eq 0 })
$rescued = @($snFail | Where-Object { $_.screenink_top1 -eq 1 }).Count
function Format-Pct($a, $b) { if ($b -eq 0) { return '-' } return ('{0:N1}%' -f (100.0 * $a / $b)) }
$md = @"
# Snipaste 대조 — $caseId ($date)

- 사례 메모: $($caseObj.note)
- 점 $n개 · Snipaste 테두리 보정 -Inset $Inset · 맞음 = 네 변 ±$HIT px

| 지표 | Snipaste | screen-ink(재생) |
|---|---:|---:|
| Top-1 | $(Format-Pct $snTop1 $n) | $(Format-Pct $siTop1 $joined.Count) |
| Top-3 | - | $(Format-Pct $siTop3 $joined.Count) |

- Snipaste가 틀린 점 $($snFail.Count)개 중 screen-ink 첫 강조가 맞힌 점: $rescued개($(Format-Pct $rescued $snFail.Count)) — G4 기준 ≥ 70%
- 점별: $(Split-Path -Leaf $Out)
"@
[IO.File]::WriteAllText(([IO.Path]::ChangeExtension($Out, '.md')), $md, (New-Object Text.UTF8Encoding $false))
Write-Host "CSV: $Out"
Write-Host $md
