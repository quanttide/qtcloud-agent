// Package store 存储抽象：数据源统一接口。
//
// 本地文件系统实现（LocalStore）为默认数据源（第二大脑 git 仓库直读）；
// OSS 实现（OSSStore）预留——多端共享/远端备份时接入。
package store

import "time"

// Entry 目录条目
type Entry struct {
	Name    string
	IsDir   bool
	Size    int64
	ModTime time.Time
}

// Store 数据源接口（read/write/list，按相对路径操作）
type Store interface {
	// ReadFile 读取文件（路径不存在返回 os.ErrNotExist）
	ReadFile(path string) ([]byte, error)
	// WriteFile 写入文件（同 key 覆盖）
	WriteFile(path string, data []byte) error
	// ListDir 列出目录条目（目录不存在返回空列表）
	ListDir(path string) ([]Entry, error)
	// IsDir 判断路径是否为目录
	IsDir(path string) bool
}
