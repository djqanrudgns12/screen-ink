# M6 live-zoom benchmark for screen-ink (PRD gate G6: 1080p >= 30 fps, record 4K; 07 plan 5.1 / 5.4).
# Procedure and decision rules: bench/m6-zoom.md. NOT run by agents - it shows a full-screen magnifier.
#
# Two modes:
#   -Entry : toggles Alt+2 N times and aggregates the app's "mode enter zoom [N ms] (...)" lines.
#   default: one zoom session of -Seconds while a video plays (scenario S9). Collects
#            (a) PresentMon on screen-ink.exe (may show 0 presents: the Magnification API may not Present from our process),
#            (b) PresentMon on dwm.exe (composition baseline),
#            (c) the app's own "zoom frames N in T s (avg F fps, max gap G ms)" line (frame loop paced by MagSetWindowSource).
#
#   powershell -ExecutionPolicy Bypass -File bench\m6-zoom.ps1 -Entry -Count 20
#   powershell -ExecutionPolicy Bypass -File bench\m6-zoom.ps1 -Seconds 60 -PresentMon C:\tools\PresentMon.exe -AutoZoom -Save -Label 1080p
#
# Requirements: screen-ink running with default Alt+2; for the fps mode a 1080p video playing full screen on the
# monitor under the cursor. Alt+2 is a registered global hotkey, so the OS delivers it to screen-ink only.
param(
    [switch]$Entry,
    [int]$Count = 20,
    [int]$HoldMs = 600,
    [int]$GapMs = 400,
    [int]$Seconds = 60,
    [string]$PresentMon = '',
    [switch]$LegacyFlags,
    [switch]$AutoZoom,
    [string]$Label = '',
    [string]$Log = "$env:LOCALAPPDATA\kr.screenink.desktop\logs\screen-ink.log",
    [string]$OutDir = "$env:TEMP\screen-ink-m6",
    [switch]$Save
)

Add-Type -AssemblyName System.Windows.Forms
if (-not (Get-Process screen-ink -ErrorAction SilentlyContinue)) { throw "screen-ink is not running" }
if (-not (Test-Path $Log)) { throw "log not found: $Log" }
New-Item -ItemType Directory -Force $OutDir | Out-Null

function Pct([double[]]$v, [double]$p) {
    if ($v.Count -eq 0) { return [double]::NaN }
    $s = $v | Sort-Object
    $rank = [math]::Max(1, [math]::Ceiling($p / 100 * $s.Count))
    return $s[[math]::Min($rank, $s.Count) - 1]
}

function New-LogLines([int]$skip) {
    return Get-Content $Log -Encoding UTF8 | Select-Object -Skip $skip
}

function Press-Zoom { [System.Windows.Forms.SendKeys]::SendWait('%2') }

$screens = [System.Windows.Forms.Screen]::AllScreens
$desc = ($screens | ForEach-Object { "$($_.Bounds.Width)x$($_.Bounds.Height)" }) -join ' + '
$before = (Get-Content $Log -Encoding UTF8 | Measure-Object -Line).Lines

if ($Entry) {
    for ($i = 1; $i -le $Count; $i++) {
        Press-Zoom
        Start-Sleep -Milliseconds $HoldMs
        Press-Zoom
        Start-Sleep -Milliseconds $GapMs
        if ($i % 5 -eq 0) { Write-Host "  $i / $Count" }
    }
    Start-Sleep -Milliseconds 500
    $total = @(); $mag = @()
    foreach ($l in (New-LogLines $before)) {
        if ($l -match 'mode enter zoom \[(\d+) ms\] \(probe (\d+), magnifier (\d+)') {
            $total += [double]$Matches[1]; $mag += [double]$Matches[3]
        }
    }
    $rows = @(
        [pscustomobject]@{ metric = 'entry (Alt+2 -> magnifier shown)'; n = $total.Count; p50 = (Pct $total 50); p95 = (Pct $total 95); max = ($total | Measure-Object -Maximum).Maximum },
        [pscustomobject]@{ metric = 'magnifier start'; n = $mag.Count; p50 = (Pct $mag 50); p95 = (Pct $mag 95); max = ($mag | Measure-Object -Maximum).Maximum }
    )
    $rows | Format-Table -AutoSize
    $verdict = if ($total.Count -lt $Count * 0.9) { "incomplete ($($total.Count)/$Count samples)" } elseif ((Pct $total 95) -le 150) { 'p95 <= 150 ms (reference target)' } else { 'p95 > 150 ms (reference target)' }
    Write-Host "verdict: $verdict"
    if ($Save) {
        $day = Get-Date -Format 'yyyy-MM-dd'
        $md = @("# M6 live-zoom entry - $day", '', "- machine: (fill in: CPU / GPU model only), monitors $desc", "- samples: $($total.Count) / $Count (hold $HoldMs ms, gap $GapMs ms)", "- verdict: $verdict", '', '| metric | n | p50 | p95 | max |', '|---|---|---|---|---|')
        foreach ($r in $rows) { $md += "| $($r.metric) | $($r.n) | $($r.p50) | $($r.p95) | $($r.max) |" }
        $path = Join-Path (Split-Path $PSScriptRoot) "bench\results\$day-m6-zoom-entry.md"
        $md -join "`n" | Out-File -Encoding utf8 $path
        Write-Host "saved $path"
    }
    return
}

# ---- fps mode ----
if ($AutoZoom) { Press-Zoom; Start-Sleep -Milliseconds 800 }
else { Read-Host 'Press Alt+2 over the playing video, then press Enter here' | Out-Null }

$csvApp = Join-Path $OutDir 'presentmon-screen-ink.csv'
$csvDwm = Join-Path $OutDir 'presentmon-dwm.csv'
$jobs = @()
if ($PresentMon -and (Test-Path $PresentMon)) {
    foreach ($t in @(@{ name = 'screen-ink.exe'; csv = $csvApp; session = 'ink_m6_app' }, @{ name = 'dwm.exe'; csv = $csvDwm; session = 'ink_m6_dwm' })) {
        if (Test-Path $t.csv) { Remove-Item $t.csv -Force }
        if ($LegacyFlags) {
            $pmArgs = @('-process_name', $t.name, '-timed', "$Seconds", '-terminate_after_timed', '-output_file', $t.csv, '-session_name', $t.session, '-stop_existing_session')
        } else {
            $pmArgs = @('--process_name', $t.name, '--timed', "$Seconds", '--terminate_after_timed', '--output_file', $t.csv, '--session_name', $t.session, '--stop_existing_session', '--v1_metrics')
        }
        $jobs += Start-Process -FilePath $PresentMon -ArgumentList $pmArgs -PassThru -WindowStyle Hidden
    }
    $jobs | Wait-Process -Timeout ($Seconds + 30) -ErrorAction SilentlyContinue
} else {
    Write-Host "PresentMon not given - collecting the app log line only for $Seconds s"
    Start-Sleep -Seconds $Seconds
}

if ($AutoZoom) { Press-Zoom } else { Read-Host 'Press Alt+2 (or Esc) to end the zoom, then press Enter here' | Out-Null }
Start-Sleep -Milliseconds 800

function Read-PresentMon([string]$csv) {
    if (-not (Test-Path $csv)) { return $null }
    $rows = Import-Csv $csv
    if (-not $rows -or $rows.Count -eq 0) { return [pscustomobject]@{ presents = 0; fps = 0; p95ms = [double]::NaN } }
    $col = @('MsBetweenPresents', 'FrameTime', 'MsBetweenDisplayChange') | Where-Object { $rows[0].PSObject.Properties.Name -contains $_ } | Select-Object -First 1
    $ft = @()
    if ($col) { $ft = $rows | ForEach-Object { [double]$_.$col } | Where-Object { $_ -gt 0 } }
    $sum = ($ft | Measure-Object -Sum).Sum
    $fps = if ($sum -gt 0) { [math]::Round($ft.Count / ($sum / 1000), 1) } else { [math]::Round($rows.Count / $Seconds, 1) }
    return [pscustomobject]@{ presents = $rows.Count; fps = $fps; p95ms = (Pct $ft 95) }
}

$app = Read-PresentMon $csvApp
$dwm = Read-PresentMon $csvDwm
$frames = $null
foreach ($l in (New-LogLines $before)) {
    if ($l -match 'zoom frames (\d+) in ([\d\.]+) s \(avg ([\d\.]+) fps, max gap (\d+) ms\)') {
        $frames = [pscustomobject]@{ frames = [int]$Matches[1]; secs = [double]$Matches[2]; fps = [double]$Matches[3]; maxGap = [int]$Matches[4] }
    }
}

$rows = @()
if ($app) { $rows += [pscustomobject]@{ source = 'PresentMon screen-ink.exe'; fps = $app.fps; p95_ms = $app.p95ms; note = "$($app.presents) presents" } }
if ($dwm) { $rows += [pscustomobject]@{ source = 'PresentMon dwm.exe (baseline)'; fps = $dwm.fps; p95_ms = $dwm.p95ms; note = "$($dwm.presents) presents" } }
if ($frames) { $rows += [pscustomobject]@{ source = 'app frame loop (log)'; fps = $frames.fps; p95_ms = [double]::NaN; note = "$($frames.frames) frames / $($frames.secs) s, max gap $($frames.maxGap) ms" } }
$rows | Format-Table -AutoSize

# G6 uses the app frame loop when PresentMon sees no presents from screen-ink.exe (see m6-zoom.md section 3).
$gate = $null
if ($app -and $app.presents -gt 0) { $gate = $app.fps } elseif ($frames) { $gate = $frames.fps }
$verdict = if ($null -eq $gate) { 'no data' } elseif ($gate -ge 30) { "PASS (>= 30 fps, $gate)" } else { "MISS (< 30 fps, $gate)" }
Write-Host "monitors: $desc"
Write-Host "verdict ($Label): $verdict"

if ($Save) {
    $day = Get-Date -Format 'yyyy-MM-dd'
    $md = @("# M6 live-zoom fps - $day $Label", '', "- monitors: $desc", "- duration: $Seconds s, scenario S9 (video playing under the zoom)", "- verdict: $verdict", '', '| source | fps | p95 frame ms | note |', '|---|---|---|---|')
    foreach ($r in $rows) { $md += "| $($r.source) | $($r.fps) | $($r.p95_ms) | $($r.note) |" }
    $suffix = if ($Label) { "-$Label" } else { '' }
    $path = Join-Path (Split-Path $PSScriptRoot) "bench\results\$day-m6-zoom$suffix.md"
    $md -join "`n" | Out-File -Encoding utf8 $path
    Write-Host "saved $path"
}
