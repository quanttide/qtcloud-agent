# Changelog

本文件仅记录 **cli（量潮智能体云 CLI，Rust）** 的版本变更。

## [Unreleased]

## [0.1.0-alpha.1] - 2026-11-20

首个 alpha。命令面围绕「会话取用」与「注册中心对照」两件事，其余历史命令已删。

### Added

- `session list`：按会话记录时的 `cwd` 判定归属，默认只列当前目录（含子目录）的会话，`--cwd` 换目录、`--all` 不筛、`--agent` 限定 harness、`--json` 机器读；
- `session show <id>`：跨 harness 按 id 查会话详情，给 agent、cwd、消息数、体积、最后活动与存储位置；
- `session audit <路径>`：只读扫描目录或文件，按六类（密钥形态、邮箱、人名、部署域名、桶名、家目录路径）计数，命中即退出码 1；
- `registry status`：对照注册中心与本机 agent 配置，未知 agent 单列「无法比较」，文件名按 stem 加扩展名族对齐（`.json` 与 `.jsonc` 一族），本地自定义文件默认汇总一行、`--verbose` 展开；
- 会话 reader 覆盖 dsh、pi、opencode、hermes 四家存储，读出来统一成同一记录形状；
- 退出码约定：0 有结果或一致、1 无匹配或有差异、2 用法错、3 数据源读不到；
- CI `release-cli`：`cli/*` tag 触发，检查版本与 CHANGELOG，跑 fmt、test、clippy 严格门禁与 `cargo publish --dry-run`，出三平台二进制挂到 Release，再推 crates.io。

### Changed

- 默认注册中心路径改为 `data/profile/default/agents`，provider 的默认路径同步修正；
- 退出码不再一律返回 0。

### Removed

- `dsh status`、`dsh sessions`、`dsh config` 三个子命令。
