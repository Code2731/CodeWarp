[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$hooksPath = Join-Path $root ".githooks"

if (-not (Test-Path $hooksPath)) {
    throw "Hooks path not found: $hooksPath"
}

git config core.hooksPath ".githooks"
if ($LASTEXITCODE -ne 0) {
    throw "Failed to set git core.hooksPath"
}

Write-Host "Installed git hooks path: .githooks"
Write-Host "Enabled hooks: pre-commit (fmt check), pre-push (harness)."
