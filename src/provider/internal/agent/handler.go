package agent

import (
	"encoding/json"
	"net/http"
)

// Handler 智能体 HTTP 处理器
type Handler struct {
	repo *Repository
}

// NewHandler 创建处理器
func NewHandler(repo *Repository) *Handler {
	return &Handler{repo: repo}
}

// ListAll 处理 GET /api/agents
func (h *Handler) ListAll(w http.ResponseWriter, r *http.Request) {
	agents, err := h.repo.ListAll()
	if err != nil {
		http.Error(w, "注册中心加载失败："+err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"agents": agents})
}

func writeJSON(w http.ResponseWriter, code int, v any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(v)
}
