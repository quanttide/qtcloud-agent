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
