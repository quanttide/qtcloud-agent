# qtcloud-agent

量潮智能体云 — 会话取用与注册中心对照 CLI（Rust）。

## 命令

### `qtcloud-agent session list` — 列出会话

按会话记录时的 `cwd` 判定归属：默认只列属于**当前目录**（含子目录）的会话，`--all` 不筛。会话按最后活动时间倒序。

```sh
qtcloud-agent session list                          # 当前目录
qtcloud-agent session list --cwd ~/repos/quanttide   # 指定目录
qtcloud-agent session list --all                     # 不按目录筛
qtcloud-agent session list --agent pi                # 只列某个 harness
qtcloud-agent session list --json                    # 机器读
```

输出每行：`agent / 会话 id / 体积 / 最后活动 / cwd`；`--json` 输出 `{target, count, sessions:[{agent,id,title,cwd,bytes,messages,modified_unix,origin}]}`。

支持的 harness（`--agent` 取值，默认全部）：

| harness | 会话存储 | cwd 来源 |
|---|---|---|
| `dsh` | `$DSH_HOME/sessions/<工作区>/<id>/`，默认 `~/.dsh` | `storages/session_projcache.json` |
| `pi` | `~/.pi/agent/sessions/<工作区>/<时间戳>_<uuid>.jsonl` | 首行 `cwd` |
| `opencode` | `~/.local/share/opencode/opencode.db` 的 `session` 表 | `directory` 列 |
| `hermes` | `~/.hermes/state.db` 的 `sessions` 表 | `cwd` 列（89 条里 29 条有值） |

### `qtcloud-agent session show` — 看一个会话

```sh
qtcloud-agent session show <id>              # 在四家存储里找
qtcloud-agent session show <id> --agent pi   # 只在一家找
```

输出 agent、cwd、title、消息数、体积、最后活动、存储位置；找不到退出码 1。

### `qtcloud-agent session audit` — 审计敏感串

对目录（递归）做只读扫描，按六类计数：密钥形态、邮箱、人名、部署域名、桶名、家目录路径。各模式独立计数，同一处可能被多个模式命中。

```sh
qtcloud-agent session audit <目录>
qtcloud-agent session audit <目录> --json
```

命中即退出码 1——用它卡「这份导出能不能发出去」。量潮专用的域名/桶名/人名清单在 `src/audit.rs` 顶部，改这里。

### `qtcloud-agent registry status` — 对照注册中心与本机配置

遍历注册中心（默认 `data/profile/default/agents`，可用 `--path` 或环境变量 `QTCLOUD_AGENT_REGISTRY` 覆盖）下的每个 agent 目录，
与本机配置（`~/.config/zed`、`~/.config/opencode`、`~/.hermes`）逐文件比对：

- 文件名对齐：同名 → 同 stem 同扩展名族（`.json`/`.jsonc` 一族，`.yaml`/`.yml` 一族）
- JSON/JSONC 展平成 `key.path` 后逐键比对，`.jsonc` 的行注释与尾逗号先去掉
- 注册中心登记了但本机没有映射路径的 agent 判为「无法比较」，单列，不并进一致
- 本地没登记过的配置文件默认汇总一行，`--verbose` 逐个展开

```sh
qtcloud-agent registry status              # 默认注册中心
qtcloud-agent registry status --verbose    # 展开本地自定义文件
qtcloud-agent registry status --path <p>   # 指定注册中心
```

## 退出码

| 码 | 含义 |
|---|---|
| 0 | 有结果 / 一致 / 无命中 |
| 1 | 无匹配 / 有差异 / 有探测命中 |
| 2 | 用法错（clap） |
| 3 | 数据源读不到（路径不存在、注册中心为空） |

## 构建与测试

```sh
cd src/cli
cargo build --release
cargo test
```
