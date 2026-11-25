# M1 capture-entry benchmark for screen-ink (PRD R-CAP-2: p50 <= 80 ms, p95 <= 150 ms, capture p50 <= 55 ms).
# Presses the region hotkey (F1) to enter, then again to leave, N times, and aggregates the app's own
# "overlay ready [N ms] (capture N, show N, ...)" log lines written during this run.
# Requirements: screen-ink is running with the default F1 key. Nothing else is sent to other apps:
# F1 is a registered global hotkey, so the OS delivers it to screen-ink only.
#
#   powershell -ExecutionPolicy Bypass -File bench\m1-overlay.ps1 -Count 100
#   powershell -ExecutionPolicy Bypass -File bench\m1-overlay.ps1 -Count 100 -Save
param(
    [int]$Count = 100,
    [int]$HoldMs = 350,
    [int]$GapMs = 350,
    [string]$Log = "$env:LOCALAPPDATA\kr.screenink.desktop\logs\screen-ink.log",
    [switch]$Save
)

Add-Type -AssemblyName System.Windows.Forms
if (-not (Get-Process screen-ink -ErrorAction SilentlyContinue)) { throw "screen-ink is not running" }
if (-not (Test-Path $Log)) { throw "log not found: $Log" }

$before = (Get-Content $Log -Encoding UTF8 | Measure-Object -Line).Lines
for ($i = 1; $i -le $Count; $i++) {
    [System.Windows.Forms.SendKeys]::SendWait('{F1}')
    Start-Sleep -Milliseconds $HoldMs
    [System.Windows.Forms.SendKeys]::SendWait('{F1}')
    Start-Sleep -Milliseconds $GapMs
    if ($i % 20 -eq 0) { Write-Host "  $i / $Count" }
}
Start-Sleep -Milliseconds 500

$lines = Get-Content $Log -Encoding UTF8 | Select-Object -Skip $before | Where-Object { $_ -match 'overlay ready \[' }
$total = @(); $capture = @(); $show = @(); $wins = @()
foreach ($l in $lines) {
    if ($l -match 'overlay ready \[(\d+) ms\] \(capture (\d+), show (\d+), windows (\d+) ms') {
        $total += [double]$Matches[1]; $capture += [double]$Matches[2]; $show += [double]$Matches[3]; $wins += [double]$Matches[4]
    }
}

function Pct([double[]]$v, [double]$p) {
    if ($v.Count -eq 0) { return [double]::NaN }
    $s = $v | Sort-Object
    $rank = [math]::Max(1, [math]::Ceiling($p / 100 * $s.Count))
    return $s[[math]::Min($rank, $s.Count) - 1]
}

$rows = @(
    [pscustomobject]@{ metric = 'entry (hotkey -> painted)'; n = $total.Count; p50 = (Pct $total 50); p90 = (Pct $total 90); p95 = (Pct $total 95); max = ($total | Measure-Object -Maximum).Maximum },
    [pscustomobject]@{ metric = 'capture (GDI)'; n = $capture.Count; p50 = (Pct $capture 50); p90 = (Pct $capture 90); p95 = (Pct $capture 95); max = ($capture | Measure-Object -Maximum).Maximum },
    [pscustomobject]@{ metric = 'window list'; n = $wins.Count; p50 = (Pct $wins 50); p90 = (Pct $wins 90); p95 = (Pct $wins 95); max = ($wins | Measure-Object -Maximum).Maximum },
    [pscustomobject]@{ metric = 'show windows'; n = $show.Count; p50 = (Pct $show 50); p90 = (Pct $show 90); p95 = (Pct $show 95); max = ($show | Measure-Object -Maximum).Maximum }
)
$rows | Format-Table -AutoSize

$p50 = Pct $total 50; $p95 = Pct $total 95; $c50 = Pct $capture 50
$verdict = if ($total.Count -lt $Count * 0.9) { "incomplete ($($total.Count)/$Count samples)" } elseif ($p50 -le 80 -and $p95 -le 150 -and $c50 -le 55) { 'PASS (R-CAP-2)' } else { 'MISS (R-CAP-2)' }
Write-Host "verdict: $verdict"

if ($Save) {
    $screens = [System.Windows.Forms.Screen]::AllScreens
    $desc = ($screens | ForEach-Object { "$($_.Bounds.Width)x$($_.Bounds.Height)" }) -join ' + '
    $date = Get-Date -Format 'yyyy-MM-dd'
    $out = Join-Path $PSScriptRoot "results\$date-m1-overlay.md"
    $md = @("# M1 capture entry - $date", "", "- machine: school PC, monitors $desc", "- samples: $($total.Count) / $Count (hold $HoldMs ms, gap $GapMs ms)", "- verdict: $verdict", "", "| metric | n | p50 | p90 | p95 | max |", "|---|---|---|---|---|---|")
    foreach ($r in $rows) { $md += "| $($r.metric) | $($r.n) | $($r.p50) | $($r.p90) | $($r.p95) | $($r.max) |" }
    [IO.File]::WriteAllLines($out, $md, (New-Object System.Text.UTF8Encoding $false))
    Write-Host "saved $out"
}
