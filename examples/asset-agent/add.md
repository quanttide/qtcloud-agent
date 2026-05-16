# 资产智能体规则引擎 — 核心设计文档

状态：已确认
日期：2026-05-16
领域：数字资产管理 × 智能体工程

—

1. 概述

1.1 项目定位

构建一个壳程序（Shell），用于可视化平台资产仓库中多智能体的协作关系。底层依赖规则引擎实现文件事件到智能体动作的路由。

1.2 规则引擎职责

· 监听资产目录的文件变化
· 根据预定义规则匹配触发条件
· 将事件路由到对应智能体（调用 Opencode）
· 记录智能体间的触发链路，供壳程序可视化

规则引擎不负责智能体的内部逻辑，只负责编排。

—

2. 核心架构

2.1 资产树结构

```
元仓库/
├── roadmap/
├── 示例/
│   ├── docs/
│   ├── test/
│   ├── src/
│   └── config/
├── 平台/
│   ├── docs/
│   │   ├── drd/
│   │   └── qa/
│   ├── test/
│   ├── src/
│   └── config/
└── 库/
    ├── docs/
    ├── test/
    ├── src/
    └── config/
```

2.2 智能体分层

层级 智能体 管辖范围
元仓库 meta-agent /, roadmap/*, */STATUS.md
单仓 platform-agent 等 各自单仓根目录
分层 doc-agent, qa-agent, code-agent, test-agent 等 对应子目录

2.3 数据流

```
文件系统事件 → 规则引擎 → 条件匹配 → 动作执行 → 记录协作图
                  ↑                        │
                  └── agent_rules.yaml      └── 调用 Opencode
```

—

3. 核心设计决策

3.1 统一事件格式（基于 CloudEvents 思想）

所有智能体间通信采用统一的轻量 JSON 格式：

```json
{
  ”id“: ”evt-20260516-001“,
  ”type“: ”drd.finalized“,
  ”source“: ”/平台/docs/drd“,
  ”time“: ”2026-05-16T10:30:00Z“,
  ”subject“: ”支付设计.md“,
  ”datacontenttype“: ”application/json“,
  ”data“: {
    ”status“: ”final“,
    ”author“: ”human“
  }
}
```

字段说明：

· id：全局唯一标识，用于去重和日志
· type：事件类型（如 drd.finalized、qa.change.request），驱动规则匹配
· source：产生事件的智能体或目录
· time：发生时间
· subject：受影响的资产文件
· data：业务载荷

设计理由：借鉴 CloudEvents 核心字段，获得标准化语义的同时保持极简实现，零外部依赖。

3.2 文件即事件

事件载体为文件系统中的 JSON 文件，存放于约定位置：

```
{管辖目录}/.events/{event-id}.json
```

规则引擎监听这些文件的变化，解析后触发下游动作。传递机制完全基于文件系统，无额外消息队列。

3.3 正向与反向触发

正向触发：上游完成 → 通知下游

· 例：drd 文档状态变更为 final → 触发 qa-agent 和 code-agent

反向触发（反馈）：下游发现问题 → 向上游发送变更请求

· 例：qa-agent 发现设计问题 → 向 drd 发送 design.change.request 事件，携带问题描述和修改建议

两种触发都使用同一事件格式，通过 type 字段区分。

—

4. 规则定义规范

4.1 规则文件格式（agent_rules.yaml）

```yaml
agents:
  - id: drd-agent
    watch:
      - ”平台/docs/drd/*.md“
    command: ”opencode run drd-agent —file ${FILE}“
    outputs:
      - ”平台/docs/drd/*.md“

  - id: qa-agent
    watch:
      - ”平台/docs/qa/*.md“
      - ”平台/docs/drd/.events/*.json“
    command: ”opencode run qa-agent —file ${FILE}“
    outputs:
      - ”平台/docs/qa/*.md“
```

4.2 带条件的规则（扩展语法）

```yaml
rules:
  - name: ”DRD定稿通知“
    trigger:
      file_pattern: ”平台/docs/drd/*.md“
      condition: ”content.status == ’final‘“
    action:
      - notify:
          target: agent/qa
          payload:
            type: ”drd.finalized“
            source_file: ”${trigger.file}“
      - notify:
          target: agent/code
          payload:
            type: ”design.finalized“
            source_file: ”${trigger.file}“

  - name: ”QA反馈变更请求“
    trigger:
      file_pattern: ”平台/docs/qa/*.md“
      condition: ”event_type == ’design.change.request‘“
    action:
      - notify:
          target: agent/drd
          payload:
            type: ”design.change.request“
            message_file: ”${trigger.file}“
```

4.3 规则字段说明

字段 说明
watch glob 模式，定义智能体关心的文件
command 触发后执行的命令，${FILE} 为触发文件路径
outputs 声明可能的输出文件，用于推断上下游关系
condition 触发条件表达式（可选，如解析 Markdown frontmatter）
action.notify 通知目标智能体，携带结构化 payload

—

5. 协作关系图

5.1 图结构

· 节点：智能体（id、监控目录、最近活跃时间）
· 有向边：触发者 → 被触发者，属性包含时间戳、触发文件、事件类型

5.2 典型链路

```
meta-agent → platform-agent → doc-agent → qa-agent
                                  ↑            │
                                  └────────────┘
                              (change.request)
```

—

6. 实现要点

6.1 技术选型

· 监听：Python watchdog 或 Node.js chokidar
· 规则解析：YAML
· 图维护：networkx（Python）或 JSON 持久化
· 可视化：TUI 或轻量 Web 仪表盘（壳程序负责）

6.2 防循环触发

· 同一文件在冷却期（如 2 秒）内重复事件忽略
· 智能体执行期间可暂停监听

6.3 与 Opencode 集成

优先使用命令行调用：

```bash
opencode run <agent-name> —file <trigger-file>
```

—

7. 下一步计划

1. 完善 agent_rules.yaml，覆盖所有单仓和分层智能体
2. 实现事件监听与 condition 解析（解析 Markdown frontmatter 中的状态字段）
3. 构建协作图可视化壳程序
4. 与 Opencode 联调，验证完整闭环链路

—

文档版本：v1.0
确认范围：事件格式、规则结构、触发模式、技术路线均已确认