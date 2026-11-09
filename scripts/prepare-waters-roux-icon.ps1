<# Derive Windows sizes from the exact original PNG; never modify the PNG. #>
param([string]$Source = (Split-Path $PSScriptRoot -Parent))
$ErrorActionPreference = 'Stop'
$logo = Join-Path $Source 'assets/branding/waters-roux-logo.png'
if (-not (Test-Path -LiteralPath $logo)) {
    Write-Warning 'Original Waters Roux PNG is missing; using the Windows system application icon.'
    return
}
Add-Type -AssemblyName System.Drawing
$image = [Drawing.Image]::FromFile($logo)
$frames = [Collections.Generic.List[byte[]]]::new()
try {
    foreach ($size in @(32, 48, 64, 128, 256)) {
        $bitmap = [Drawing.Bitmap]::new($size, $size)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $stream = [IO.MemoryStream]::new()
        try {
            $graphics.Clear([Drawing.Color]::Transparent)
            $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $scale = [Math]::Min($size / $image.Width, $size / $image.Height)
            $width = [int][Math]::Round($image.Width * $scale)
            $height = [int][Math]::Round($image.Height * $scale)
            $graphics.DrawImage($image, [int](($size - $width) / 2), [int](($size - $height) / 2), $width, $height)
            $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
            $frames.Add($stream.ToArray())
        } finally {
            $stream.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
        }
    }
} finally { $image.Dispose() }
$path = Join-Path $Source 'assets/branding/waters-roux.ico'
$file = [IO.File]::Create($path)
$writer = [IO.BinaryWriter]::new($file)
try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$frames.Count)
    $offset = 6 + 16 * $frames.Count
    $sizes = @(32, 48, 64, 128, 256)
    for ($i = 0; $i -lt $frames.Count; $i++) {
        $sizeByte = [byte]($sizes[$i] % 256)
        $writer.Write($sizeByte); $writer.Write($sizeByte)
        $writer.Write([byte]0); $writer.Write([byte]0)
        $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$frames[$i].Length); $writer.Write([uint32]$offset)
        $offset += $frames[$i].Length
    }
    foreach ($frame in $frames) { $writer.Write($frame) }
} finally { $writer.Dispose(); $file.Dispose() }
