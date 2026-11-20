# qtcloud-agent

量潮智能体云 — 会话取用与注册中心对照 CLI（Rust）。

## 命令

### `qtcloud-agent session list` — 列出会话

按会话记录时的 `cwd` 判定归属：默认只列属于**当前目录**（含子目录）的会话，`--all` 不筛。会话按最后活动时间倒序。

```sh
qtcloud-agent session list                          # 当前目录
qtcloud-agent session list --cwd ~/repos/quanttide   # 指定目录
qtcloud-agent session list --all                     # 不按目录筛
qtcloud-agent session list --agent dsh               # 只列 dsh
qtcloud-agent session list --json                    # 机器读
```

输出每行：`agent / 会话 id / 体积 / 最后活动 / cwd`；`--json` 输出 `{target, count, sessions:[{agent,id,workspace,cwd,bytes,modified_unix}]}`。

当前支持的 harness：`dsh`（DeepSeek Harness，数据目录 `$DSH_HOME`，默认 `~/.dsh`；`cwd` 从 `storages/session_projcache.json` 反查）。`workspace` 是会话目录名解码所得，含连字符的工作区会失真，`cwd` 以 projcache 为准。

退出码：`0` 有结果、`1` 无匹配（stderr 说明是没匹配还是存储读不到）、`2` 用法错、`3` 数据源读不到。

### `qtcloud-agent registry status` — 对照注册中心与本机配置

遍历注册中心（默认 `data/profile/default/agents`，可用 `--path` 或环境变量 `QTCLOUD_AGENT_REGISTRY` 覆盖）下的每个 agent 目录，
与本机配置（`~/.config/zed`、`~/.config/opencode`、`~/.hermes`）逐文件比对：JSON/JSONC 展平成 `key.path` 后逐键比，
输出每个 Agent 的一致 / 差异 / 缺失情况与总览。

```sh
qtcloud-agent registry status                        # 默认注册中心
qtcloud-agent registry status --path <path>          # 指定注册中心
```

退出码：`0` 一致、`1` 有差异、`3` 注册中心读不到。

## 构建

```sh
cd src/cli
cargo build --release
```
