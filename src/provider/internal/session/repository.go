// Package session 会话数据访问（直读 DSH 数据目录，不依赖 CLI）。
package session

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// Item 单个会话条目
type Item struct {
	ID        string     `json:"id"`
	SizeBytes int64      `json:"sizeBytes"`
	Mtime     *time.Time `json:"mtime,omitempty"`
	Cwd       string     `json:"cwd,omitempty"`
}

// Group 工作区会话组
type Group struct {
	Workspace     string `json:"workspace"`
	Decoded       string `json:"decoded"`
	TotalSessions int    `json:"totalSessions"`
	TotalSize     int64  `json:"totalSize"`
	Items         []Item `json:"items"`
}

// Repository 会话数据访问
type Repository struct {
	dshHome string
}

// NewRepository 创建仓库（dshHome 默认 ~/.dsh）
func NewRepository(dshHome string) *Repository {
	if dshHome == "" {
		if h, err := os.UserHomeDir(); err == nil {
			dshHome = filepath.Join(h, ".dsh")
		} else {
			dshHome = ".dsh"
		}
	}
	return &Repository{dshHome: dshHome}
}

// DSHHome 返回数据目录
func (r *Repository) DSHHome() string { return r.dshHome }

// ListAll 读取全部会话（按工作区分组）
func (r *Repository) ListAll() ([]Group, error) {
	sessionsDir := filepath.Join(r.dshHome, "sessions")
	wsEntries, err := os.ReadDir(sessionsDir)
	if err != nil {
		if os.IsNotExist(err) {
			return []Group{}, nil
		}
		return nil, err
	}

	cwds := r.projCacheCwds()
	var groups []Group
	for _, ws := range wsEntries {
		if !ws.IsDir() {
			continue
		}
		name := ws.Name()
		wsPath := filepath.Join(sessionsDir, name)
		sessEntries, err := os.ReadDir(wsPath)
		if err != nil {
			continue
		}
		var items []Item
		var totalSize int64
		for _, se := range sessEntries {
			if !se.IsDir() {
				continue
			}
			sid := se.Name()
			item := Item{
				ID:        sid,
				SizeBytes: dirSize(filepath.Join(wsPath, sid)),
				Cwd:       cwds[sid],
			}
			if fi, err := os.Stat(filepath.Join(wsPath, sid)); err == nil {
				t := fi.ModTime()
				item.Mtime = &t
			}
			items = append(items, item)
			totalSize += item.SizeBytes
		}
		sort.Slice(items, func(i, j int) bool { return items[i].ID < items[j].ID })
		groups = append(groups, Group{
			Workspace:     name,
			Decoded:       decodeWorkspace(name),
			TotalSessions: len(items),
			TotalSize:     totalSize,
			Items:         items,
		})
	}
	sort.Slice(groups, func(i, j int) bool { return groups[i].Workspace < groups[j].Workspace })
	return groups, nil
}

// projCacheCwds 读取 $DSH_HOME/storages/session_projcache.json 的 /tables/sessions/{sid}/identity/cwd
func (r *Repository) projCacheCwds() map[string]string {
	p := filepath.Join(r.dshHome, "storages", "session_projcache.json")
	raw, err := os.ReadFile(p)
	if err != nil {
		return map[string]string{}
	}
	var data map[string]any
	if err := json.Unmarshal(raw, &data); err != nil {
		return map[string]string{}
	}
	tables, _ := data["tables"].(map[string]any)
	sessions, _ := tables["sessions"].(map[string]any)
	out := map[string]string{}
	for sid, val := range sessions {
		obj, ok := val.(map[string]any)
		if !ok {
			continue
		}
		identity, _ := obj["identity"].(map[string]any)
		if cwd, ok := identity["cwd"].(string); ok {
			out[sid] = cwd
		}
	}
	return out
}

// decodeWorkspace 工作区目录名解码（对齐 CLI）：--tmp-- → 临时工作区；--path-- → 还原
func decodeWorkspace(name string) string {
	if name == "--tmp--" {
		return "临时工作区"
	}
	inner := name
	if strings.HasPrefix(name, "--") && strings.HasSuffix(name, "--") {
		inner = name[2 : len(name)-2]
	}
	if inner == "" {
		return name
	}
	return strings.ReplaceAll(inner, "-", "/")
}

// dirSize 递归目录大小
func dirSize(path string) int64 {
	var total int64
	entries, err := os.ReadDir(path)
	if err != nil {
		return 0
	}
	for _, e := range entries {
		p := filepath.Join(path, e.Name())
		if e.IsDir() {
			total += dirSize(p)
		} else if fi, err := e.Info(); err == nil {
			total += fi.Size()
		}
	}
	return total
}
