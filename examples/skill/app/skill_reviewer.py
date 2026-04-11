#!/usr/bin/env python3
"""Skill 审查器 — 检查 SKILL.md 的完整性、正确性和可执行性。

用法:
    python skill_reviewer.py <SKILL.md 路径>

审查维度:
    1. 结构完整性 — 必需的章节是否存在
    2. 步骤可执行性 — 代码块是否包含有效命令
    3. 变量一致性 — 模板变量是否定义和使用一致
    4. 安全性 — 是否有危险操作（rm -rf, force push 等）
    5. 可恢复性 — 是否有失败回退方案
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


# ── 审查规则 ─────────────────────────────────────────────

REQUIRED_SECTIONS = ["用途", "触发", "执行步骤"]
OPTIONAL_SECTIONS = ["注意事项", "子模块发布", "标签格式"]

DANGEROUS_PATTERNS = [
    (r"git\s+push\s+--force", "强制推送会覆盖远程历史"),
    (r"rm\s+-rf", "递归强制删除"),
    (r"git\s+reset\s+--hard", "硬重置会丢弃未提交更改"),
    (r"git\s+clean\s+-fd", "强制清理未跟踪文件"),
]

VARIABLE_PATTERN = re.compile(r"\{(\w+)\}")


# ── 数据结构 ─────────────────────────────────────────────


@dataclass
class Issue:
    severity: str  # error | warning | info
    rule: str
    message: str
    line: int | None = None


@dataclass
class ReviewResult:
    file: Path
    issues: list[Issue] = field(default_factory=list)
    sections_found: list[str] = field(default_factory=list)
    code_blocks: int = 0
    variables: dict[str, list[int]] = field(default_factory=dict)

    @property
    def errors(self) -> list[Issue]:
        return [i for i in self.issues if i.severity == "error"]

    @property
    def warnings(self) -> list[Issue]:
        return [i for i in self.issues if i.severity == "warning"]

    @property
    def passed(self) -> bool:
        return len(self.errors) == 0


# ── 代码块解析 ───────────────────────────────────────────


def parse_code_blocks(lines: list[str]) -> list[tuple[int, list[str]]]:
    """解析所有代码块，返回 [(起始行号, 内容行列表), ...]"""
    blocks: list[tuple[int, list[str]]] = []
    in_code_block = False
    current_lines: list[str] = []
    current_start = 0

    for i, line in enumerate(lines):
        if line.strip().startswith("```"):
            if in_code_block:
                blocks.append((current_start, current_lines))
                current_lines = []
                in_code_block = False
            else:
                in_code_block = True
                current_start = i + 1  # 1-based
        elif in_code_block:
            current_lines.append(line)

    return blocks


# ── 审查器 ──────────────────────────────────────────────


class SkillReviewer:
    def review(self, path: Path) -> ReviewResult:
        result = ReviewResult(file=path)

        if not path.exists():
            result.issues.append(
                Issue("error", "file_exists", f"文件不存在: {path}")
            )
            return result

        content = path.read_text(encoding="utf-8")
        lines = content.splitlines()

        self._check_sections(lines, result)
        self._check_code_blocks(lines, result)
        self._check_variables(lines, result)
        self._check_safety(lines, result)
        self._check_recoverability(content, result)

        return result

    def _check_sections(
        self, lines: list[str], result: ReviewResult
    ) -> None:
        # 只扫描非代码块内的标题
        in_code_block = False
        for line in lines:
            if line.strip().startswith("```"):
                in_code_block = not in_code_block
                continue
            if not in_code_block and line.startswith("## "):
                result.sections_found.append(line[3:].strip())

        for section in REQUIRED_SECTIONS:
            if section not in result.sections_found:
                result.issues.append(
                    Issue(
                        "error",
                        "missing_section",
                        f"缺少必需章节: {section}",
                    )
                )

    def _check_code_blocks(
        self, lines: list[str], result: ReviewResult
    ) -> None:
        blocks = parse_code_blocks(lines)
        result.code_blocks = len(blocks)

        if result.code_blocks == 0:
            result.issues.append(
                Issue("warning", "no_code_blocks", "没有可执行的代码块")
            )
            return

        for start, block_lines in blocks:
            # 检查是否为空代码块
            if not block_lines:
                result.issues.append(
                    Issue(
                        "warning",
                        "empty_code_block",
                        f"第 {start} 行: 代码块为空",
                        line=start,
                    )
                )
                continue

            # 跳过语言标识行
            code_lines = block_lines[1:] if block_lines else []
            has_command = any(
                line.strip() and not line.strip().startswith("#")
                for line in code_lines
            )
            if not has_command:
                result.issues.append(
                    Issue(
                        "warning",
                        "empty_code_block",
                        f"第 {start} 行: 代码块无有效命令",
                        line=start,
                    )
                )

    def _check_variables(
        self, lines: list[str], result: ReviewResult
    ) -> None:
        for line_num, line in enumerate(lines, 1):
            for match in VARIABLE_PATTERN.finditer(line):
                var_name = match.group(1)
                result.variables.setdefault(var_name, []).append(line_num)

        # 检查模板变量是否只有引用没有赋值
        for var, line_nums in result.variables.items():
            # 检查是否有 bash 变量赋值 (VAR=value 或 VAR = value)
            has_assignment = any(
                re.search(rf"\b{var}\s*=", lines[ln - 1])
                or re.search(rf"\$\{{{var}\}}", lines[ln - 1])
                for ln in line_nums
            )
            if not has_assignment and len(line_nums) > 2:
                result.issues.append(
                    Issue(
                        "warning",
                        "unassigned_variable",
                        f"变量 {{{var}}} 在 {len(line_nums)} 处使用但未赋值",
                    )
                )

    def _check_safety(
        self, lines: list[str], result: ReviewResult
    ) -> None:
        for line_num, line in enumerate(lines, 1):
            for pattern, desc in DANGEROUS_PATTERNS:
                if re.search(pattern, line):
                    result.issues.append(
                        Issue(
                            "warning",
                            "dangerous_command",
                            f"第 {line_num} 行: {desc}",
                            line=line_num,
                        )
                    )

    def _check_recoverability(
        self, content: str, result: ReviewResult
    ) -> None:
        has_failure_handling = any(
            kw in content.lower()
            for kw in ["回滚", "rollback", "恢复", "revert",
                       "删除标签", "回退", "失败", "failure"]
        )
        if not has_failure_handling:
            result.issues.append(
                Issue(
                    "warning",
                    "no_failure_handling",
                    "没有失败回退方案",
                )
            )


# ── 输出 ────────────────────────────────────────────────


def print_report(result: ReviewResult) -> None:
    icon = {"error": "✗", "warning": "⚠", "info": "ℹ"}

    print(f"\n{'=' * 60}")
    print(f"Skill 审查报告: {result.file}")
    print(f"{'=' * 60}")

    print(f"\n章节 ({len(result.sections_found)}): "
          f"{', '.join(result.sections_found)}")
    print(f"代码块: {result.code_blocks}")
    print(f"变量: {', '.join(f'{{{k}}}' for k in result.variables)}")

    if not result.issues:
        print(f"\n✅ 审查通过，无问题。")
        return

    print(f"\n发现问题: {len(result.errors)} 错误, "
          f"{len(result.warnings)} 警告\n")

    for issue in sorted(result.issues, key=lambda i: i.line or 9999):
        loc = f" 第 {issue.line} 行" if issue.line else ""
        print(f"  {icon.get(issue.severity, '?')} [{issue.severity}] "
              f"{issue.rule}{loc}: {issue.message}")

    print(f"\n{'=' * 60}")
    if result.passed:
        print("✅ 审查通过（有警告但无错误）")
    else:
        print(f"✗ 审查失败: {len(result.errors)} 个错误需要修复")
    print(f"{'=' * 60}\n")


# ── 入口 ────────────────────────────────────────────────


def main() -> int:
    if len(sys.argv) < 2:
        print("用法: python skill_reviewer.py <SKILL.md 路径>")
        return 1

    path = Path(sys.argv[1])
    reviewer = SkillReviewer()
    result = reviewer.review(path)
    print_report(result)

    return 0 if result.passed else 1


if __name__ == "__main__":
    sys.exit(main())
