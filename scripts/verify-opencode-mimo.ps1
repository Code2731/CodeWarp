[CmdletBinding()]
param(
    [string]$ProModel = "mimo/mimo-v2.5-pro",
    [string]$BaseModel = "mimo/mimo-v2.5",
    [string]$Gpt55Model = "openai/gpt-5.5",
    [string]$SparkModel = "openai/gpt-5.3-codex-spark"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Action
    )

    Write-Host "==> $Name"
    & $Action
    Write-Host "OK: $Name`n"
}

function Invoke-External {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter()][string[]]$Args = @()
    )

    & $Command @Args
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE"
    }
}

function Test-OpenCodeModel {
    param(
        [Parameter(Mandatory = $true)][string]$Model,
        [Parameter(Mandatory = $true)][string]$Expected,
        [Parameter(Mandatory = $true)][string]$Title
    )

    $output = & opencode run --pure --model $Model --format json --title $Title "Reply with exactly: $Expected" 2>&1
    if ($LASTEXITCODE -ne 0) {
        $joined = ($output | Out-String).Trim()
        throw "opencode run failed for $Model with exit code $LASTEXITCODE. Output: $joined"
    }

    $text = ($output | Out-String)
    if ($text -notmatch [regex]::Escape($Expected)) {
        throw "Expected '$Expected' in opencode output for $Model."
    }
}

$blockedEnv = @(
    "OPENCODE_CLIENT",
    "OPENCODE_DISABLE_EMBEDDED_WEB_UI",
    "OPENCODE_SERVER_USERNAME",
    "OPENCODE_SERVER_PASSWORD"
)

foreach ($name in $blockedEnv) {
    Remove-Item -LiteralPath "Env:\$name" -ErrorAction SilentlyContinue
}

if (-not $env:MIMO_API_KEY) {
    throw "MIMO_API_KEY is not set. Set it as a user environment variable before running this helper."
}

Invoke-Step -Name "opencode models mimo" -Action {
    Invoke-External -Command "opencode" -Args @("models", "mimo")
}

Invoke-Step -Name "opencode run $ProModel" -Action {
    Test-OpenCodeModel -Model $ProModel -Expected "mimo-pro-ok" -Title "mimo-pro-smoke"
}

Invoke-Step -Name "opencode run $BaseModel" -Action {
    Test-OpenCodeModel -Model $BaseModel -Expected "mimo-base-ok" -Title "mimo-base-smoke"
}

Invoke-Step -Name "opencode run $Gpt55Model" -Action {
    Test-OpenCodeModel -Model $Gpt55Model -Expected "gpt55-ok" -Title "gpt55-smoke"
}

Invoke-Step -Name "opencode run $SparkModel" -Action {
    Test-OpenCodeModel -Model $SparkModel -Expected "spark-ok" -Title "spark-smoke"
}

Invoke-Step -Name "oh-my-openagent doctor" -Action {
    Invoke-External -Command "bunx" -Args @("oh-my-openagent", "doctor")
}

Write-Host "OpenCode model stack verification completed."
