// Package session 会话数据访问（直读 DSH 数据目录，不依赖 CLI）。
package session

import (
	"encoding/json"
	"sort"
	"strings"
	"time"

	"github.com/quanttide/qtcloud-agent-provider/internal/store"
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
	st store.Store
}

// NewRepository 创建仓库（st 为数据源——LocalStore 指向 DSH 数据目录，如 ~/.dsh）
func NewRepository(st store.Store) *Repository {
	return &Repository{st: st}
}

// ListAll 读取全部会话（按工作区分组）
func (r *Repository) ListAll() ([]Group, error) {
	wsEntries, err := r.st.ListDir("sessions")
	if err != nil {
		return nil, err
	}

	cwds := r.projCacheCwds()
	var groups []Group
	for _, ws := range wsEntries {
		if !ws.IsDir {
			continue
		}
		name := ws.Name
		sessEntries, err := r.st.ListDir("sessions/" + name)
		if err != nil {
			continue
		}
		var items []Item
		var totalSize int64
		for _, se := range sessEntries {
			if !se.IsDir {
				continue
			}
			sid := se.Name
			item := Item{
				ID:        sid,
				SizeBytes: r.dirSize("sessions/" + name + "/" + sid),
				Cwd:       cwds[sid],
			}
			if !se.ModTime.IsZero() {
				t := se.ModTime
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

// projCacheCwds 读取 storages/session_projcache.json 的 /tables/sessions/{sid}/identity/cwd
func (r *Repository) projCacheCwds() map[string]string {
	raw, err := r.st.ReadFile("storages/session_projcache.json")
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

// dirSize 递归目录大小（通过数据源遍历）
func (r *Repository) dirSize(path string) int64 {
	var total int64
	entries, err := r.st.ListDir(path)
	if err != nil {
		return 0
	}
	for _, e := range entries {
		if e.IsDir {
			total += r.dirSize(path + "/" + e.Name)
		} else {
			total += e.Size
		}
	}
	return total
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
