# 项目协作指南

## 项目定位

- 本项目是开源、自编译安装的个人离线视频库，支持在用户有权使用的前提下嗅探、解析、缓存和播放视频。
- 一期目标平台为 Android、iOS、Windows、macOS 和 Android TV；各端片库默认独立，仅支持手机/电脑向 TV 投送已缓存内容。
- 不提供侵权片源导航、DRM 绕过、站点破解插件或未经用户授权的数据采集能力。

## 仓库结构

- `engine/`：Rust 核心，负责领域类型、解析、下载、任务、片库和 LAN 能力。
- `app/`：Flutter UI、播放桥接与 FFI 调用方；目录尚未创建时按计划初始化。
- `platforms/`：iOS Share Extension、WebView 钩子等最小原生胶水；目录尚未创建时按计划初始化。

## 需求来源

- 开始实现前，先阅读 `README.md`、最接近任务的规格和对应实现计划。
- 规格、计划与代码冲突时不要猜测：指出冲突，优先修正规格/计划或请求用户确认，再继续实现。
- 修改领域行为、公共接口、持久化结构或平台范围时，同步更新相关规格、计划和 README。

## 工作方式

- 开工前运行 `git status --short`；现有修改默认属于用户，禁止覆盖、回滚或混入无关改动。
- 功能和缺陷修复遵循 TDD：先写能说明行为的失败测试，再实现最小改动使其通过。
- 保持改动聚焦；不顺手重构无关模块，不提前实现后续阶段功能。
- 优先使用项目已有依赖和模式；新增生产依赖前说明必要性与替代方案。
- 除非用户明确要求，不创建 commit、分支、tag 或 PR。需要提交时使用中文提交信息并保持单一意图。

## Rust Engine 约束

- `Engine` 是领域数据的唯一公共写入口；`LibraryStore`、`TaskStore` 等底层存储保持 crate-private。
- 父任务及其子任务、片库条目及其分集必须在单一 SQLite 事务中原子写入，失败不得留下部分状态。
- `media_dir` 必须是相对 `data_dir` 的单层普通目录名；读取和保存设置时使用相同校验。
- 写入片库的文件必须 canonicalize 后位于当前 `media_dir` 下；不得只做字符串前缀判断。
- Series 合并键为 `kind=series + title + season`，包括 `season=None`；数据库唯一约束必须兜底。
- 分集唯一键为 `(item_id, episode_index)`；重复登记更新文件信息但保留播放进度。
- 每个可下载源使用同时包含 URL、媒体类型和清晰度的模型；不能保存无法映射回 URL 的清晰度选项。
- `source_url` 可能包含鉴权参数，仅限本机持久化；投送或导出前必须脱敏或剥离。

## 验证命令

在仓库根目录执行：

```bash
cargo fmt --manifest-path engine/Cargo.toml --all -- --check
cargo test --manifest-path engine/Cargo.toml
cargo clippy --manifest-path engine/Cargo.toml --all-targets --all-features -- -D warnings
```

- 开发中至少运行与改动直接相关的测试。
- 交付前运行格式检查和完整测试；环境具备 Clippy 时运行严格静态检查。
- 不得声称通过未实际运行的检查；失败时报告命令和关键错误。

## Code Review Rules

- 标记任何绕过 `Engine` 写入领域数据的代码。
- 标记缺少事务的多行持久化、目录逃逸、Series 重复、重下清零播放进度和敏感 URL 外发问题。
- 标记只验证 happy path、没有覆盖重启持久化、失败回滚、`season=None` 或恶意 `media_dir` 的测试。
- 标记规格范围外的站点破解、DRM 绕过、账号/云同步或系统级抓包实现。

