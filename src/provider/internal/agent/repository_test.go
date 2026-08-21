package agent

import (
	"github.com/quanttide/qtcloud-agent-provider/internal/store"
	"os"
	"path/filepath"
	"testing"
)

func setupRegistry(t *testing.T, home string) string {
	t.Helper()
	registry := filepath.Join(home, "data", "profile")
	agentDir := filepath.Join(registry, "agents", "zed")
	if err := os.MkdirAll(agentDir, 0o755); err != nil {
		t.Fatal(err)
	}
	// 注册配置（JSONC：行注释 + 尾逗号）
	regConfig := `{
  // zed 模型配置
  "model": "deepseek-v4-flash",
  "temperature": 0.7,
}
`
	if err := os.WriteFile(filepath.Join(agentDir, "settings.json"), []byte(regConfig), 0o644); err != nil {
		t.Fatal(err)
	}
	// 文档文件（应跳过）
	if err := os.WriteFile(filepath.Join(agentDir, "README.md"), []byte("# zed"), 0o644); err != nil {
		t.Fatal(err)
	}
	return registry
}

func TestListAll_OkAndMissing(t *testing.T) {
	home := t.TempDir()
	registry := setupRegistry(t, home)
	// 本地 zed 配置（内容一致）
	localDir := filepath.Join(home, ".config", "zed")
	if err := os.MkdirAll(localDir, 0o755); err != nil {
		t.Fatal(err)
	}
	localConfig := `{"model":"deepseek-v4-flash","temperature":0.7}`
	if err := os.WriteFile(filepath.Join(localDir, "settings.json"), []byte(localConfig), 0o644); err != nil {
		t.Fatal(err)
	}

	repo := NewRepository(store.NewLocalStore(registry))
	// 指向临时 HOME
	t.Setenv("HOME", home)

	agents, err := repo.ListAll()
	if err != nil {
		t.Fatalf("ListAll: %v", err)
	}
	if len(agents) != 1 {
		t.Fatalf("期望 1 个 agent，实际 %d", len(agents))
	}
	a := agents[0]
	if a.Name != "zed" {
		t.Errorf("agent 名 = %s, 期望 zed", a.Name)
	}
	if !a.LocalExists {
		t.Error("本地目录应存在")
	}
	// settings.json 应一致（JSONC 与标准 JSON 展平后相同）
	// README.md 应跳过
	byName := map[string]FileStatus{}
	for _, f := range a.Files {
		byName[f.Name] = f.Status
	}
	if byName["settings.json"] != StatusOK {
		t.Errorf("settings.json 状态 = %s, 期望 ok", byName["settings.json"])
	}
	if byName["README.md"] != StatusSkip {
		t.Errorf("README.md 状态 = %s, 期望 skip", byName["README.md"])
	}
	if a.IssueCount != 0 {
		t.Errorf("IssueCount = %d, 期望 0", a.IssueCount)
	}
}

func TestListAll_DiffAndMissing(t *testing.T) {
	home := t.TempDir()
	registry := setupRegistry(t, home)
	// 本地 zed 配置（内容不同 → diff）
	localDir := filepath.Join(home, ".config", "zed")
	if err := os.MkdirAll(localDir, 0o755); err != nil {
		t.Fatal(err)
	}
	localConfig := `{"model":"gpt-4o","temperature":0.2}`
	if err := os.WriteFile(filepath.Join(localDir, "settings.json"), []byte(localConfig), 0o644); err != nil {
		t.Fatal(err)
	}
	// 额外注册文件（本地缺失 → missing）
	agentDir := filepath.Join(registry, "agents", "zed")
	if err := os.WriteFile(filepath.Join(agentDir, "keybindings.json"), []byte(`{"copy":"ctrl+c"}`), 0o644); err != nil {
		t.Fatal(err)
	}

	repo := NewRepository(store.NewLocalStore(registry))
	t.Setenv("HOME", home)

	agents, err := repo.ListAll()
	if err != nil {
		t.Fatalf("ListAll: %v", err)
	}
	a := agents[0]
	byName := map[string]FileStatus{}
	for _, f := range a.Files {
		byName[f.Name] = f.Status
	}
	if byName["settings.json"] != StatusDiff {
		t.Errorf("settings.json 状态 = %s, 期望 diff", byName["settings.json"])
	}
	if byName["keybindings.json"] != StatusMissing {
		t.Errorf("keybindings.json 状态 = %s, 期望 missing", byName["keybindings.json"])
	}
	if a.IssueCount != 2 {
		t.Errorf("IssueCount = %d, 期望 2", a.IssueCount)
	}
}

func TestListAll_EmptyRegistry(t *testing.T) {
	repo := NewRepository(store.NewLocalStore(t.TempDir() + "/nonexistent"))
	agents, err := repo.ListAll()
	if err != nil {
		t.Fatalf("ListAll: %v", err)
	}
	if len(agents) != 0 {
		t.Errorf("期望空列表，实际 %d", len(agents))
	}
}
