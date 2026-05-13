[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Args
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = (Resolve-Path (Join-Path $scriptDir "..")).Path

Push-Location $rootDir
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "[run] cargo is not installed or not in PATH."
        exit 1
    }

    Write-Host "[run] Launching CodeWarp with cargo run..."
    & cargo run -- @Args
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
