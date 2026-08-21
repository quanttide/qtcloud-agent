// qtcloud-agent provider：量潮智能体云服务端（注册中心 + 会话 API）
//
// 数据源兼容本地与云端（QTCLOUD_AGENT_STORE）：
//
//	local（默认）— 直读本地文件系统（第二大脑 git 仓库 / ~/.dsh），本地测试友好
//	oss          — 阿里云 OSS（多端共享），凭证见 store.NewOSSStoreFromEnv
//
// 数据路径：注册中心（QTCLOUD_AGENT_REGISTRY，默认 data/profile）+ DSH（DSH_HOME，默认 ~/.dsh）
package main

import (
	"log"
	"net/http"
	"os"

	"github.com/quanttide/qtcloud-agent-provider/internal/agent"
	"github.com/quanttide/qtcloud-agent-provider/internal/session"
	"github.com/quanttide/qtcloud-agent-provider/internal/store"
)

func main() {
	registry := os.Getenv("QTCLOUD_AGENT_REGISTRY")
	dshHome := os.Getenv("DSH_HOME")
	if registry == "" {
		registry = "data/profile"
	}
	if dshHome == "" {
		if h, err := os.UserHomeDir(); err == nil {
			dshHome = h + "/.dsh"
		} else {
			dshHome = ".dsh"
		}
	}

	var agentStore store.Store
	var sessionStore store.Store
	switch os.Getenv("QTCLOUD_AGENT_STORE") {
	case "oss":
		ossStore, err := store.NewOSSStoreFromEnv()
		if err != nil {
			log.Fatalf("OSS 数据源初始化失败：%v", err)
		}
		agentStore = ossStore
		sessionStore = ossStore
		log.Printf("数据源：OSS（bucket=%s）", os.Getenv("ALIYUN_OSS_BUCKET"))
	default:
		agentStore = store.NewLocalStore(registry)
		sessionStore = store.NewLocalStore(dshHome)
		log.Printf("数据源：本地（registry=%s, dsh=%s）", registry, dshHome)
	}

	agentRepo := agent.NewRepository(agentStore)
	sessionRepo := session.NewRepository(sessionStore)
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
	log.Printf("qtcloud-agent provider listening on %s", addr)
	if err := http.ListenAndServe(addr, mux); err != nil {
		log.Fatal(err)
	}
}
