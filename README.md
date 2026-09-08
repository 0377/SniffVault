# video_sniffing

[![CI](https://github.com/0377/SniffVault/actions/workflows/ci.yml/badge.svg)](https://github.com/0377/SniffVault/actions/workflows/ci.yml)

开源个人离线视频库：在应用内解析/嗅探视频资源，缓存到本地播放。支持 Android、iOS、Windows、macOS、Android TV（自编译安装）。

## 使用责任

本工具仅供用户缓存其有权离线使用的内容。请遵守当地法律法规与内容提供方条款。项目不提供任何侵权片源导航。

## 仓库结构

- `engine/` — Rust 核心（片库、任务、后续下载/解析/LAN）
- `app/` — Flutter UI（片库、浏览、任务、添加、播放器；Riverpod + go_router + media_kit）
- `platforms/` — 原生胶水（`webview_sniff`：Cookie 仓 + Android `isTelevision`；`share_ingress`：iOS Share Extension 源文件托管）

## 持续集成

合并到 `main` 前须通过 GitHub Actions：**fmt**（ubuntu）、**test + clippy**（Linux / macOS / Windows 三平台）、**flutter-test**（macOS 单元测试）、**flutter-integration**（macOS 引擎 FFI、UI 与深链 U8 集成冒烟，并行 job）。

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

# FFI 集成冒烟（需 macOS 设备；CI 在独立 job 中各跑一次）
flutter test integration_test/engine_smoke_test.dart -d macos
flutter test integration_test/ui_test.dart -d macos
flutter test integration_test/deep_link_test.dart -d macos
# 完整 UI 流程（含播放器，本地有 GUI 时）
# flutter test integration_test/ui_test.dart -d macos

# 浏览 U6 门禁（本地交付必须通过，不要 skip）
flutter test integration_test/browse_test.dart -d macos
# CI 无头环境可跳过浏览 WebView
# flutter test integration_test/browse_test.dart -d macos --dart-define=INTEGRATION_SKIP_BROWSE=true
```

### 测试依赖 ffmpeg

HLS 合并相关集成测试需要本机可用的 `ffmpeg`。在 `engine/` 目录执行：

```bash
./scripts/fetch_ffmpeg.sh
```

脚本会将当前平台的 `ffmpeg` 复制到 `engine/vendor/ffmpeg/{os}-{arch}/`（例如 macOS Apple Silicon 为 `macos-aarch64/ffmpeg`）。详见 [`engine/vendor/ffmpeg/README.md`](engine/vendor/ffmpeg/README.md)。

## 应用 UI（Plan 5）

主流程：启动应用 →「添加」粘贴 URL → 解析并入队 →「任务」查看进度 →「片库」播放已缓存内容。

```bash
cd app
flutter pub get
flutter run -d macos   # 或 android / ios / windows
flutter test
# CI 等价（跳过无头环境不稳定的播放器步骤）
flutter test integration_test/ui_test.dart -d macos --dart-define=INTEGRATION_SKIP_PLAYER=true
# 完整 U1–U3（含播放进度回写，需本机 GUI）
flutter test integration_test/ui_test.dart -d macos
```

规格见 `docs/superpowers/specs/2026-08-11-app-ui-player-design.md`。

## 内置浏览（Plan 6a）

主路径：添加页粘贴 URL → 解析返回 **NeedsBrowser** →「打开内置浏览」→ 浏览页加载该页 →「解析本页」→ 向导「下载」。也可直接点「浏览」，在地址栏打开页面后点「解析本页」（带着当前页 Cookie / Referer）。

Android / iOS / macOS 使用官方 `webview_flutter`（NavigationDelegate + 注入脚本）。官方包 **没有 Windows 实现**，Windows 上浏览页不可用。Android TV 隐藏浏览入口。

```bash
# 本地交付必须通过 U6（不要加 INTEGRATION_SKIP_BROWSE）
cd app && flutter test integration_test/browse_test.dart -d macos

# CI 可用 skip；U7（video src 嗅探候选）5 秒内无条目时可 skip，不是门禁
# flutter test integration_test/browse_test.dart -d macos --dart-define=INTEGRATION_SKIP_BROWSE=true
```

## 系统分享与深链（Plan 6b）

主路径：Android / iOS 系统「分享」选中本应用 → 主 App 打开添加 Tab 并预填 `http(s)` URL → 用户手动点「解析」（与粘贴行为一致，不自动下载）。

- Android：`MainActivity` 将 `ACTION_SEND` 文本归一为 `sniffvault://add?url=<encoded>`，供 `app_links` 读取。
- iOS：Share Extension（`platforms/share_ingress/ios/ShareExtension/`）读取分享载荷后 `extensionContext.open(sniffvault://…)` 唤起主 App。
- Flutter：`DeepLinkHost` 解析深链后 `router.go('/add?url=…')`；无效载荷打开 `/add` 并 SnackBar 提示。

```bash
# U8 门禁（本地交付必须通过）
cd app && flutter test integration_test/deep_link_test.dart -d macos
```

规格见 `docs/superpowers/specs/2026-09-08-system-share-design.md`。

## 许可证

本项目采用 [Apache License 2.0](LICENSE) 开源。
