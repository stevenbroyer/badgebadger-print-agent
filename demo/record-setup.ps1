# Records a first setup of the agent on a clean Windows machine: install,
# pairing with a BadgeBadger account (when DEMO_SESSION holds a signed-in
# session for the demo school), choosing the card printer, and a test print.
# The cursor glides to each button and really clicks it.
param(
    [string]$Setup, [string]$OutDir,
    [int]$X = 240, [int]$Y = 66, [int]$W = 1440, [int]$H = 900
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "ui.ps1")
. (Join-Path $PSScriptRoot "recorder.ps1")
$cdp = Join-Path $PSScriptRoot "cdp.mjs"

function Cdp([int]$Port, [string]$Match, [string]$Expr) {
    # In base64, or the double quotes inside it never reach node.
    $b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Expr))
    $out = & node $cdp $Port $Match $b64
    if (-not $out -or $out -eq "null") { return $null }
    return ($out | ConvertFrom-Json)
}

# Centre of the first enabled button or link whose visible text is $Text,
# in page pixels, plus the window metrics to turn it into screen pixels.
function Find-Text([int]$Port, [string]$Match, [string]$Text) {
    $t = ConvertTo-Json $Text
    $expr = "(() => { const el = [...document.querySelectorAll('button, a, summary')].find(e => (e.innerText || '').trim() === $t && !e.disabled); if (!el) return null; el.scrollIntoView({ block: 'center' }); const r = el.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2, sx: window.screenX, sy: window.screenY, ch: window.outerHeight - window.innerHeight, cw: window.outerWidth - window.innerWidth }; })()"
    return Cdp $Port $Match $expr
}

function Has-Text([int]$Port, [string]$Match, [string]$Text) {
    $t = ConvertTo-Json $Text
    return [bool](Cdp $Port $Match "document.body ? document.body.innerText.includes($t) : false")
}

function Wait-Text([int]$Port, [string]$Match, [string]$Text, [int]$Seconds = 30) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        if (Has-Text $Port $Match $Text) { return $true }
        Start-Sleep -Milliseconds 400
    }
    return $false
}

# The agent window through Windows accessibility: its buttons and headings
# come with their screen rectangles, so the cursor can go straight to them.
function Agent-Rect([IntPtr]$Agent, [string]$Name, [int]$Seconds = 20) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        $el = Find-Element $Agent $Name
        if ($el) {
            $r = $el.Current.BoundingRectangle
            if (-not $r.IsEmpty -and $r.Width -gt 0) { return $r }
        }
        Start-Sleep -Milliseconds 300
    }
    return $null
}

function Agent-Has([IntPtr]$Agent, [string]$Name, [int]$Seconds = 20) {
    return [bool](Agent-Rect $Agent $Name $Seconds)
}

function Click-Agent([IntPtr]$Agent, [string]$Name) {
    $r = Agent-Rect $Agent $Name
    if (-not $r) { throw "no '$Name' in the agent window" }
    Click-At ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2))
}

# Edge: the page sits below the tab strip and the address bar.
function Click-Edge([string]$Match, [string]$Text) {
    $deadline = (Get-Date).AddSeconds(20)
    $r = $null
    while (-not $r -and (Get-Date) -lt $deadline) {
        $r = Find-Text 9333 $Match $Text
        if (-not $r) { Start-Sleep -Milliseconds 400 }
    }
    if (-not $r) { throw "no '$Text' in Edge" }
    $border = [int]($r.cw / 2)
    Click-At ([int]($r.sx + $border + $r.x)) ([int]($r.sy + $r.ch - $border + $r.y))
}

# A button in Edge's own interface, such as a permission prompt, which is
# a window of its own: its screen rectangle, or $null.
function Edge-Button([string]$Name) {
    $ids = @(Get-Process msedge -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
    $A = [System.Windows.Automation.AutomationElement]
    $byName = New-Object System.Windows.Automation.PropertyCondition ($A::NameProperty), $Name
    $isButton = New-Object System.Windows.Automation.PropertyCondition ($A::ControlTypeProperty), ([System.Windows.Automation.ControlType]::Button)
    $cond = New-Object System.Windows.Automation.AndCondition -ArgumentList (,[System.Windows.Automation.Condition[]]@($byName, $isButton))
    try {
        foreach ($w in $A::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)) {
            if ($ids -notcontains $w.Current.ProcessId) { continue }
            $el = $w.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
            if ($el) {
                $r = $el.Current.BoundingRectangle
                if (-not $r.IsEmpty -and $r.Width -gt 0) { return $r }
            }
        }
    } catch {}
    return $null
}

$SessionJson = $env:DEMO_SESSION
if ($SessionJson) {
    # Edge is signed in to the demo school before the recording starts, so
    # no sign-in screen shows: the session's cookies go in on the site's own
    # origin, then the dashboard loads. Edge allows remote debugging only on a
    # profile of its own; the workflow sends the links the agent opens there.
    Start-Process -FilePath "msedge.exe" -ArgumentList @(
        "--remote-debugging-port=9333", "--user-data-dir=C:\demo\edge", "--no-first-run", "--no-default-browser-check",
        "--window-position=832,$($Y + 96)", "--window-size=736,680", "https://hq.badgebadger.app/login")
    $u = $null
    $deadline = (Get-Date).AddSeconds(60)
    while (-not $u -and (Get-Date) -lt $deadline) { $u = Cdp 9333 "badgebadger.app" "location.href"; if (-not $u) { Start-Sleep -Milliseconds 700 } }
    if (-not $u) { throw "Edge never reached BadgeBadger" }
    foreach ($c in ($SessionJson | ConvertFrom-Json).cookies) {
        $cookie = ConvertTo-Json "$($c.name)=$($c.value); path=/; secure; samesite=lax; max-age=7200"
        [void](Cdp 9333 "badgebadger.app" "document.cookie = $cookie; true")
    }
    [void](Cdp 9333 "badgebadger.app" "location.href = 'https://hq.badgebadger.app/dashboard'; true")
    $deadline = (Get-Date).AddSeconds(40)
    do { Start-Sleep -Milliseconds 800; $u = Cdp 9333 "badgebadger.app" "location.href" } while ((-not $u -or $u -match "/login") -and (Get-Date) -lt $deadline)
    if (-not $u -or $u -match "/login") { throw "the demo session didn't sign in" }
    Write-Host "Edge signed in to the demo school"
}

$backdrop = Start-Backdrop $X $Y $W $H
$rec = Start-Recording (Join-Path $OutDir "setup-raw.mp4") $X $Y $W $H
$agent = [IntPtr]::Zero
$edge = [IntPtr]::Zero
try {
    [void][Mouse]::SetCursorPos($X + $W - 180, $Y + $H - 140)
    Start-Sleep -Milliseconds 1200
    Caption "Run the installer you downloaded" "Install"
    Start-Process -FilePath $Setup
    $setupWin = Wait-Window -Contains "Setup" -TimeoutSec 40
    if ($setupWin -eq [IntPtr]::Zero) { throw "the installer window never appeared" }
    [Win]::Topmost($setupWin, $true)

    $seen = @{}
    for ($step = 0; $step -lt 16; $step++) {
        Start-Sleep -Milliseconds 1600
        $setupWin = [Win]::Find("Setup", $null, $null)
        if ($setupWin -eq [IntPtr]::Zero) { break }
        [Win]::Topmost($setupWin, $true)
        if (-not [Win]::ButtonEnabled($setupWin, 1)) {
            if (-not $seen["installing"]) { $seen["installing"] = $true; Caption "It installs in a few seconds" "Install" }
            continue
        }
        $label = [Win]::ButtonText($setupWin, 1).Replace("&", "")
        if ((Find-Element $setupWin "Choose Install Location") -and -not $seen["location"]) {
            $seen["location"] = $true
            Caption "It installs just for you, so there's no admin password" "Install"
            Start-Sleep -Milliseconds 1800
        }
        if ($label -eq "Finish") { Caption "Click Finish, and the agent opens" "Install"; Start-Sleep -Milliseconds 1400 }
        $c = Button-Center $setupWin 1
        Click-At $c[0] $c[1]
        if ($label -eq "Finish") {
            $deadline = (Get-Date).AddSeconds(8)
            while ([Win]::Find("Setup", $null, $null) -ne [IntPtr]::Zero -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 250 }
            break
        }
    }

    $deadline = (Get-Date).AddSeconds(40)
    while (($agent = [Win]::FindAgent()) -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 250 }
    if ($agent -eq [IntPtr]::Zero) { throw "the agent window never appeared" }
    # Up a little, so the caption labels below never cover it. When pairing
    # it sits left of Edge, and a little lower so the chapter tag in the
    # top-left corner clears its title bar.
    $wr0 = New-Object Mouse+RECT
    [void][Mouse]::GetWindowRect($agent, [ref]$wr0)
    $aw, $ah = ($wr0.R - $wr0.L), ($wr0.B - $wr0.T)
    $ax = if ($SessionJson) { 352 } else { $X + [int](($W - $aw) / 2) }
    $ay = if ($SessionJson) { $Y + 96 } else { $Y + 60 }
    [void][Mouse]::MoveWindow($agent, $ax, $ay, $aw, $ah, $true)
    [Win]::Topmost($agent, $true)
    Write-Host ("agent shows Not connected yet: " + (Agent-Has $agent "Not connected yet" 30))
    try {
        $health = Invoke-RestMethod -Uri "http://127.0.0.1:9988/health" -TimeoutSec 5
        Write-Host ("agent reports " + $health.hostname + ", id " + $health.agentId)
    } catch { Write-Host "no answer from the agent's /health" }
    Start-Sleep -Milliseconds 800

    if ($SessionJson) {
        Caption "Pair it with your BadgeBadger account" "Pair"
        Start-Sleep -Milliseconds 2200
        Click-Agent $agent "Pair this computer"
        $deadline = (Get-Date).AddSeconds(30)
        while (-not (Has-Text 9333 "/pair-agent" "Confirm pair") -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 400 }
        $edge = Wait-Window -Contains "Edge" -TimeoutSec 10
        if ($edge -ne [IntPtr]::Zero) { [void][Mouse]::MoveWindow($edge, 832, $ay, 736, $ah, $true); [Win]::Topmost($edge, $true) }
        Caption "BadgeBadger opens in your browser. Confirm this computer" "Pair"
        Start-Sleep -Milliseconds 2600
        Click-Edge "/pair-agent" "Confirm pair"
        # The page says "Computer paired", then moves on to the computers
        # list a second later; either one means it worked.
        $deadline = (Get-Date).AddSeconds(20)
        while (-not ((Has-Text 9333 "/pair-agent" "Computer paired") -or (Cdp 9333 "/settings/printers/agents" "true")) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 300 }
        # On the computers list the page looks for the agent, and Edge first
        # asks whether the site may reach it. Every real setup meets this.
        $allow = $null
        $deadline = (Get-Date).AddSeconds(15)
        while (-not $allow -and (Get-Date) -lt $deadline) { $allow = Edge-Button "Allow"; if (-not $allow) { Start-Sleep -Milliseconds 300 } }
        if ($allow) {
            Caption "Your browser asks to connect to the agent. Click Allow" "Pair"
            Start-Sleep -Milliseconds 2400
            Click-At ([int]($allow.X + $allow.Width / 2)) ([int]($allow.Y + $allow.Height / 2))
            # The page would look again by itself in a few seconds; Try
            # again looks now.
            Start-Sleep -Milliseconds 1200
            if (Has-Text 9333 "/settings/printers/agents" "No print agent on this computer") { Click-Edge "/settings/printers/agents" "Try again" }
            $deadline = (Get-Date).AddSeconds(15)
            while ((Has-Text 9333 "/settings/printers/agents" "No print agent on this computer") -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 300 }
            Write-Host ("page sees the agent: " + (-not (Has-Text 9333 "/settings/printers/agents" "No print agent on this computer")))
        } else {
            Write-Host "no Allow prompt in Edge"
        }
        Start-Sleep -Milliseconds 1600
        # By now the page has looked up this computer's pairing.
        Write-Host ("page asks to pair it again: " + (Has-Text 9333 "/settings/printers/agents" "Pair this workstation"))
        # Coming back to the agent re-checks the connection straight away.
        $o = [Mouse]::ClientOrigin($agent)
        Click-At ($o.X + 228) ($o.Y + 30) 500
        [void](Agent-Has $agent "Ready to print" 30)
        Caption "Back in the agent: ready to print" "Pair"
        Start-Sleep -Milliseconds 2600
        if ($edge -ne [IntPtr]::Zero) { [Win]::Topmost($edge, $false) }
    }

    Caption "It finds your card printer. Click Use it for badges" "Badge printer"
    Start-Sleep -Milliseconds 2400
    Click-Agent $agent "Use it for badges"
    Start-Sleep -Milliseconds 2200
    Caption "Then print a test card to check" "Badge printer"
    Start-Sleep -Milliseconds 1600
    Click-Agent $agent "Test print"
    # "Done" only once the agent says the card went out; a PDF printer can
    # take a few seconds.
    if (-not (Agent-Has $agent "Test card sent" 30)) { Write-Host "no 'Test card sent' notice" }
    Start-Sleep -Milliseconds 400
    Caption "Done. Badges you print in BadgeBadger come out here" "Badge printer"
    Start-Sleep -Milliseconds 3800
    Caption "Close the window and it keeps running in the tray" "Badge printer"
    Start-Sleep -Milliseconds 1200
    $wr = New-Object Mouse+RECT
    [void][Mouse]::GetWindowRect($agent, [ref]$wr)
    Click-At ($wr.R - 24) ($wr.T + 16) 600
    Start-Sleep -Milliseconds 2200
} finally {
    Save-Captions (Join-Path $OutDir "setup-captions.json")
    Stop-Recording $rec
    if ($agent -ne [IntPtr]::Zero) { [Win]::Topmost($agent, $false) }
    Stop-Process -Id $backdrop.Id -Force -ErrorAction SilentlyContinue
}
