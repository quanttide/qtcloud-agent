output "oss_bucket" {
  description = "数据桶名（预留——provider 部署用）"
  value       = alicloud_oss_bucket.secrets.bucket
}
