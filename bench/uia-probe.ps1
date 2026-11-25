# 이 PC에 떠 있는 일부 앱 창에서, 점마다 UIA·MSAA가 어떤 요소를 돌려주는지 잰다(읽기 전용).
# 창 제목·요소 이름·값은 읽지 않는다. 프로세스 이름·클래스 이름·컨트롤 종류·사각형 크기·시간만 남긴다.
param([string[]]$Apps = @('chrome', 'explorer', 'ApplicationFrameHost', 'claude', 'tidy_task'), [int]$Cols = 6, [int]$Rows = 5)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, Accessibility, WindowsBase
Add-Type -Namespace P -Name W -ReferencedAssemblies Accessibility -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
[StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; public POINT(int x, int y) { X = x; Y = y; } }
public delegate bool EnumProc(System.IntPtr h, System.IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, System.IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(System.IntPtr p, EnumProc f, System.IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr h);
[DllImport("user32.dll")] public static extern bool IsIconic(System.IntPtr h);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(System.IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern System.IntPtr WindowFromPoint(POINT p);
[DllImport("user32.dll")] public static extern System.IntPtr GetAncestor(System.IntPtr h, uint f);
[DllImport("user32.dll")] public static extern int GetWindowLong(System.IntPtr h, int i);
[DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(System.IntPtr h, int a, out RECT r, int s);
[DllImport("dwmapi.dll", EntryPoint="DwmGetWindowAttribute")] public static extern int DwmGetInt(System.IntPtr h, int a, out int v, int s);
[DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
[DllImport("oleacc.dll")] public static extern int AccessibleObjectFromPoint(POINT p, [MarshalAs(UnmanagedType.Interface)] out Accessibility.IAccessible acc, [MarshalAs(UnmanagedType.Struct)] out object child);
'@
function ClassOf($h) { $sb = New-Object System.Text.StringBuilder 256; [void][P.W]::GetClassNameW($h, $sb, 256); $sb.ToString() }
$procById = @{}; Get-Process | ForEach-Object { $procById[[uint32]$_.Id] = $_.ProcessName }
$targets = New-Object System.Collections.ArrayList
$cb = [P.W+EnumProc] {
    param($h, $l)
    if (-not [P.W]::IsWindowVisible($h) -or [P.W]::IsIconic($h)) { return $true }
    $cloak = 0; [void][P.W]::DwmGetInt($h, 14, [ref]$cloak, 4); if ($cloak -ne 0) { return $true }
    $ex = [P.W]::GetWindowLong($h, -20); if ($ex -band 0x80) { return $true } # 도구 창 제외
    $procId = 0; [void][P.W]::GetWindowThreadProcessId($h, [ref]$procId)
    $name = $procById[$procId]
    if ($Apps -contains $name) {
        $r = New-Object P.W+RECT; [void][P.W]::DwmGetWindowAttribute($h, 9, [ref]$r, 16)
        $wr = New-Object P.W+RECT; [void][P.W]::GetWindowRect($h, [ref]$wr)
        if (($r.R - $r.L) -gt 200 -and ($r.B - $r.T) -gt 150) { [void]$targets.Add([pscustomobject]@{ H = $h; App = $name; Cls = (ClassOf $h); R = $r; WR = $wr }) }
    }
    return $true
}
[void][P.W]::EnumWindows($cb, [IntPtr]::Zero)
$seen = @{}
foreach ($t in $targets) {
    if ($seen[$t.App]) { continue }; $seen[$t.App] = $true # 앱마다 맨 위 창 하나
    $kids = 0; $ccb = [P.W+EnumProc] { param($h, $l) if ([P.W]::IsWindowVisible($h)) { $script:kids++ }; return $true }
    $script:kids = 0; [void][P.W]::EnumChildWindows($t.H, $ccb, [IntPtr]::Zero); $kids = $script:kids
    $W = $t.R.R - $t.R.L; $H = $t.R.B - $t.R.T
    $shadow = ($t.WR.R - $t.WR.L) - $W
    $uiaMs = @(); $deep = 0; $n = 0; $first = $null; $types = @{}; $fw = @{}; $msaaDeep = 0; $msaaMs = @(); $offEdge = 0
    for ($i = 0; $i -lt $Cols; $i++) { for ($j = 0; $j -lt $Rows; $j++) {
        $x = [int]($t.R.L + $W * ($i + 0.5) / $Cols); $y = [int]($t.R.T + $H * ($j + 0.5) / $Rows)
        $root = [P.W]::GetAncestor([P.W]::WindowFromPoint((New-Object P.W+POINT($x, $y))), 2)
        if ($root -ne $t.H) { continue } # 다른 창에 가려진 점은 건너뜀
        $n++
        $sw = [Diagnostics.Stopwatch]::StartNew()
        try { $el = [Windows.Automation.AutomationElement]::FromPoint((New-Object Windows.Point($x, $y))) } catch { $el = $null }
        $ms = $sw.Elapsed.TotalMilliseconds; if ($null -eq $first) { $first = $ms }; $uiaMs += $ms
        if ($el) {
            $c = $el.Current
            $br = $c.BoundingRectangle
            $ct = $c.ControlType.ProgrammaticName -replace 'ControlType\.', ''
            $types[$ct] = 1 + [int]$types[$ct]; $fw[$c.FrameworkId] = 1 + [int]$fw[$c.FrameworkId]
            if (-not $br.IsEmpty -and ($br.Width * $br.Height) -lt (0.5 * $W * $H)) { $deep++ }
            if (-not $br.IsEmpty -and ($x -lt $br.Left -or $x -gt $br.Right -or $y -lt $br.Top -or $y -gt $br.Bottom)) { $offEdge++ }
        }
        $sw.Restart()
        try {
            $acc = $null; $child = $null
            if ([P.W]::AccessibleObjectFromPoint((New-Object P.W+POINT($x, $y)), [ref]$acc, [ref]$child) -eq 0 -and $acc) {
                $l = 0; $tp = 0; $w2 = 0; $h2 = 0; $acc.accLocation([ref]$l, [ref]$tp, [ref]$w2, [ref]$h2, $child)
                if (($w2 * $h2) -gt 0 -and ($w2 * $h2) -lt (0.5 * $W * $H)) { $msaaDeep++ }
            }
        } catch { }
        $msaaMs += $sw.Elapsed.TotalMilliseconds
    } }
    if ($n -eq 0) { "{0,-21} {1,-28} no visible points" -f $t.App, $t.Cls; continue }
    $s = $uiaMs | Sort-Object; $m = $msaaMs | Sort-Object
    $p = { param($a, $q) $a[[math]::Min($a.Count - 1, [int][math]::Floor($a.Count * $q))] }
    [pscustomobject]@{
        App = $t.App; Class = $t.Cls; Size = "${W}x${H}"; ShadowPx = $shadow; ChildHwnd = $kids; Points = $n
        UiaDeepPct = [math]::Round(100 * $deep / $n); UiaFirstMs = [math]::Round($first, 1); UiaP50 = [math]::Round((& $p $s 0.5), 1); UiaP95 = [math]::Round((& $p $s 0.95), 1)
        UiaOffPoint = $offEdge; MsaaDeepPct = [math]::Round(100 * $msaaDeep / $n); MsaaP50 = [math]::Round((& $p $m 0.5), 1)
        Framework = (($fw.GetEnumerator() | Sort-Object Value -Descending | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ',')
        Types = (($types.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 5 | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ',')
    }
}
