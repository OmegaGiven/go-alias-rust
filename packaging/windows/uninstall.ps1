# Runs as part of the MSI uninstall (elevated). Removes the Scheduled Task.
# Data directory in %ProgramData%\go-alias-rust is left in place so
# reinstalling keeps existing shortcuts.

$ErrorActionPreference = "SilentlyContinue"

$taskName = "go-alias-rust"
$existing = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existing) {
    Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
}
