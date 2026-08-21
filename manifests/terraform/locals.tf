locals {
  # 应用级资源命名：<app>-<env>（系统级资源由 quanttide-platform 管理）
  app_name_prefix = "${var.project}-${var.environment}"

  # 数据桶：命名对齐站点规范 {repo}-{type}（如 qtdata-studio）；OSS 全局唯一，跨环境需自行区分
  oss_bucket = var.oss_bucket_name

  # FC 通过 OSS 公网 endpoint 访问（当前阶段无 RDS，不挂 VPC）
  oss_endpoint = "https://oss-${var.region}.aliyuncs.com"
}
