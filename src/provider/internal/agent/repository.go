// Package agent 智能体注册中心数据访问（直读文件系统，不依赖 CLI）。
package agent

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/quanttide/qtcloud-agent-provider/internal/store"
)

// FileStatus 注册文件状态
type FileStatus string

const (
	StatusOK      FileStatus = "ok"
	StatusDiff    FileStatus = "diff"
	StatusMissing FileStatus = "missing"
	StatusSkip    FileStatus = "skip"
	StatusUnknown FileStatus = "unknown"
)

// File 注册文件条目
type File struct {
	Name   string     `json:"name"`
	Status FileStatus `json:"status"`
}

// Status 单个 Agent 的状态（注册中心 vs 本地）
type Status struct {
	Name        string `json:"name"`
	LocalDir    string `json:"localDir,omitempty"`
	LocalExists bool   `json:"localExists"`
	Files       []File `json:"files"`
	IssueCount  int    `json:"issueCount"`
}

// Repository 注册中心数据访问
type Repository struct {
	st store.Store
}

// NewRepository 创建仓库（st 为数据源——LocalStore 指向注册中心父目录，如 data/profile）
func NewRepository(st store.Store) *Repository {
	return &Repository{st: st}
}

// agentLocalDir 已知 Agent 本地配置目录映射（对齐 CLI agent_local_dir）
func agentLocalDir(name string) string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	switch name {
	case "zed":
		return filepath.Join(home, ".config/zed")
	case "opencode":
		return filepath.Join(home, ".config/opencode")
	case "hermes":
		return filepath.Join(home, ".config/hermes")
	}
	return ""
}

func skipFile(name string) bool {
	return name == "README.md" || name == "LICENSE" || name == ".gitkeep"
}

// ListAll 读取注册中心下所有 Agent 的状态
func (r *Repository) ListAll() ([]Status, error) {
	entries, err := r.st.ListDir("agents")
	if err != nil {
		return nil, err
	}

	var result []Status
	for _, e := range entries {
		if !e.IsDir {
			continue
		}
		s, err := r.loadAgent(e.Name)
		if err != nil {
			return nil, err
		}
		result = append(result, s)
	}
	sort.Slice(result, func(i, j int) bool { return result[i].Name < result[j].Name })
	return result, nil
}

func (r *Repository) loadAgent(name string) (Status, error) {
	localDir := agentLocalDir(name)
	localExists := localDir != "" && isDir(localDir)

	entries, err := r.st.ListDir("agents/" + name)
	if err != nil {
		return Status{}, err
	}

	var files []File
	issueCount := 0
	for _, e := range entries {
		if e.IsDir {
			continue
		}
		fname := e.Name
		if skipFile(fname) {
			files = append(files, File{Name: fname, Status: StatusSkip})
			continue
		}
		if localDir == "" {
			files = append(files, File{Name: fname, Status: StatusUnknown})
			continue
		}
		localPath := filepath.Join(localDir, fname)
		if !isFile(localPath) {
			files = append(files, File{Name: fname, Status: StatusMissing})
			issueCount++
			continue
		}
		status := StatusDiff
		if sameKeys(r.flattenJSON("agents/"+name+"/"+fname), flattenLocalJSON(localPath)) {
			status = StatusOK
		} else {
			issueCount++
		}
		files = append(files, File{Name: fname, Status: status})
	}

	return Status{
		Name:        name,
		LocalDir:    localDir,
		LocalExists: localExists,
		Files:       files,
		IssueCount:  issueCount,
	}, nil
}

// flattenJSON 从数据源读取 JSON(JSONC) 文件并展平
func (r *Repository) flattenJSON(path string) map[string]string {
	raw, err := r.st.ReadFile(path)
	if err != nil {
		return map[string]string{}
	}
	return flattenRaw(raw)
}

// flattenLocalJSON 读取本地配置文件并展平（对比目标）
func flattenLocalJSON(path string) map[string]string {
	raw, err := os.ReadFile(path)
	if err != nil {
		return map[string]string{}
	}
	return flattenRaw(raw)
}

func flattenRaw(raw []byte) map[string]string {
	cleaned := stripJSONC(string(raw))
	var value any
	if err := json.Unmarshal([]byte(cleaned), &value); err != nil {
		return map[string]string{}
	}
	out := map[string]string{}
	flatten(value, "", out)
	return out
}

func flatten(v any, prefix string, out map[string]string) {
	switch t := v.(type) {
	case map[string]any:
		keys := make([]string, 0, len(t))
		for k := range t {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			key := k
			if prefix != "" {
				key = prefix + "." + k
			}
			flatten(t[k], key, out)
		}
	case []any:
		for i, item := range t {
			flatten(item, prefix+"["+strconv.Itoa(i)+"]", out)
		}
	default:
		out[prefix] = toStr(v)
	}
}

func sameKeys(a, b map[string]string) bool {
	if len(a) != len(b) {
		return false
	}
	for k, v := range a {
		if b[k] != v {
			return false
		}
	}
	return true
}

// stripJSONC 去除行注释（//）与尾逗号
func stripJSONC(raw string) string {
	lines := strings.Split(raw, "\n")
	var cleaned []string
	for _, line := range lines {
		if idx := strings.Index(line, "//"); idx >= 0 {
			line = line[:idx]
		}
		cleaned = append(cleaned, line)
	}
	out := strings.Join(cleaned, "\n")
	for {
		replaced := strings.ReplaceAll(out, ",\n}", "\n}")
		replaced = strings.ReplaceAll(replaced, ",\n]", "\n]")
		if replaced == out {
			break
		}
		out = replaced
	}
	return out
}

func isDir(p string) bool {
	fi, err := os.Stat(p)
	return err == nil && fi.IsDir()
}

func isFile(p string) bool {
	fi, err := os.Stat(p)
	return err == nil && !fi.IsDir()
}

func toStr(v any) string {
	switch t := v.(type) {
	case string:
		return t
	case nil:
		return "null"
	case bool:
		return strconv.FormatBool(t)
	case float64:
		return strconv.FormatFloat(t, 'f', -1, 64)
	case json.Number:
		return t.String()
	default:
		b, _ := json.Marshal(v)
		return string(b)
	}
}
