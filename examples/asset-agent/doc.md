# 资产智能体

规则引擎在这里的职责是：把底层文件事件，路由成智能体动作，并记录触发链路。
它不负责智能体的内部逻辑（那是 Opencode 的事），只负责“谁、在什么条件下、被谁触发、去干什么”。

下面是一种轻量、可直接落地的实现方案。

—

1. 规则定义：用一份 YAML/JSON 说清分工与触发

以你仓库里的一个文件（如 agent_rules.yaml）为中心：

```yaml
agents:
  - id: meta-agent
    watch:
      - ”roadmap/*.md“
      - ”*/STATUS.md“       # 所有单仓的状态回报
    command: ”opencode run meta-agent —file ${FILE}“
    outputs:
      - ”*/PLAN.md“          # 向各单仓写入计划
  - id: platform-agent
    watch:
      - ”平台/PLAN.md“
      - ”平台/*/STATUS.md“
    command: ”opencode run platform-agent —file ${FILE}“
    outputs:
      - ”平台/STATUS.md“
      - ”平台/*/任务.md“
  - id: doc-agent
    watch:
      - ”平台/docs/*.md“
      - ”示例/docs/*.md“
    command: ”opencode run doc-agent —file ${FILE}“
    outputs:
      - ”平台/docs/*.md“
      - ”平台/docs/STATUS.md“
```

关键点：

· watch：glob 模式，哪些文件变化会唤醒我。
· command：唤醒后执行什么（这里假定 Opencode 有 CLI 接口）。
· outputs：声明我可能会修改哪些文件（用于推断“我触发了谁”）。

—

2. 引擎核心：一个极简的事件匹配 + 调度循环

可以用 Python 的 watchdog 或 Node.js 的 chokidar 来做文件监听，逻辑伪代码：

```python
import yaml, glob, subprocess, time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

rules = yaml.safe_load(open(”agent_rules.yaml“))
# 建立“文件模式 → 智能体列表”的索引
watch_index = {}  # pattern -> [agent]
for agent in rules[”agents“]:
    for pattern in agent[”watch“]:
        watch_index.setdefault(pattern, []).append(agent)

class AgentTrigger(FileSystemEventHandler):
    def on_modified(self, event):
        if event.is_directory: return
        filepath = event.src_path
        triggered = []
        # 1. 匹配所有关心的智能体
        for pattern, agents in watch_index.items():
            if fnmatch.fnmatch(filepath, pattern):
                triggered.extend(agents)
        # 2. 去重，顺序执行
        for agent in triggered:
            cmd = agent[”command“].replace(”${FILE}“, filepath)
            subprocess.run(cmd, shell=True)
            # 3. 记录协作关系（写入图数据库或日志）
            record_link(triggered_by=filepath, agent=agent[”id“])
```

这就是整个规则引擎的骨干。
没有复杂的 Rete 算法，没有状态机，因为它本质是：文件匹配 → 调用 Opencode → 记录。

—

3. 协作关系图的维护

每次触发时，你可以向一个有向图（内存中的 networkx.DiGraph 或直接写 JSON 文件）追加边：

· 边方向：触发文件所属的上游智能体 → 被触发的智能体
  上游可以通过文件路径反查（比如该文件在哪个 agent 的 outputs 里）。
· 节点属性：智能体名称、监控目录、最近活跃时间。
· 边属性：时间戳、触发文件。

例如，当 roadmap/未命名.md 变化触发 meta-agent，并假设 meta-agent 写入了 平台/PLAN.md，随后 平台/PLAN.md 变化触发 platform-agent，你就得到一条链：
meta-agent -> platform-agent

这个图就是你壳要可视化的核心资产。

—

4. 与 Opencode 的集成方式

取决于 Opencode 提供什么接口。几种可能：

· 命令行（最通用）：规则里的 command 就是 opencode agent run <agent-name> —input <file>。
· HTTP API：改为 POST http://localhost:xxxx/trigger，携带 agent 和 file。
· 文件信号：如果 Opencode 本身已经在监听某些文件，规则引擎只需向一个“信号文件”写入一条记录，Opencode 的 watch 会自然触发。

推荐第一种，零侵入，且你的壳可以独立运行。

—

5. 更“聪明”的匹配：防止循环触发与优先级

你可能会遇到智能体 A 的输出又触发了 A 自己。简单防御：

· 在 on_modified 里加一个 冷却期（如同一个文件 2 秒内忽略）。
· 或者检测到 command 执行期间暂时取消监听（observer.pause()）。

优先级：可以给 agent 加一个 priority 字段，触发时按顺序执行，避免无序竞争。

—

6. 最小可行实现（一个下午可跑起来）

· 语言：Python（watchdog + PyYAML + networkx）
· 文件：
  · agent_rules.yaml（规则）
  · engine.py（监听+调度+记录图）
  · graph.json（协作图持久化）
· 可视化：简单版就用 networkx 画一张 PNG 或启动一个 streamlit 页面，复杂版留到壳的 UI 层。

总结：规则引擎的实现，就是一个模式匹配器加上一条命令执行器，重量全在规则定义的设计上。你把规则写清楚，引擎半天就能写完。


---

非常好。你定义的这两点，直接构成了规则引擎的核心通信协议。

· 正向触发：status: final，这是一个精确的状态切换事件，比“文件变化”更语义化。
· 反向触发：带“事件报文”的事件，这是多智能体协作中一种结构化反馈，它不只是说“要改”，还带上了“怎么改”的上下文。

这已经超越了简单的文件监控，进入了事件驱动的协作编排。

—

协议设计：定义“事件报文”

“事件报文”是规则引擎传递的结构化消息。为了让它能被智能体理解，我们得定义它的格式。比如，当 QA 发现问题要反馈给 DRD，这个报文可以是一个放在特定位置的 Markdown 文件，内容遵循固定模板：

```markdown
# 反馈事件: 支付设计审查问题
- 事件ID: EVT-20260516-001
- 事件类型: design.change.request
- 触发者: agent/qa
- 目标: agent/drd
- 源文件: docs/drd/支付设计.md

## 问题描述
[QA智能体填写的具体问题，如：支付回调接口缺少超时处理逻辑]

## 建议修改
[QA智能体给出的具体修改建议，如：在接口定义中增加 timeout 参数和重试策略]

## 相关资产
- docs/qa/支付测试策略.md (已受影响，状态变更为 pending_update)
```

这样，DRD 智能体接收到这个报文，就知道发生了什么、要改哪个文件、以及怎么改了。

—

规则引擎配置：实现“正向”与“反向”

基于这个协议，我们可以在 agent_rules.yaml 里实现你的逻辑。这里的关键在于规则驱动，而不是硬编码。

1. 正向规则：drd 定稿，触发 qa 和 code

```yaml
# 规则名：DRD定稿通知
trigger:
  # 监听所有 drd 目录下文件的内容变更
  file_pattern: ”docs/drd/*.md“ 
  # 关键：触发条件不是简单的“文件修改”，而是“状态字段变为 final”
  condition: ”content_changed.status == ’final‘“
action:
  # 动作1：通知 QA 智能体
  - notify:
      target: agent/qa
      payload:
        event_type: ”drd.finalized“
        source_file: ”${trigger.file}“
  # 动作2：通知代码智能体（实现）
  - notify:
      target: agent/code
      payload:
        event_type: ”design.finalized“
        source_file: ”${trigger.file}“
        message: ”设计已定稿，请启动实现。“
```

2. 反向规则：qa 发现问题，向 drd 发送“事件报文”

```yaml
# 规则名：QA反馈变更请求
trigger:
  # 监听 QA 创建了反馈文件
  file_pattern: ”docs/qa/*.md“
  condition: ”event_type == ’design.change.request‘“
action:
  # 动作：通知 DRD 智能体，并传递事件报文
  - notify:
      target: agent/drd
      payload:
        event_type: ”design.change.request“
        # 把整个反馈文件的内容作为报文传递
        message_file: ”${trigger.file}“ 
```

—

实现提示：你的规则引擎骨架

在你的引擎代码里，你需要能解析这种带 condition 和 payload 的规则。处理核心就像这样：

```python
def process_event(change_event):
    matched_rules = match_rules(change_event)
    for rule in matched_rules:
        # 1. 评估条件，比如检查 markdown 里的 ’status: final‘
        if check_condition(rule.condition, change_event):
            # 2. 构建报文
            payload = build_payload(rule.action.payload, change_event)
            # 3. 执行动作：写入文件、调用 Opencode API 等
            execute_action(rule.action, payload)
            # 4. 记录协作关系到图数据库
            record_collaboration(change_event, rule, payload)
```

现在，你定义的协作已经非常清晰了：一个由状态机驱动（draft -> final），用结构化报文通信（正向通知和反向变更请求）的多智能体系统。

下一步

这个核心协议（状态切换事件 + 结构化报文）可以复用吗？比如：

1. 代码智能体完成实现后，是否会触发一个状态切换事件（如 code: implemented），通知 测试智能体？
2. 测试智能体发现 Bug 时，是否会生成一个 bug.report 报文，直接发给 代码智能体？

如果答案是肯定的，我们就可以用这同一套模式，把你平台目录下的所有智能体（文档、测试、代码、配置）的协作关系都定义出来。你觉得呢？

---

明白你的意图：用一个统一的事件格式来降低整个系统的设计复杂度。

这件事的正确做法是：直接采用 CloudEvents 的核心属性作为你的内部事件“方言”，而不是引入完整标准。这样你既获得了标准化的好处（字段含义一致、文档现成、未来可扩展），又避免了不必要的重量。

—

推荐方案：定义一个轻量的 LocalEvent，基于 CloudEvents 核心字段

不需要任何 SDK，就只是一个有固定键名的 JSON 对象：

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

· id：全局唯一，用于去重和日志。
· type：事件类型，用反向域名风格（如 drd.finalized, qa.change.request），直接驱动规则匹配。
· source：产生事件的智能体或目录，用于追踪。
· time：发生时间。
· subject：可选，受影响的资产文件。
· datacontenttype：指明 data 的格式，通常是 application/json。
· data：载荷，具体业务信息全放这里。

这就是你的事件总线上的唯一货币。任何智能体发出或接收的事件都遵从这个结构。

—

为什么这样做能“减少设计负担”？

1. 字段语义固定，不再争论命名
   不会再出现“这个字段叫 triggered_by 还是 source”的讨论，CloudEvents 已经帮你定好了，而且文档齐全。
2. 规则引擎只认几个固定字段
   匹配规则时，引擎只需要看 type、source、subject，其他全透传。处理逻辑极其单一。
3. 可观测性自然融入
   id、time、source 让你画出时间线、拓扑图时，所有信息已就绪，不用再去推断。
4. 未来扩展成本低
   如果某天你确实需要把事件发到外部系统（比如云端分析服务），只需加一个 CloudEvents 的 HTTP binding 适配层，内部事件结构不变。

—

落地建议：保持“文件即事件”

在你的场景里，事件载体仍然是文件。你可以规定：
当一个智能体想要发布事件时，就在约定位置写入这样一个 JSON 文件，比如：

```
/平台/docs/drd/.events/evt-20260516-001.json
```

规则引擎的文件监听器检测到这个新文件，解析 JSON，匹配规则，触发下游。
这样，事件格式标准化了，传递机制还是你原本的文件系统，零额外依赖。

—

总结

你的设计负担来自“不定形”。
用 CloudEvents 核心字段作为内部事件 Schema，就是用一个业界公认的“定形器”，一次性解决事件格式问题。然后你就可以把精力放回智能体本身的协作逻辑上。
