# Run Cargo with the Visual C++ environment instead of Git's unrelated link.exe.
# Usage: .\cargo-windows.ps1 check --workspace
$ErrorActionPreference = 'Stop'
$cargoArguments = @($args)
if ($cargoArguments.Count -eq 0) {
    $cargoArguments = @('check', '--workspace')
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'Install Visual Studio Build Tools with the Desktop development with C++ workload.'
}
$vsPath = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsPath) {
    throw 'Visual Studio C++ tools are missing. Modify Build Tools and select Desktop development with C++, including a Windows SDK.'
}
$devShell = Join-Path $vsPath 'Common7/Tools/Launch-VsDevShell.ps1'
if (-not (Test-Path -LiteralPath $devShell)) {
    throw "Visual Studio developer shell is missing: $devShell"
}

$originalEnvironment = @{}
Get-ChildItem Env: | ForEach-Object { $originalEnvironment[$_.Name] = $_.Value }
$cargoExitCode = 1
Push-Location $PSScriptRoot
try {
    & $devShell -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
    $linker = (Get-Command link.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    if (-not $linker.StartsWith($vsPath, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Expected the MSVC linker under $vsPath, but found $linker"
    }
    if (-not $env:WindowsSdkDir) {
        throw 'A Windows SDK is required. Add it through the Visual Studio Installer.'
    }
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $linker
    & cargo +stable-x86_64-pc-windows-msvc @cargoArguments
    $cargoExitCode = $LASTEXITCODE
} finally {
    Pop-Location
    # Keep the developer environment local to this script invocation.
    Get-ChildItem Env: | Where-Object { -not $originalEnvironment.ContainsKey($_.Name) } | ForEach-Object {
        Remove-Item -LiteralPath "Env:$($_.Name)"
    }
    foreach ($environmentName in $originalEnvironment.Keys) {
        Set-Item -LiteralPath "Env:$environmentName" -Value $originalEnvironment[$environmentName]
    }
}
exit $cargoExitCode
