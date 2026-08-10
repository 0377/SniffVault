# video_sniffing

开源个人离线视频库：在应用内解析/嗅探视频资源，缓存到本地播放。支持 Android、iOS、Windows、macOS、Android TV（自编译安装）。

## 使用责任

本工具仅供用户缓存其有权离线使用的内容。请遵守当地法律法规与内容提供方条款。项目不提供任何侵权片源导航。

## 仓库结构

- `engine/` — Rust 核心（片库、任务、后续下载/解析/LAN）
- `app/` — Flutter UI（后续计划）
- `platforms/` — 极少原生胶水（后续计划）

## 构建引擎

```bash
cd engine && cargo test
```

## 许可证

本项目采用 [Apache License 2.0](LICENSE) 开源。
