# Skill 审查器设计文档

## 为什么需要 Skill 审查

Skill（技能）是 Agent 的执行指南，质量直接影响 Agent 行为。一个低质量的 Skill 会导致：

1. **执行失败** — 步骤不完整或命令错误，Agent 无法完成任务
2. **安全隐患** — 包含危险操作（`rm -rf`、`force push`），可能破坏仓库
3. **不可恢复** — 没有失败回退方案，一旦出错只能人工介入
4. **变量不一致** — 模板变量未定义或命名混乱，执行时替换失败

Skill 审查器的定位是 **Skill 的 linter**，在 Skill 投入使用前自动发现问题。

## 设计原则

### 1. 静态分析，不执行
审查器只分析 SKILL.md 的文本结构，不实际执行其中的命令。原因：
- 执行成本高（需要 git、gh CLI 等环境）
- 安全风险（Skill 可能包含破坏性命令）
- 静态分析已能覆盖大部分问题

### 2. 规则可扩展
每个审查维度是独立的方法，规则定义在模块顶部常量中，易于增删：
```python
REQUIRED_SECTIONS = ["用途", "触发", "执行步骤"]
DANGEROUS_PATTERNS = [
    (r"git\s+push\s+--force", "强制推送会覆盖远程历史"),
    ...
]
```

### 3. 分级报告
问题分三级：
- **error** — 必须修复，否则 Skill 不可用（如缺少必需章节）
- **warning** — 建议修复，不影响基本使用（如无失败回退）
- **info** — 提示信息（预留）

## 审查维度

### 1. 结构完整性（_check_sections）
**规则**: 必须包含"用途"、"触发"、"执行步骤"三个章节。

**逻辑**: 扫描 Markdown 二级标题（`## `），但要跳过代码块内的标题。

**为什么跳过代码块**: Skill 示例代码中可能包含 Markdown 片段，其中的 `## ` 不应被误识别为章节。

### 2. 代码块可执行性（_check_code_blocks）
**规则**: 每个代码块应包含可执行命令，不能为空或只有注释。

**逻辑**: 
- 解析所有代码块（` ``` ` 到 ` ``` ` 之间的内容）
- 跳过第一行（语言标识如 `bash`）
- 检查剩余行是否有非注释内容

**为什么跳过语言标识**: ````bash` 中的 `bash` 是语言标识，不是可执行命令。

### 3. 变量一致性（_check_variables）
**规则**: 模板变量 `{var}` 如果被多次使用，应该有赋值定义。

**逻辑**:
- 正则提取所有 `{变量名}` 模式
- 检查是否有 bash 赋值语法（`VAR=value` 或 `$ {VAR}`）
- 只在使用超过 2 次时才告警（避免对单次使用的模板变量误报）

**为什么设阈值**: Skill 中 `{version}` 等变量通常在外部由用户传入，不需要内部赋值，过度告警会降低信任度。

### 4. 安全性（_check_safety）
**规则**: 检测危险操作并警告。

**当前规则**:
| 模式 | 风险 |
|---|---|
| `git push --force` | 覆盖远程历史 |
| `rm -rf` | 递归强制删除 |
| `git reset --hard` | 丢弃未提交更改 |
| `git clean -fd` | 强制清理未跟踪文件 |

**为什么是 warning 不是 error**: 有些 Skill 确实需要这些操作（如发布 Skill 需要 push），只需提醒审查者确认。

### 5. 可恢复性（_check_recoverability）
**规则**: Skill 应包含失败回退方案。

**逻辑**: 检查是否包含关键词（回滚/rollback/恢复/revert/删除标签/失败/failure）。

**为什么用关键词而非结构化分析**: 回退方案的表达方式多样（文本描述或代码），关键词匹配最简单且覆盖率高。

## 数据结构

```
Issue                    # 单个问题
├── severity: str        # error | warning | info
├── rule: str            # 规则 ID（如 missing_section）
├── message: str         # 人类可读描述
└── line: int | None     # 所在行号（可选）

ReviewResult             # 审查结果
├── file: Path           # 被审查文件
├── issues: list[Issue]  # 问题列表
├── sections_found       # 发现的章节
├── code_blocks: int     # 代码块数量
├── variables: dict      # 变量使用位置
├── errors               # 只返回 error 级问题
├── warnings             # 只返回 warning 级问题
└── passed               # 是否有 error（布尔值）
```

## 执行流程

```
review(path)
├── _check_sections      → 章节完整性
├── _check_code_blocks   → 代码块可执行性
├── _check_variables     → 变量一致性
├── _check_safety        → 安全检测
├── _check_recoverability → 可恢复性
└── 返回 ReviewResult
```

每个维度独立执行，互不依赖。所有问题汇总到同一个 `ReviewResult`。

## 扩展方式

### 添加新规则
1. 在 `REQUIRED_SECTIONS` 或 `OPTIONAL_SECTIONS` 中添加章节名
2. 在 `DANGEROUS_PATTERNS` 中添加正则+描述
3. 添加新的 `_check_xxx` 方法并在 `review()` 中调用
4. 在 `print_report` 中添加新的输出格式（如果需要）

### 集成到 CI
```yaml
# GitHub Actions 示例
- name: Review Skills
  run: |
    for skill in apps/*/examples/skill/sample/*/SKILL.md; do
      python3 apps/qtcloud-agent/examples/skill/app/skill_reviewer.py "$skill"
    done
```
