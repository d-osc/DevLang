param([Parameter(Mandatory=$true)][string]$Installer, [string]$Preview)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$assembly = [Reflection.Assembly]::LoadFrom([IO.Path]::GetFullPath($Installer))
$type = $assembly.GetType('SetupWindow', $true)
$prefix = Join-Path ([IO.Path]::GetTempPath()) ('devlang-gui-test-' + [guid]::NewGuid().ToString('N'))
$flags = [Reflection.BindingFlags]::Instance -bor [Reflection.BindingFlags]::NonPublic
$constructor = $type.GetConstructor($flags, $null, [type[]]@([string], [bool], [bool]), $null)
$window = $constructor.Invoke([object[]]@([string]$prefix, [bool]$true, [bool]$true))
try {
    # Exercise the real button/event path without showing a test window to the user.
    $window.ShowInTaskbar = $false
    $window.Opacity = 0
    $window.Show()
    [Windows.Forms.Application]::DoEvents()
    if ($Preview) {
        $bitmap = New-Object Drawing.Bitmap($window.Width, $window.Height)
        try {
            $window.DrawToBitmap($bitmap, (New-Object Drawing.Rectangle(0, 0, $window.Width, $window.Height)))
            $bitmap.Save([IO.Path]::GetFullPath($Preview), [Drawing.Imaging.ImageFormat]::Png)
        } finally { $bitmap.Dispose() }
    }
    $button = @($window.Controls | Where-Object { $_ -is [Windows.Forms.Button] -and $_.Text -eq 'Install' })[0]
    if (-not $button) { throw 'Install button missing' }
    $button.PerformClick()
    $deadline = [DateTime]::UtcNow.AddSeconds(60)
    while ($button.Text -ne 'Close') {
        [Windows.Forms.Application]::DoEvents()
        if ($button.Text -eq 'Close') { break }
        if ($button.Enabled) {
            $messages = @($window.Controls | Where-Object { $_ -is [Windows.Forms.TextBox] -and $_.Multiline })[0].Text
            throw "GUI installation failed: $messages"
        }
        if ([DateTime]::UtcNow -gt $deadline) { throw 'GUI installation timed out' }
        Start-Sleep -Milliseconds 50
    }
    if (-not (Test-Path -LiteralPath (Join-Path $prefix 'd.exe'))) { throw 'GUI did not install d.exe' }
    $button.PerformClick()
    Write-Host 'Passed: GUI Install button, background installation, success state and Close button'
} finally {
    $window.Dispose()
    if (Test-Path -LiteralPath (Join-Path $prefix '.devlang-install.json')) {
        & (Join-Path $prefix 'uninstall.ps1') -Prefix $prefix -Uninstall
    }
}
