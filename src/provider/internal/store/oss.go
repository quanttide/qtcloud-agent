package store

import (
	"bytes"
	"errors"
	"io"
	"os"
	"path"
	"strings"
	"time"

	"github.com/aliyun/aliyun-oss-go-sdk/oss"
)

// OSSStore 阿里云 OSS 实现（云端数据源）。
//
// 目录语义（OSS 无目录概念）：以 "/" 为分隔模拟——
//   - ListDir(p)  = ListObjectsV2(Prefix=p+"/", Delimiter="/") → CommonPrefixes 为子目录
//   - IsDir(p)    = 前缀 p+"/" 下存在对象
//
// 凭证从环境变量读取（与 CI 一致）：
//
//	ALIYUN_OSS_BUCKET / ALIYUN_OSS_ENDPOINT / ALIYUN_ACCESS_KEY_ID / ALIYUN_ACCESS_KEY_SECRET
type OSSStore struct {
	bucket *oss.Bucket
}

// NewOSSStore 创建 OSS 存储（endpoint 如 oss-cn-hangzhou.aliyuncs.com）
func NewOSSStore(bucketName, endpoint, accessKeyID, accessKeySecret string) (*OSSStore, error) {
	if bucketName == "" || endpoint == "" || accessKeyID == "" || accessKeySecret == "" {
		return nil, errors.New("OSS 配置缺失（bucket/endpoint/access key）")
	}
	client, err := oss.New(endpoint, accessKeyID, accessKeySecret)
	if err != nil {
		return nil, err
	}
	bucket, err := client.Bucket(bucketName)
	if err != nil {
		return nil, err
	}
	return &OSSStore{bucket: bucket}, nil
}

// NewOSSStoreFromEnv 从环境变量创建 OSS 存储
func NewOSSStoreFromEnv() (*OSSStore, error) {
	return NewOSSStore(
		os.Getenv("ALIYUN_OSS_BUCKET"),
		os.Getenv("ALIYUN_OSS_ENDPOINT"),
		os.Getenv("ALIYUN_ACCESS_KEY_ID"),
		os.Getenv("ALIYUN_ACCESS_KEY_SECRET"),
	)
}

func (s *OSSStore) ReadFile(p string) ([]byte, error) {
	body, err := s.bucket.GetObject(p)
	if err != nil {
		if isNotFound(err) {
			return nil, os.ErrNotExist
		}
		return nil, err
	}
	defer body.Close()
	return io.ReadAll(body)
}

func (s *OSSStore) WriteFile(p string, data []byte) error {
	return s.bucket.PutObject(p, bytes.NewReader(data))
}

func (s *OSSStore) ListDir(p string) ([]Entry, error) {
	prefix := p
	if prefix != "" && !strings.HasSuffix(prefix, "/") {
		prefix += "/"
	}
	var result []Entry
	marker := ""
	for {
		res, err := s.bucket.ListObjectsV2(oss.Prefix(prefix), oss.Delimiter("/"), oss.ContinuationToken(marker))
		if err != nil {
			return nil, err
		}
		// 子目录（CommonPrefixes）
		for _, cp := range res.CommonPrefixes {
			name := strings.TrimSuffix(strings.TrimPrefix(cp, prefix), "/")
			result = append(result, Entry{Name: name, IsDir: true})
		}
		// 文件（Objects，不含前缀本身）
		for _, obj := range res.Objects {
			rel := strings.TrimPrefix(obj.Key, prefix)
			if rel == "" {
				continue
			}
			result = append(result, Entry{
				Name:    rel,
				IsDir:   false,
				Size:    obj.Size,
				ModTime: obj.LastModified,
			})
		}
		if !res.IsTruncated {
			break
		}
		marker = res.NextContinuationToken
	}
	return result, nil
}

func (s *OSSStore) IsDir(p string) bool {
	prefix := p
	if !strings.HasSuffix(prefix, "/") {
		prefix += "/"
	}
	res, err := s.bucket.ListObjectsV2(oss.Prefix(prefix), oss.MaxKeys(1))
	if err != nil {
		return false
	}
	return len(res.Objects) > 0 || len(res.CommonPrefixes) > 0
}

func isNotFound(err error) bool {
	if se, ok := err.(oss.ServiceError); ok {
		return se.StatusCode == 404
	}
	return false
}

var _ = path.Join
var _ = time.Time{}
