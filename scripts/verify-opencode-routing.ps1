[CmdletBinding()]
param(
    [string]$ConfigPath = "C:\Users\USER\.config\opencode\oh-my-openagent.json",
    [string]$PolicyPath = "docs\OPENCODE_ROUTING.md"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Parse-ModelCell {
    param(
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Cell
    )

    if ([string]::IsNullOrWhiteSpace($Cell)) {
        return [pscustomobject]@{
            Model      = ""
            Variant    = ""
            HasVariant = $false
        }
    }

    $match = [regex]::Match($Cell, '^\s*(?<Model>.+?)\s*\((?<Variant>[^)]+)\)\s*$')
    if ($match.Success) {
        return [pscustomobject]@{
            Model      = $match.Groups['Model'].Value.Trim()
            Variant    = $match.Groups['Variant'].Value.Trim()
            HasVariant = $true
        }
    }

    return [pscustomobject]@{
        Model      = $Cell.Trim()
        Variant    = ""
        HasVariant = $false
    }
}

function Get-PolicyRouting {
    param(
        [Parameter(Mandatory = $true)][string]$PolicyPath
    )

    if (-not (Test-Path -LiteralPath $PolicyPath)) {
        throw "Missing policy file: $PolicyPath"
    }

    $text = Get-Content -LiteralPath $PolicyPath -Raw
    $lines = $text -split "`r?`n"
    $inTable = $false
    $routing = @{}

    foreach ($line in $lines) {
        if (-not $inTable) {
            if ($line -match '^\s*\|\s*Route\s*\|') {
                $inTable = $true
            }
            continue
        }

        if ($line -match '^\s*\|(?:\s*[:-]+\s*\|)+\s*$') {
            continue
        }

        if ($line -match '^\s*\|') {
            $cells = $line -split '\|'
            if ($cells.Count -lt 4) {
                continue
            }

            $route = $cells[1].Trim()
            if ([string]::IsNullOrWhiteSpace($route)) {
                continue
            }

            $primary = Parse-ModelCell -Cell $cells[2]
            $fallback = Parse-ModelCell -Cell $cells[3]

            $routing[$route] = [pscustomobject]@{
                PrimaryModel      = $primary.Model
                PrimaryHasVariant = $primary.HasVariant
                PrimaryVariant    = $primary.Variant
                FallbackModel     = $fallback.Model
                FallbackHasValue  = -not [string]::IsNullOrWhiteSpace($cells[3])
                FallbackVariant   = $fallback.Variant
            }

            if ($route -eq 'Route' -or $route -eq '---') {
                $routing.Remove($route)
            }

            continue
        }

        break
    }

    if ($routing.Count -eq 0) {
        throw "Unable to parse routing matrix from policy: $PolicyPath"
    }

    return $routing
}

function Get-ConfigRoute {
    param(
        [Parameter(Mandatory = $true)]$Config,
        [Parameter(Mandatory = $true)][string]$Route
    )

    if ($Config.agents -and ($Config.agents.PSObject.Properties.Name -contains $Route)) {
        return $Config.agents.$Route
    }

    if ($Config.categories -and ($Config.categories.PSObject.Properties.Name -contains $Route)) {
        return $Config.categories.$Route
    }

    return $null
}

function Get-FallbackModels {
    param(
        [Parameter(Mandatory = $true)]$RouteConfig
    )

    if ($null -eq $RouteConfig -or -not ($RouteConfig.PSObject.Properties.Name -contains 'fallback_models')) {
        return @()
    }

    $fallbacks = $RouteConfig.fallback_models
    if ($null -eq $fallbacks) {
        return @()
    }

    if ($fallbacks -is [System.Array]) {
        return $fallbacks
    }

    if ($fallbacks -is [System.Collections.IEnumerable] -and -not ($fallbacks -is [string])) {
        return @($fallbacks)
    }

    return @($fallbacks)
}

function Get-ConfigRouteNames {
    param(
        [Parameter(Mandatory = $true)]$Config
    )

    $names = New-Object System.Collections.Generic.HashSet[string]
    if ($Config.agents) {
        foreach ($name in $Config.agents.PSObject.Properties.Name) {
            [void]$names.Add($name)
        }
    }

    if ($Config.categories) {
        foreach ($name in $Config.categories.PSObject.Properties.Name) {
            [void]$names.Add($name)
        }
    }

    return @($names)
}

try {
    if (-not (Test-Path -LiteralPath $ConfigPath)) {
        throw "Missing config file: $ConfigPath"
    }

    try {
        $configText = Get-Content -LiteralPath $ConfigPath -Raw
        $config = $configText | ConvertFrom-Json -ErrorAction Stop
    }
    catch {
        throw "Malformed config JSON: $ConfigPath"
    }

    if ($null -eq $config) {
        throw "Malformed config JSON: $ConfigPath"
    }

    $policy = Get-PolicyRouting -PolicyPath $PolicyPath
    $driftMessages = New-Object System.Collections.Generic.List[string]
    $expectedRoutes = New-Object System.Collections.Generic.HashSet[string]

    foreach ($route in @($policy.Keys | Sort-Object)) {
        [void]$expectedRoutes.Add($route)
        $expected = $policy[$route]
        $routeConfig = Get-ConfigRoute -Config $config -Route $route

        if ($null -eq $routeConfig) {
            $driftMessages.Add("Route '$route': missing route in config.")
            continue
        }

        if (-not ($routeConfig.PSObject.Properties.Name -contains 'model')) {
            $driftMessages.Add("Route '$route': expected primary model '$($expected.PrimaryModel)', found '<missing>'.")
        }
        elseif ($routeConfig.model -ne $expected.PrimaryModel) {
            $driftMessages.Add("Route '$route': expected primary model '$($expected.PrimaryModel)', found '$($routeConfig.model)'.")
        }

        if ($expected.PrimaryHasVariant) {
            if (-not ($routeConfig.PSObject.Properties.Name -contains 'variant')) {
                $driftMessages.Add("Route '$route': expected primary variant '$($expected.PrimaryVariant)', found '<missing>'.")
            }
            elseif ($routeConfig.variant -ne $expected.PrimaryVariant) {
                $driftMessages.Add("Route '$route': expected primary variant '$($expected.PrimaryVariant)', found '$($routeConfig.variant)'.")
            }
        }
        elseif (($routeConfig.PSObject.Properties.Name -contains 'variant') -and -not [string]::IsNullOrWhiteSpace($routeConfig.variant)) {
            $driftMessages.Add("Route '$route': unexpected primary variant '$($routeConfig.variant)'.")
        }

        $fallbacks = @(Get-FallbackModels -RouteConfig $routeConfig)

        if ($expected.FallbackHasValue) {
            if ($fallbacks.Count -eq 0) {
                $driftMessages.Add("Route '$route': missing expected fallback model '$($expected.FallbackModel)'.")
            }
            elseif ($fallbacks.Count -ne 1) {
                $driftMessages.Add("Route '$route': expected exactly one fallback model '$($expected.FallbackModel)', found $($fallbacks.Count).")
            }
            else {
                $fallback = $fallbacks[0]

                if (-not ($fallback.PSObject.Properties.Name -contains 'model')) {
                    $driftMessages.Add("Route '$route': expected fallback model '$($expected.FallbackModel)', found '<missing>'.")
                }
                elseif ($fallback.model -ne $expected.FallbackModel) {
                    $driftMessages.Add("Route '$route': expected fallback model '$($expected.FallbackModel)', found '$($fallback.model)'.")
                }

                if (-not [string]::IsNullOrWhiteSpace($expected.FallbackVariant)) {
                    if (-not ($fallback.PSObject.Properties.Name -contains 'variant')) {
                        $driftMessages.Add("Route '$route': expected fallback variant '$($expected.FallbackVariant)', found '<missing>'.")
                    }
                    elseif ($fallback.variant -ne $expected.FallbackVariant) {
                        $driftMessages.Add("Route '$route': expected fallback variant '$($expected.FallbackVariant)', found '$($fallback.variant)'.")
                    }
                }
                elseif (($fallback.PSObject.Properties.Name -contains 'variant') -and -not [string]::IsNullOrWhiteSpace($fallback.variant)) {
                    $driftMessages.Add("Route '$route': unexpected fallback variant '$($fallback.variant)'.")
                }
            }
        }
        else {
            if ($fallbacks.Count -gt 0) {
                $driftMessages.Add("Route '$route': unexpected fallback configured.")
            }
        }
    }

    foreach ($route in @(Get-ConfigRouteNames -Config $config | Sort-Object)) {
        if (-not $expectedRoutes.Contains($route)) {
            $driftMessages.Add("Route '$route': unexpected route in config.")
        }
    }

    if ($driftMessages.Count -gt 0) {
        foreach ($msg in $driftMessages) {
            Write-Host $msg
        }

        exit 1
    }

    Write-Host "OpenCode routing policy matches documentation." -ForegroundColor Green
    exit 0
}
catch {
    Write-Host $_.Exception.Message
    exit 1
}
