package session

import (
	"encoding/json"
	"net/http"
)

// Handler 会话 HTTP 处理器
type Handler struct {
	repo *Repository
}

// NewHandler 创建处理器
func NewHandler(repo *Repository) *Handler {
	return &Handler{repo: repo}
}

// ListAll 处理 GET /api/sessions
func (h *Handler) ListAll(w http.ResponseWriter, r *http.Request) {
	groups, err := h.repo.ListAll()
	if err != nil {
		http.Error(w, "会话加载失败："+err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"groups": groups})
}

func writeJSON(w http.ResponseWriter, code int, v any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(v)
}
