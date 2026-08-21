# 系统级资源引用：由 quanttide-platform 仓库管理（资源组；当前阶段无 VPC/RDS 需求）
data "terraform_remote_state" "platform" {
  backend = "oss"
  config = {
    bucket = "quanttide-terraform-state"
    key    = "quanttide-platform/terraform.tfstate"
    region = "cn-hangzhou"
  }
}
