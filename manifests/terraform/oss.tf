# =============================================================================
# 密文数据存储（对齐 docs/dev-guide/model.md）
#
# 当前阶段（单团队、小数据量、纯 OSS、无 PG）：
#   - 每个密钥一个信封 JSON 对象：secrets/<id>.json
#   - 桶开启版本控制：误删/误写可回滚（删除产生 delete marker，覆盖写保留历史版本）
#   - SSE-OSS 服务端加密：第二层加密兜底（第一层为客户端 AES-256-GCM）
#     （SSE-OSS 由 OSS 托管密钥、自动轮换、免费；KMS 共享版已停服，专属密钥管理实例
#     价格 2499 元/月起，当前阶段不引入，见 README「安全说明」）
#   - 生命周期：清理非当前版本，防止版本无限膨胀
#   - ACL 私有：客户端永不直接接触 OSS，读写经 FC 代理（STS 最小权限，见 fc.tf）
# =============================================================================

# 主存储桶：密文信封对象 secrets/<id>.json
resource "alicloud_oss_bucket" "secrets" {
  bucket            = local.oss_bucket
  storage_class     = "Standard"
  resource_group_id = data.terraform_remote_state.platform.outputs.resource_group_id
  tags = {
    project     = var.project
    environment = var.environment
  }

  # 版本控制：恢复保险（误删/误写回滚任意历史版本）
  versioning {
    status = "Enabled"
  }

  # 服务端加密：SSE-OSS（第二层加密，OSS 托管密钥、自动轮换、免费）
  server_side_encryption_rule {
    sse_algorithm = "AES256"
  }

  # 生命周期：非当前版本保留 N 天后清理；删除标记过期后自动移除
  lifecycle_rule {
    id      = "version-cleanup"
    enabled = true

    noncurrent_version_expiration {
      days = var.oss_version_retention_days
    }

    expiration {
      expired_object_delete_marker = true
    }
  }
}

# 桶私有：客户端不直接接触 OSS，经 FC 代理访问（STS 临时凭证）
resource "alicloud_oss_bucket_acl" "secrets" {
  bucket = alicloud_oss_bucket.secrets.bucket
  acl    = "private"
}
