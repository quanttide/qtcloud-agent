# qtcloud-agent

量潮智能体云 — Agent 注册中心 CLI（Rust）。

## 命令

### `qtcloud-agent status`

对比注册中心（`data/profile/agents/`）与本地 Agent 配置（`~/.config/zed`、`~/.config/opencode`、`~/.hermes`），
输出每个 Agent 的配置一致 / 差异 / 缺失情况与总览。

```sh
qtcloud-agent status                      # 使用默认注册中心 data/profile/
qtcloud-agent status --registry <path>    # 指定注册中心路径
```

### `qtcloud-agent dsh` — 本地 DSH（DeepSeek Harness）管理

| 子命令 | 说明 |
|--------|------|
| `dsh status` | 检查本地 DSH：安装目录与版本（`dsh --version` / 包目录 package.json）、CLI 路径、`DSH_HOME` 数据目录（profiles / sessions / storages）、`dsh web` 服务进程（/proc）与 Web UI 端口（默认 3080）响应 |
| `dsh sessions` | 列出 `$DSH_HOME/sessions/` 下的会话，按工作区分组，显示会话大小、最后活动时间，并从 `session_projcache.json` 还原真实工作目录 |
| `dsh config` | 显示 `$DSH_HOME/settings.yaml` 与各 profile（web / headless）的插件 bundle 清单 |

```sh
qtcloud-agent dsh status --port 3080      # 指定 Web UI 端口
```

DSH 数据目录默认 `~/.dsh`，可用环境变量 `DSH_HOME` 覆盖。

## 构建

```sh
cd src/cli
cargo build --release
```
