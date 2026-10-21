# Build a local, unsigned Windows package of the Anshot (安截) desktop app into dist/.
# Usage: pwsh -File scripts/package.ps1
# Nothing is uploaded or published; signing and an installer are out of scope.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$version = (Select-String -Path "$root\apps\desktop\Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$name = "Anshot-$version-win-x64"
$dist = Join-Path $root "dist\$name"

cargo build --release --locked --manifest-path "$root\apps\desktop\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "$root\apps\desktop\target" }

if (Test-Path $dist) { Get-ChildItem $dist | ForEach-Object { [System.IO.File]::Delete($_.FullName) } } else { New-Item -ItemType Directory -Force $dist | Out-Null }
Copy-Item "$target\release\anshot.exe" "$dist\Anshot.exe"
Copy-Item "$root\LICENSE" "$dist\LICENSE.txt"
Copy-Item "$root\apps\desktop\THIRD_PARTY_LICENSES.md" "$dist\THIRD_PARTY_LICENSES.md"
Copy-Item "$root\assets\fonts\OFL-1.1.txt" "$dist\FONT-OFL-1.1.txt"
@"
安截 Anshot $version — Windows 截图、标注、遮挡后分享

运行：双击 Anshot.exe。程序常驻托盘；按 Ctrl+Shift+A 截图，或打开 / 粘贴 / 拖入 PNG、JPEG。
编辑：V 选择 · A 箭头 · R 框 · T 文字 · N 序号 · H 高亮 · B 实心遮挡 · M 安全马赛克 · C 裁剪；1–6 选色。
交付：Ctrl+C 复制，Ctrl+S 另存为新 PNG（不覆盖已有文件）。
设置：Ctrl+, 或托盘“设置…”可更换截图快捷键、复制后是否隐藏编辑窗、默认保存文件夹、登录后在托盘待命（开机启动）。

隐私：图片只在本机处理，不上传、无账号、无遥测。遮挡区域导出为不含原图信息的像素；
请自行检查未遮挡的内容。本软件未签名、未经安全审计，不承诺自动发现隐私信息。

许可：MIT（LICENSE.txt）。界面使用 Slint（Royalty-free 2.0，见“关于”）；字体 Noto Sans CJK SC（OFL-1.1）。
"@ | Set-Content -Encoding utf8 "$dist\README.txt"

$sums = Get-ChildItem $dist -File | Where-Object Name -ne 'SHA256SUMS.txt' | Sort-Object Name | ForEach-Object { "$((Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower())  $($_.Name)" }
$sums | Set-Content -Encoding ascii "$dist\SHA256SUMS.txt"
$zip = Join-Path $root "dist\$name.zip"
if (Test-Path $zip) { [System.IO.File]::Delete($zip) }
Compress-Archive -Path "$dist\*" -DestinationPath $zip
"PACKAGED $zip"
$sums
