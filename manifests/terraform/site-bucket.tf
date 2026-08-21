# Site 静态站点桶（agent.cloud.quanttide.com）
#
# 部署链路（.github/workflows/deploy-site.yml）：
#   site/* tag → Actions（terraform apply + npm build + ossutil cp）→ 本桶（静态网站模式）
#   → 阿里云 CDN（agent.cloud.quanttide.com）

resource "alicloud_oss_bucket" "site" {
  bucket            = "qtcloud-agent-site"
  storage_class     = "Standard"
  resource_group_id = data.terraform_remote_state.platform.outputs.resource_group_id
  tags = {
    project     = var.project
    environment = var.environment
  }

  # 静态网站托管：index.html 为入口
  website {
    index_document = "index.html"
    error_document = "index.html"
  }
}
