# Runs as part of the MSI install (elevated). Sets up the data directory
# with default shortcut/theme files and registers a Scheduled Task that
# starts go_service.exe at system boot with SYSTEM privileges (needed to
# bind port 80).

$ErrorActionPreference = "Stop"

$InstallDir = $PSScriptRoot
$DataDir = Join-Path $env:ProgramData "go-alias-rust"

New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

$defaults = @(
    "shortcuts.json",
    "hidden-shortcuts.json",
    "work-shortcuts.json",
    "themes.json",
    "current_theme.json"
)

foreach ($f in $defaults) {
    $dest = Join-Path $DataDir $f
    if (-not (Test-Path $dest)) {
        Copy-Item (Join-Path $InstallDir "defaults\$f") $dest
    }
}

$staticDest = Join-Path $DataDir "static"
if (-not (Test-Path $staticDest)) {
    Copy-Item -Recurse (Join-Path $InstallDir "static") $staticDest
}

$taskName = "go-alias-rust"
$exePath = Join-Path $InstallDir "go_service.exe"

$existing = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existing) {
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
}

$action = New-ScheduledTaskAction -Execute $exePath -WorkingDirectory $DataDir
$trigger = New-ScheduledTaskTrigger -AtStartup
$principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet -Restart -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries

Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings | Out-Null
Start-ScheduledTask -TaskName $taskName
