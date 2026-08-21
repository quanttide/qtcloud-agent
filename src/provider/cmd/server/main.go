// qtcloud-agent provider：量潮智能体云服务端（注册中心 + 会话 API）
// 数据源：注册中心（QTCLOUD_AGENT_REGISTRY，默认 data/profile）+ DSH 数据目录（DSH_HOME，默认 ~/.dsh）
package main

import (
	"log"
	"net/http"
	"os"

	"github.com/quanttide/qtcloud-agent-provider/internal/agent"
	"github.com/quanttide/qtcloud-agent-provider/internal/session"
)

func main() {
	registry := os.Getenv("QTCLOUD_AGENT_REGISTRY")
	dshHome := os.Getenv("DSH_HOME")

	agentRepo := agent.NewRepository(registry)
	sessionRepo := session.NewRepository(dshHome)
	agentH := agent.NewHandler(agentRepo)
	sessionH := session.NewHandler(sessionRepo)

	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/agents", agentH.ListAll)
	mux.HandleFunc("GET /api/sessions", sessionH.ListAll)
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"status":"ok"}`))
	})

	addr := os.Getenv("QTCLOUD_AGENT_ADDR")
	if addr == "" {
		addr = ":8080"
	}
	log.Printf("qtcloud-agent provider listening on %s (registry=%s, dsh=%s)", addr, agentRepo.RegistryPath(), sessionRepo.DSHHome())
	if err := http.ListenAndServe(addr, mux); err != nil {
		log.Fatal(err)
	}
}
