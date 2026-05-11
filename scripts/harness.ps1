[CmdletBinding()]
param(
    [switch]$SkipFmt,
    [switch]$SkipCheck,
    [switch]$SkipTests,
    [switch]$SkipClippy,
    [switch]$StrictClippy,
    [string]$Endpoint = "",
    [string]$Token = "",
    [int]$TimeoutSec = 5
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

function Invoke-Cargo {
    param(
        [Parameter(Mandatory = $true)][string]$Subcommand,
        [Parameter()][string[]]$Args = @()
    )

    & cargo $Subcommand @Args
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $Subcommand failed with exit code $LASTEXITCODE"
    }
}

function Get-ModelsUrl {
    param([Parameter(Mandatory = $true)][string]$BaseUrl)

    $trimmed = $BaseUrl.Trim().TrimEnd("/")
    if ($trimmed.EndsWith("/v1")) {
        return "$trimmed/models"
    }
    return "$trimmed/v1/models"
}

if (-not $SkipFmt) {
    Invoke-Step -Name "cargo fmt -- --check" -Action {
        Invoke-Cargo -Subcommand "fmt" -Args @("--", "--check")
    }
}

if (-not $SkipCheck) {
    Invoke-Step -Name "cargo check" -Action {
        Invoke-Cargo -Subcommand "check"
    }
}

if (-not $SkipTests) {
    Invoke-Step -Name "cargo test --all-targets" -Action {
        Invoke-Cargo -Subcommand "test" -Args @("--all-targets")
    }
}

if (-not $SkipClippy) {
    Write-Host "==> cargo clippy --all-targets"
    & cargo clippy --all-targets
    $clippyExit = $LASTEXITCODE
    if ($clippyExit -ne 0) {
        if ($StrictClippy) {
            throw "cargo clippy failed with exit code $clippyExit"
        }
        Write-Warning "cargo clippy reported issues (exit $clippyExit). Continuing because -StrictClippy was not set."
    } else {
        Write-Host "OK: cargo clippy --all-targets`n"
    }
}

if (-not [string]::IsNullOrWhiteSpace($Endpoint)) {
    $modelsUrl = Get-ModelsUrl -BaseUrl $Endpoint
    Invoke-Step -Name "endpoint health check ($modelsUrl)" -Action {
        $headers = @{}
        if (-not [string]::IsNullOrWhiteSpace($Token)) {
            $headers["Authorization"] = "Bearer $($Token.Trim())"
        }

        try {
            $resp = Invoke-WebRequest -Uri $modelsUrl -Headers $headers -Method Get -TimeoutSec $TimeoutSec -UseBasicParsing
            Write-Host "HTTP $($resp.StatusCode) from $modelsUrl"
        } catch {
            $status = $null
            if ($_.Exception.Response) {
                try {
                    $status = [int]$_.Exception.Response.StatusCode
                } catch {
                    $status = $null
                }
            }

            if ($status) {
                throw "Endpoint health check failed: HTTP $status ($modelsUrl)"
            }

            throw "Endpoint health check failed: $($_.Exception.Message) ($modelsUrl)"
        }
    }
}

Write-Host "Harness completed successfully."
