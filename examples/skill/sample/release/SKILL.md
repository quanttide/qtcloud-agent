# 发布版本 Skill

## 用途
自动化发布流程：检查状态 → 更新 Changelog → 打标签 → 推送。

## 触发
用户说"发布"、"打版本"、"release"时自动激活。

## 执行步骤

### 1. 收集信息
```bash
git status
git diff HEAD
git log -n 10 --oneline
```
确认工作区干净，了解最近提交。

### 2. 确定版本号
根据 AGENTS.md 约定：
- **SemVer**: `v{major}.{minor}.{patch}`
- **阶段约定**:
  - Exploration（探索期）→ `0.0.x`
  - Validation（验证期）→ `0.x.y`
  - Release（正式版）→ `x.y.z`

如果用户未指定版本号，根据提交内容建议：
- 新增功能 → minor +1
- 仅修复 → patch +1
- 破坏性变更 → major +1

### 3. 更新 Changelog
编辑 `CHANGELOG.md`，在顶部添加新版本：
```markdown
## [0.2.0] - YYYY-MM-DD

### Features
- 新增 XXX
- 支持 YYY

### Fixes
- 修复 ZZZ
```

### 4. 提交并打标签
```bash
git add CHANGELOG.md
git commit -m "chore: release v{version}"
git tag -a "v{version}" -m "Release v{version}"
git push origin main --tags
```

### 5. 创建 GitHub Release
```bash
# 提取 CHANGELOG 中该版本的变更内容
CHANGELOG=$(sed -n "/## \[{version}\]/,/## \[/p" CHANGELOG.md | tail -n +2 | head -n -1)

gh release create "v{version}" \
  --title "Release v{version}" \
  --notes "$CHANGELOG" \
  --target main
```

## 子模块发布

子模块在自己的仓库独立发布，不需要在主仓库打版本。

```bash
# 在子模块中
cd apps/{module}
git status && git log -n 5 --oneline
# 更新子模块 CHANGELOG.md（如有）
git add .
git commit -m "chore: release v{version}"
git tag -a "v{version}" -m "Release v{version}"
git push origin main --tags

# 创建子模块 GitHub Release
CHANGELOG=$(cat CHANGELOG.md | sed -n "/## \[{version}\]/,/## \[/p" | tail -n +2 | head -n -1)
gh release create "v{version}" \
  --title "Release v{version}" \
  --notes "$CHANGELOG" \
  --target main
```

## 标签格式
- **主仓库**: `v{version}` (如 `v0.2.0`)
- **子模块**: `{module}/v{version}` (如 `qtcloud-asset/v0.1.0`)

## 注意事项
- 发布前确认测试通过、lint 无错误
- 主仓库和子模块分开打标签
- 标签推送到远程前需用户确认
- 如果发布失败，使用 `git tag -d` 删除本地标签，`git push --delete` 删除远程标签
