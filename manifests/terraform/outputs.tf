output "oss_bucket" {
  description = "密文数据桶名（secrets/<id>.json，版本控制 + SSE-OSS）"
  value       = alicloud_oss_bucket.secrets.bucket
}

output "fc_function_name" {
  description = "函数计算函数名"
  value       = alicloud_fcv3_function.this.function_name
}

output "fc_http_url" {
  description = "FC HTTP 触发器公网地址（系统级 API 网关接入前的直连入口）"
  value       = try(alicloud_fcv3_trigger.http.http_trigger[0].url_internet, "尚未创建")
}
