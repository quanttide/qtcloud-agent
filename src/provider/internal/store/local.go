package store

import (
	"os"
	"path/filepath"
)

// LocalStore 本地文件系统实现（默认数据源）
type LocalStore struct {
	root string
}

// NewLocalStore 创建本地存储（root 为数据根目录）
func NewLocalStore(root string) *LocalStore {
	return &LocalStore{root: root}
}

// Root 返回数据根目录
func (s *LocalStore) Root() string { return s.root }

func (s *LocalStore) resolve(path string) string {
	return filepath.Join(s.root, path)
}

func (s *LocalStore) ReadFile(path string) ([]byte, error) {
	return os.ReadFile(s.resolve(path))
}

func (s *LocalStore) WriteFile(path string, data []byte) error {
	full := s.resolve(path)
	if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
		return err
	}
	return os.WriteFile(full, data, 0o644)
}

func (s *LocalStore) ListDir(path string) ([]Entry, error) {
	entries, err := os.ReadDir(s.resolve(path))
	if err != nil {
		if os.IsNotExist(err) {
			return []Entry{}, nil
		}
		return nil, err
	}
	result := make([]Entry, 0, len(entries))
	for _, e := range entries {
		info, err := e.Info()
		if err != nil {
			continue
		}
		result = append(result, Entry{
			Name:    e.Name(),
			IsDir:   e.IsDir(),
			Size:    info.Size(),
			ModTime: info.ModTime(),
		})
	}
	return result, nil
}

func (s *LocalStore) IsDir(path string) bool {
	fi, err := os.Stat(s.resolve(path))
	return err == nil && fi.IsDir()
}
