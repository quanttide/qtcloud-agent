# 资产智能体协作上下文 — 数据需求文档

文档状态：v1.0 草案
角色：数据产品经理
日期：2026-05-16

—

1. 数据域概述

整个系统围绕三类数据运转：

| 数据域 | 内容 | 归属 |
|--------|------|------|
| 资产数据 | 各小项目内的文档、代码、测试、配置等资产文件 | 各自小项目，跨域不可见 |
| 事件数据 | 智能体间传递的结构化消息，携带上下文 | .events/ 目录，按需路由 |
| 配置数据 | 项目分解定义、智能体分配、规则绑定 | agent_rules.yaml |

核心规则：**资产数据不出边界，事件数据只带摘要。**

—

2. 审计数据结构

审计数据记录一次 Agent 调用的上下文使用情况，是前后端交换的核心数据契约。

2.1 审计记录结构

```json
{
  "time": "14:30:05",
  "agent": "drd-agent",
  "tokens": 8230,
  "trigger": "平台/docs/drd/支付设计.md (finalized)",
  "files": [
    { "icon": "📄", "name": "支付设计.md", "tokens": 3200, "status": "ok" },
    { "icon": "📄", "name": "架构决策.md", "tokens": 1800, "status": "ok" },
    { "icon": "📄", "name": "平台技术选型.md", "tokens": 2300, "status": "waste" }
  ],
  "eventData": "2,300",
  "boundary": "✅ 未跨域",
  "json": "{...事件原始报文...}",
  "wasteRate": 38
}
```

字段说明：

| 字段 | 类型 | 说明 |
|------|------|------|
| time | string | 调用时间，HH:mm:ss |
| agent | string | 智能体 ID |
| tokens | number | 本次调用总 token 消耗 |
| trigger | string | 触发来源：文件路径或事件类型 |
| files | array | 上下文来源文件清单 |
| files[].icon | string | 图标：📄 本地文件 / 📨 事件 data |
| files[].name | string | 文件名 |
| files[].tokens | number | 该文件占用的 token 数 |
| files[].status | string | ok 引用 / waste 未用 / low 引用率低 |
| eventData | string | 事件 data 的 token 数（格式化） |
| boundary | string | 边界检查结果：✅ 未跨域 / 🔴 越权详情 |
| json | string | 事件原始报文（CloudEvents 格式 JSON 字符串） |
| wasteRate | number | 浪费占比（百分比） |
| warn | bool | 可选，浪费超 50% 标记 |
| violation | bool | 可选，越权标记 |

2.2 状态判定规则

| status | 判定 | 前端表现 |
|--------|------|---------|
| ok | Agent 输出引用了该文件内容 | 绿色 ✓ |
| waste | 加载了但 Agent 输出中未引用 | 红色 ✗ |
| low | 引用率低（< 10%） | 黄色 ⚡ |

warn = wasteRate > 50，violation = boundary 含"越权"。

2.3 配置数据结构

Agent 上下文边界配置存储为：

```json
{
  "drd-agent": {
    "rules": ["平台/docs/drd/*.md", "平台/docs/drd/.events/*.json"],
    "preview": ["📄 平台/docs/drd/支付设计.md", "..."],
    "count": 10,
    "tokens": "~15K"
  }
}
```

| 字段 | 说明 |
|------|------|
| rules | watch 路径列表，定义 Agent 的上下文边界 |
| preview | 预览边界内匹配的文件列表 |
| count | 边界内文件总数 |

注意：token 数量只能审计（跑完才知道实际消耗），不能在规划阶段预估，故规划数据中不包含 token 字段。

—

3. 数据流转契约

3.1 事件驱动数据交换

智能体之间不直接共享资产文件，所有跨域数据必须通过事件传递。

```
发送方                             接收方
┌─────────────┐                   ┌─────────────┐
│  自己的资产  │   事件 data 携带    │  只读事件    │
│  文件 + 提取 │ →  摘要上下文   →  │  data，不    │
│  摘要进 data │                   │  跨域读文件   │
└─────────────┘                   └─────────────┘
```

3.2 数据流按事件类型分类

| 事件类型 | 方向 | 发送方产出数据 | 接收方消费数据 | 不消费什么 |
|----------|------|---------------|---------------|-----------|
| drd.finalized | 正向 | 摘要、关键决策、变更列表 | data.summary + data.key_decisions | DRD 源文件全文 |
| design.change.request | 反向 | 问题描述、修改建议、引用位置 | data.issue + data.suggestion | QA 目录下其他文件 |
| test.passed | 正向 | 测试范围、通过率、失败详情 | data.coverage + data.failures | 测试用例源代码 |
| code.reviewed | 正向 | 审查结论、问题列表、建议 | data.conclusion + data.issues | 代码全文 |
| sync.request | 反向 | 差异摘要、冲突文件 | data.conflict_summary + data.files | 仓库 git 历史 |

3.3 数据映射：从资产文件到事件 data

每种资产文件类型定义固定的 data 映射规则，确保事件携带足够的业务上下文。

```
源文件: DRD 文档 (Markdown)
  ↓ 提取
  data.summary       ← 文档开头概要段落（前 200 字）
  data.status        ← frontmatter.status 字段
  data.key_decisions ← ## 关键决策 章节下的列表
  data.changed_files ← git diff 中变更的文件列表
  data.review_target ← frontmatter.review_required 字段

源文件: QA 报告 (Markdown)
  ↓ 提取
  data.issue         ← 问题描述（前 200 字）
  data.severity      ← 严重级别 (blocker/critical/major/minor)
  data.suggestion    ← 建议方案章节
  data.related_doc   ← 引用的 DRD 文档路径
```

—

4. 数据边界

4.1 小项目数据边界

每个小项目是一个封闭数据域：

```
平台/docs/                    ← doc-agent 的数据边界
├── 允许加载：该目录下所有 .md 文件
├── 允许生成：该目录下的新文件、更新文件
├── 允许输出：事件 data（摘要而非全文）
└── 不允许：读取平台/test/、平台/src/ 下的任何文件

平台/test/                    ← test-agent 的数据边界
├── 允许加载：该目录下所有文件
├── 允许生成：测试报告、测试日志
├── 允许输出：事件 data（测试摘要而非源码）
└── 不允许：读取平台/docs/ 下的文件
```

4.2 事件数据边界

事件 data 是唯一合法的跨域数据载体，但其本身也受约束：

· data 字段只携带摘要、结论、关键信息，不携带源文件全文
· data 不引用边界外文件的内部细节（如行号、变量名），除非接收方有权限
· data 不可包含敏感信息（如密钥、凭据）

4.3 数据流向约束

```
合法流向：
  资产文件 → 同项目智能体 ✓
  资产文件 → 事件 data（摘要） → 跨项目智能体 ✓
  事件 data → 智能体任务执行 ✓

非法流向：
  资产文件 → 跨项目智能体直接读取 ✗
  事件 data → 转存为接收方资产文件后未脱敏 ✗
```

—

5. 数据质量要求

| 维度 | 要求 | 判定方式 |
|------|------|---------|
| 完整性 | 事件 data 包含接收方完成任务所需的核心上下文 | 人工审计，看 ✗ 标记的文件是否确实不需要 |
| 准确性 | data 摘要准确反映源文件内容 | 抽样人工核对 |
| 边界合规 | 不允许加载上下文边界外的文件 | 自动检测，violation 标记 |
| 精简度 | 浪费率（wasteRate）越低越好，目标 < 50% | 审计面板直接展示 |

—

6. 数据生命周期

```
资产文件变更
    ↓
提取摘要（产生事件 data）
    ↓
事件持久化到 .events/{event-id}.json
    ↓
路由到目标智能体（文件复制或链接）
    ↓
智能体消费 data + 自己小项目内的资产文件
    ↓
（可选）产生新事件 → 继续流转
    ↓
事件归档：.events/ 中保留最近 30 条，旧事件自动清理
```

关键设计：事件 data 是一次性的上下文快照。接收方不应依赖事件 data 的长期可用性，重要信息应落地为自己的资产文件。

—

7. 数据成功指标

| 指标 | 说明 | 目标值 |
|------|------|--------|
| data 命中率 | 智能体输出引用的信息中，来自 data 的占比 | > 30%（其余来自小项目内文件） |
| 跨域读文件率 | 智能体主动读取边界外文件的次数占比 | < 5% |
| data 精简率 | data 实际 token / 源文件 token | < 20%（即压缩比 > 5倍） |
| 事件重试率 | 因 data 信息不足导致接收方要求重发的比例 | < 2% |
| 数据迟到率 | 事件延迟超过 5 秒的比例 | < 1% |

—

文档版本：v1.0
与其它文档的关系：
- brd.md 定义"为什么要拆"，本文定义"拆完后数据怎么流"
- add.md 定义"引擎怎么实现"，本文定义"引擎要处理哪些数据"
- ixd.md 定义"界面怎么展示"，本文定义"界面要展示什么数据"
