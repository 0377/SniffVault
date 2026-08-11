# video_sniffing

[![CI](https://github.com/0377/SniffVault/actions/workflows/ci.yml/badge.svg)](https://github.com/0377/SniffVault/actions/workflows/ci.yml)

开源个人离线视频库：在应用内解析/嗅探视频资源，缓存到本地播放。支持 Android、iOS、Windows、macOS、Android TV（自编译安装）。

## 使用责任

本工具仅供用户缓存其有权离线使用的内容。请遵守当地法律法规与内容提供方条款。项目不提供任何侵权片源导航。

## 仓库结构

- `engine/` — Rust 核心（片库、任务、后续下载/解析/LAN）
- `app/` — Flutter UI（后续计划）
- `platforms/` — 极少原生胶水（后续计划）

## 持续集成

合并到 `main` 前须通过 GitHub Actions：**fmt**（ubuntu）、**test + clippy**（Linux / macOS / Windows 三平台）、**flutter-smoke**（macOS，单元测试与 FFI 集成冒烟）。

本地可运行与 CI 相同检查：

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml --workspace
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
cd app && flutter pub get && flutter test
```

发版：推送 `v*` tag（如 `v0.1.0`）触发 Release workflow（`.github/workflows/release.yml`），质量检查通过后自动创建 GitHub Release。

## 构建引擎

```bash
cd engine && cargo test --workspace
```

解析与嗅探（Plan 3）：`Engine::resolve_url`、`resolve_qualities`、`sniff_urls`。

## Flutter + FFI

`app/` 通过 Cargokit（`rust_lib_video_sniffing`）在构建时编译 `engine/ffi`；`app/rust` 与 `app/rust_builder/rust` 均符号链接至 `engine/ffi`。

前置：安装 [Flutter](https://docs.flutter.dev/get-started/install) stable，macOS 桌面需启用 desktop 支持。

```bash
# 引擎 workspace（含 ffi crate）
cargo test --manifest-path engine/Cargo.toml --workspace

# macOS 桌面构建与测试
flutter config --enable-macos-desktop
cd app
flutter pub get
flutter test
flutter build macos --debug

# FFI 集成冒烟（需 macOS 设备）
flutter test integration_test/smoke_test.dart -d macos
```

### 测试依赖 ffmpeg

HLS 合并相关集成测试需要本机可用的 `ffmpeg`。在 `engine/` 目录执行：

```bash
./scripts/fetch_ffmpeg.sh
```

脚本会将当前平台的 `ffmpeg` 复制到 `engine/vendor/ffmpeg/{os}-{arch}/`（例如 macOS Apple Silicon 为 `macos-aarch64/ffmpeg`）。详见 [`engine/vendor/ffmpeg/README.md`](engine/vendor/ffmpeg/README.md)。

## 许可证

本项目采用 [Apache License 2.0](LICENSE) 开源。
