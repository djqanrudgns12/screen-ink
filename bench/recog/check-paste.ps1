<#
.SYNOPSIS
  붙여 넣기 확인(3e-paste, R-CAP-7) — 한글(HWP)·PowerPoint에 붙여 넣은 그림을 PNG로 다시 내보낸 파일을 본다.
.DESCRIPTION
  기준 = bench\recog\pages\paste-pattern.html(400×200 물리 px, 1 px 검은 테두리, 색 칸 #FF9F0A · #2F6FED · #2E9E6A).
  - 크기: 400×200이면 "원본 그대로". 다르면 배율(가로·세로)과 비율 차를 보인다(앱이 늘려 내보냈을 수 있음).
  - 색: 세 색 칸 가운데 화소가 정확히 같은지(채널 차 0이 합격, 1~2는 색 관리 흔들림으로 표시).
  - 뒤집힘: 맨 위 줄·맨 아래 줄이 검은 테두리인지, 왼쪽부터 주황·파랑·초록 순서인지(DIB 위아래 뒤집힘·BGR 뒤바뀜 확인).
  화면을 건드리지 않는다(파일만 읽음).
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File bench\recog\check-paste.ps1 -Png "$env:TEMP\paste-hwp.png" -App 한글
#>
param(
  [Parameter(Mandatory = $true)][string]$Png,
  [string]$App = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap((Resolve-Path $Png).Path)
try {
  $w = $bmp.Width; $h = $bmp.Height
  $sx = $w / 400.0; $sy = $h / 200.0
  $at = { param($x, $y) $bmp.GetPixel([int][math]::Min($w - 1, [math]::Floor($x * $sx)), [int][math]::Min($h - 1, [math]::Floor($y * $sy))) }
  $want = @(
    @{ name = '주황 #FF9F0A'; x = 80; y = 100; r = 0xFF; g = 0x9F; b = 0x0A },
    @{ name = '파랑 #2F6FED'; x = 200; y = 100; r = 0x2F; g = 0x6F; b = 0xED },
    @{ name = '초록 #2E9E6A'; x = 320; y = 100; r = 0x2E; g = 0x9E; b = 0x6A }
  )
  $ok = $true
  $lines = @()
  $sizeText = if ($w -eq 400 -and $h -eq 200) { '400×200 그대로' } else { ('{0}×{1} (가로 {2:N3}배 · 세로 {3:N3}배 · 비율 차 {4:N2}%)' -f $w, $h, $sx, $sy, (100.0 * [math]::Abs($sx - $sy) / $sx)) }
  if ([math]::Abs($sx - $sy) / $sx -gt 0.005) { $ok = $false }
  $lines += "크기: $sizeText"
  foreach ($c in $want) {
    $p = & $at $c.x $c.y
    $d = @([math]::Abs($p.R - $c.r), [math]::Abs($p.G - $c.g), [math]::Abs($p.B - $c.b)) | Measure-Object -Maximum
    $tag = if ($d.Maximum -eq 0) { '같음' } elseif ($d.Maximum -le 2) { "차 $($d.Maximum)(색 관리 흔들림?)" } else { "다름 차 $($d.Maximum)" }
    if ($d.Maximum -gt 2) { $ok = $false }
    $lines += ('{0}: #{1:X2}{2:X2}{3:X2} → {4}' -f $c.name, $p.R, $p.G, $p.B, $tag)
  }
  # 테두리(위·아래 줄 가운데)와 순서 — 뒤집힘·BGR 확인
  $top = & $at 200 0; $bottom = & $at 200 199.5
  $border = ($top.R + $top.G + $top.B) -le 30 -and ($bottom.R + $bottom.G + $bottom.B) -le 30
  if (-not $border) { $ok = $false }
  $lines += "테두리(위·아래): " + $(if ($border) { '검정 — 뒤집힘 없음' } else { '검정 아님 — 잘림·뒤집힘 확인' })
  $verdict = if ($ok) { '합격' } else { '확인 필요' }
  $head = if ($App) { "[$App] " } else { '' }
  Write-Host "$head$verdict"
  $lines | ForEach-Object { Write-Host "  $_" }
  if (-not $ok) { exit 1 }
} finally { $bmp.Dispose() }
