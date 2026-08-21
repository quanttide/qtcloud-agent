package store

// OSSStore 阿里云 OSS 实现（预留）。
//
// 接入时机：多端共享（studio/provider 读写同一数据源）或远端备份。
// 实现要点：
//   - 引入 github.com/aliyun/aliyun-oss-go-sdk/oss
//   - ReadFile  → bucket.GetObject(path)
//   - WriteFile → bucket.PutObject(path, bytes.NewReader(data))
//   - ListDir   → bucket.ListObjectsV2(oss.Prefix(path+"/"))
//   - IsDir     → ListObjectsV2 前缀判断
//   - bucket 名/endpoint 从环境变量（ALIYUN_OSS_BUCKET / ALIYUN_OSS_ENDPOINT）读取
type OSSStore struct {
	// TODO: bucket *oss.Bucket
}

// NewOSSStore 创建 OSS 存储（预留——未实现）
func NewOSSStore(bucketName, endpoint, accessKeyID, accessKeySecret string) (*OSSStore, error) {
	panic("OSSStore 未实现——见 store.go 接入要点")
}
