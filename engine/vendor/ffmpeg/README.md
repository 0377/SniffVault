# Vendor ffmpeg

本目录存放各平台 **ffmpeg 可执行文件**，供 HLS 分片合并为 MP4 使用。二进制体积较大，**不纳入 git**；开发机与 CI 通过 `scripts/fetch_ffmpeg.sh` 拉取。

## 目录结构

```
vendor/ffmpeg/
  README.md
  {os}-{arch}/ffmpeg          # Unix / macOS
  {os}-{arch}/ffmpeg.exe      # Windows
```

`{os}-{arch}` 与 Rust `std::env::consts` 一致，例如：

| 平台 | 目录 |
|------|------|
| macOS Apple Silicon | `macos-aarch64/` |
| macOS Intel | `macos-x86_64/` |
| Linux x86_64 | `linux-x86_64/` |
| Linux aarch64 | `linux-aarch64/` |
| Windows x86_64 | `windows-x86_64/` |
| Android arm64-v8a | `android-arm64-v8a/` |
| Android armeabi-v7a | `android-armeabi-v7a/` |
| Android x86 | `android-x86/` |
| Android x86_64 | `android-x86_64/` |

引擎通过 `BundledFfmpegLocator` 解析路径，优先级：

1. **macOS 应用包**：`Contents/Resources/ffmpeg`（`flutter build macos` / `flutter run -d macos` 时由 `app/macos/Runner/copy_ffmpeg.sh` 从 vendor 复制并签名）
2. **Android 应用数据目录**：`{data_dir}/bin/ffmpeg`（启动时由 `MainActivity` 从 APK assets 解压）
3. **开发 / 测试**：`engine/vendor/ffmpeg/{os}-{arch}/ffmpeg`

## 获取二进制

在 `engine/` 目录执行：

```bash
./scripts/fetch_ffmpeg.sh
./scripts/fetch_ffmpeg_android.sh   # Android HLS 所需各 ABI
```

脚本行为：

- **macOS**：优先从 `PATH` 或 Homebrew 复制；若无则尝试 `brew install ffmpeg`
- **Linux / Windows**：优先使用 `PATH` 中的 `ffmpeg`；若无则从 [BtbN FFmpeg Builds](https://github.com/BtbN/FFmpeg-Builds/releases) 自动下载对应 `linux64` / `linuxarm64` / `win64` 包

复制完成后会 `chmod +x`。

## 验证

```bash
os="$(uname -s | tr '[:upper:]' '[:lower:]')"; [[ "${os}" == darwin ]] && os=macos
arch="$(uname -m)"; [[ "${arch}" == arm64 ]] && arch=aarch64
test -x "vendor/ffmpeg/${os}-${arch}/ffmpeg"
```

或直接运行测试（Task 6 起需要真实 ffmpeg）：

```bash
cargo test --manifest-path Cargo.toml
```

## 许可证

ffmpeg 为 LGPL/GPL 软件。分发应用时请遵守 [FFmpeg 许可证](https://ffmpeg.org/legal.html) 与所选构建版本的说明。
