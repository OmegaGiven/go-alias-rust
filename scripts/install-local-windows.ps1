$ErrorActionPreference = "Stop"

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Admin)) {
    Write-Error "Run this installer from an elevated PowerShell prompt so it can update hosts and install a service."
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ParentDir = Split-Path -Parent $ScriptDir
if (Test-Path (Join-Path $ScriptDir "Cargo.toml")) {
    $RepoDir = $ScriptDir
} elseif (Test-Path (Join-Path $ParentDir "Cargo.toml")) {
    $RepoDir = $ParentDir
} else {
    $RepoDir = $null
}

$InstallDir = Join-Path $env:ProgramFiles "GoAlias"
$BinaryDest = Join-Path $InstallDir "go_service.exe"
$StaticDest = Join-Path $InstallDir "static"
$TaskName = "GoAlias"

if ($RepoDir) {
    $BinarySource = Join-Path $RepoDir "target\release\go_service.exe"
    $StaticSource = Join-Path $RepoDir "static"
    Write-Host "Building release binary..."
    Push-Location $RepoDir
    cargo build --release
    Pop-Location
} else {
    $BinarySource = Join-Path $ScriptDir "go_service.exe"
    $StaticSource = Join-Path $ScriptDir "static"
    Write-Host "Using bundled release binary..."
}

Write-Host "Installing files into $InstallDir..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item $BinarySource $BinaryDest -Force
if (Test-Path $StaticDest) {
    Remove-Item $StaticDest -Recurse -Force
}
Copy-Item $StaticSource $StaticDest -Recurse -Force

Write-Host "Ensuring local hostname 'go' resolves to this machine..."
$HostsPath = "$env:SystemRoot\System32\drivers\etc\hosts"
$HostsContent = Get-Content $HostsPath -Raw
if ($HostsContent -notmatch "(^|\s)go(\s|$)") {
    Add-Content -Path $HostsPath -Value "`r`n# GoAlias local browser alias`r`n127.0.0.1 go`r`n::1 go"
}

Write-Host "Installing Windows startup task..."
$ExistingTask = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if ($ExistingTask) {
    Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
}

$Action = New-ScheduledTaskAction -Execute $BinaryDest -WorkingDirectory $InstallDir
$Trigger = New-ScheduledTaskTrigger -AtStartup
$Principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -RunLevel Highest
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)

Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings -Description "Go Alias developer tool" | Out-Null
Start-ScheduledTask -TaskName $TaskName

Write-Host ""
Write-Host "Installed. Open http://go/ or http://go/<alias> in a browser."
Write-Host "If port 80 is not available, stop the conflicting service and restart the GoAlias scheduled task."
