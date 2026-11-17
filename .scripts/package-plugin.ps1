#Requires -Version 7.0

param(
    [Parameter(Mandatory = $true)]
    [string]$Plugin
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

function Write-DllChecksum {
    param(
        [string]$ChecksumPath,
        [string]$DllPath,
        [string]$Hash
    )

    # Use relative paths and LF line endings for sha256sum-compatible checksums.
    $checksumLine = '{0}  {1}' -f $Hash, $DllPath
    Add-Content -LiteralPath $ChecksumPath -Value "$checksumLine`n" -Encoding utf8NoBOM -NoNewline

    # Covector includes this Markdown output in the release notes.
    Write-Output ('- SHA256 (`{0}`): `{1}`' -f $DllPath, $Hash)
}
function New-PluginStaging {
    param(
        [string]$PluginName,
        [string]$StagingDirectory
    )

    $dllFileName = "$PluginName.dll"
    $checksumPath = Join-Path $StagingDirectory "$PluginName.sha256"
    $variantBuildDirectories = [ordered]@{
        'amd64-unicode' = 'x86_64-pc-windows-msvc/release'
        'x86-unicode' = 'i686-pc-windows-msvc/release'
        'x86-ansi' = 'x86-ansi/i686-pc-windows-msvc/release'
    }

    # Stage each DLL under the target directory expected by NSIS.
    foreach ($variant in $variantBuildDirectories.GetEnumerator()) {
        $variantName = $variant.Key
        $sourceDll = Join-Path $repoRoot 'target' $variant.Value $dllFileName
        $destinationDirectory = Join-Path $StagingDirectory $variantName
        $destinationDll = Join-Path $destinationDirectory $dllFileName
        $archiveDllPath = "$variantName/$dllFileName"

        New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null
        Copy-Item -LiteralPath $sourceDll -Destination $destinationDll

        $hash = (Get-FileHash -LiteralPath $destinationDll -Algorithm SHA256).Hash
        Write-DllChecksum -ChecksumPath $checksumPath -DllPath $archiveDllPath -Hash $hash
    }

    # Stage license files alongside the plugin DLLs.
    foreach ($licenseName in 'LICENSE_MIT', 'LICENSE_APACHE-2.0') {
        $sourceLicense = Join-Path $RepoRoot $licenseName
        $destinationLicense = Join-Path $StagingDirectory $licenseName

        Copy-Item -LiteralPath $sourceLicense -Destination $destinationLicense
    }
}


# Verify plugin exists in the configuration
$configPath = Join-Path $repoRoot '.changes' 'config.json'
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$pluginNames = $config.packages.PSObject.Properties.Name
if ($Plugin -cnotin $pluginNames) {
    throw "Unknown plugin: $Plugin"
}

# Prepare staging and archive paths for the plugin
$packageDirectory = Join-Path $repoRoot 'target' 'packages'
$stagingName = "$Plugin-$([guid]::NewGuid().ToString('N'))"
$stagingDirectory = Join-Path $packageDirectory $stagingName
$archivePath = Join-Path $packageDirectory "$Plugin.zip"

try {
    New-PluginStaging -PluginName $Plugin -StagingDirectory $stagingDirectory

    Compress-Archive -LiteralPath (Get-ChildItem -LiteralPath $stagingDirectory).FullName `
        -DestinationPath $archivePath -CompressionLevel Optimal -Force

    # Covector includes this Markdown output in the release notes.
    $archiveHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
    Write-Output ('- SHA256 (ZIP): `{0}`' -f $archiveHash)
} finally {
    # Clean up the staging directory after packaging the plugin.
    if (Test-Path -LiteralPath $stagingDirectory) {
        Remove-Item -LiteralPath $stagingDirectory -Recurse -Force
    }
}
