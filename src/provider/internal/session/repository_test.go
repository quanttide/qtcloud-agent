package session

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func setupDSH(t *testing.T, home string) {
	t.Helper()
	// 工作区 --home-iguo-repos-qtfounder--（DSH 编码：/ → -，前后加 -）
	ws := filepath.Join(home, ".dsh", "sessions", "--home-iguo-repos-qtfounder--")
	s1 := filepath.Join(ws, "session-abc")
	s2 := filepath.Join(ws, "session-def")
	for _, d := range []string{s1, s2} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.WriteFile(filepath.Join(s1, "history.jsonl"), []byte("{}"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(s2, "history.jsonl"), []byte("{}"), 0o644); err != nil {
		t.Fatal(err)
	}
	// 临时工作区
	if err := os.MkdirAll(filepath.Join(home, ".dsh", "sessions", "--tmp--", "s-tmp"), 0o755); err != nil {
		t.Fatal(err)
	}
	// projcache：cwd 还原
	cache := filepath.Join(home, ".dsh", "storages")
	if err := os.MkdirAll(cache, 0o755); err != nil {
		t.Fatal(err)
	}
	cacheJSON := `{"tables":{"sessions":{"session-abc":{"identity":{"cwd":"/home/iguo/repos/qtfounder"}}}}}`
	if err := os.WriteFile(filepath.Join(cache, "session_projcache.json"), []byte(cacheJSON), 0o644); err != nil {
		t.Fatal(err)
	}
}

func TestListAll_GroupsAndCwd(t *testing.T) {
	home := t.TempDir()
	setupDSH(t, home)

	repo := NewRepository(filepath.Join(home, ".dsh"))
	groups, err := repo.ListAll()
	if err != nil {
		t.Fatalf("ListAll: %v", err)
	}
	if len(groups) != 2 {
		t.Fatalf("期望 2 个工作区，实际 %d", len(groups))
	}

	// --home-iguo-repos-qtfounder-- → home/iguo/repos/qtfounder（strip -- + '-'→'/'）
	g0 := groups[0]
	if g0.Decoded != "home/iguo/repos/qtfounder" {
		t.Errorf("解码 = %s, 期望 home/iguo/repos/qtfounder", g0.Decoded)
	}
	if g0.TotalSessions != 2 {
		t.Errorf("TotalSessions = %d, 期望 2", g0.TotalSessions)
	}
	// cwd 还原（session-abc）
	foundCwd := false
	for _, it := range g0.Items {
		if it.ID == "session-abc" && it.Cwd == "/home/iguo/repos/qtfounder" {
			foundCwd = true
		}
	}
	if !foundCwd {
		t.Error("session-abc 的 cwd 未还原")
	}
	// mtime 非空
	for _, it := range g0.Items {
		if it.Mtime == nil {
			t.Errorf("session %s 无 mtime", it.ID)
		}
	}

	// --tmp-- → 临时工作区
	for _, g := range groups {
		if g.Workspace == "--tmp--" && g.Decoded != "临时工作区" {
			t.Errorf("临时工作区解码 = %s", g.Decoded)
		}
	}
}

func TestListAll_Empty(t *testing.T) {
	repo := NewRepository(t.TempDir() + "/.dsh")
	groups, err := repo.ListAll()
	if err != nil {
		t.Fatalf("ListAll: %v", err)
	}
	if len(groups) != 0 {
		t.Errorf("期望空，实际 %d", len(groups))
	}
}

func TestDirSize(t *testing.T) {
	home := t.TempDir()
	if err := os.WriteFile(filepath.Join(home, "a.txt"), []byte("12345"), 0o644); err != nil {
		t.Fatal(err)
	}
	sub := filepath.Join(home, "sub")
	if err := os.MkdirAll(sub, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(sub, "b.txt"), []byte("12345"), 0o644); err != nil {
		t.Fatal(err)
	}
	if got := dirSize(home); got != 10 {
		t.Errorf("dirSize = %d, 期望 10", got)
	}
}

var _ = time.Time{}
