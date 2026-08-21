# =============================================================================
# Studio CDN + DNS（agent.cloud.quanttide.com）
#
# 链路：OSS 静态网站桶（studio-bucket.tf，私有）→ CDN 加速 + 私有回源鉴权
#   → CNAME 接入（云解析）→ 用户浏览器
#
# 说明：
#   - 桶 ACL 私有（RAM 用户无权限设公共读），回源鉴权分两步：
#     ① 账号级授权（本文件 RAM 角色/策略，对齐阿里云官方文档命名）
#     ② 域名级开关：CDN 控制台「回源配置 → 阿里云OSS私有Bucket回源」开启，
#        回源类型选「同账号回源（STS）」——该开关无公开 OpenAPI，
#        且与 OSS 静态网站托管默认首页存在已知冲突，开启时按官方文档处理
#        （https://www.alibabacloud.com/help/zh/cdn/user-guide/grant-alibaba-cloud-cdn-access-permissions-on-private-oss-buckets）
#   - 证书：复用泛域名证书 *.quanttide.com（delib 已在使用）。
#     证书 ID 需在控制台/证书服务查询后填入 certificate_config；
#     未配置前域名仅 HTTP 可用
#   - 前置：quanttide.com 已完成 ICP 备案（delib.cloud.quanttide.com 已上线）
# =============================================================================

resource "alicloud_cdn_domain_new" "studio" {
  domain_name = "agent.cloud.quanttide.com"
  cdn_type    = "web"

  # 源站：OSS 静态网站桶（site 专用桶 qtcloud-agent-site——上传目标一致）
  sources {
    content  = "qtcloud-agent-site.oss-cn-hangzhou.aliyuncs.com"
    type     = "oss"
    port     = 80
    priority = 20
  }

  # HTTPS 证书：由 acme.sh 管理（Let's Encrypt 单域名证书
  # agent.cloud.quanttide.com，90 天自动续期；续期后 reloadcmd 自动重配 CDN），
  # terraform 不管理证书内容（避免私钥入库）——注意 *.quanttide.com 泛域名
  # 证书不匹配两层子域 agent.cloud.quanttide.com，须用单域名证书。
  # certificate_config {
  #   cert_type              = "upload"
  #   server_certificate     = "<PEM 公钥，acme.sh 签发>"
  #   private_key            = "<PEM 私钥>"
  #   server_certificate_status = "on"
  # }
}

# ── 私有 Bucket 回源开关（l2_oss_key：private_oss_auth=on，自动 STS 同账号回源） ──
# 说明：oss_auth 函数（FunctionID 10）由平台在配置 OSS 源站时自动添加（计费减免 +
# 私有回源鉴权支持），勿手动配置；此处仅开启私有回源开关。
# 注意：与 OSS 静态网站托管默认首页存在已知冲突，若回源 403 需按官方文档处理。
resource "alicloud_cdn_domain_config" "studio_private_back" {
  domain_name   = alicloud_cdn_domain_new.studio.domain_name
  function_name = "l2_oss_key"
  function_args {
    arg_name  = "private_oss_auth"
    arg_value = "on"
  }
}

# 根路径改写为 /index.html：私有回源（签名请求）与 OSS 静态网站托管
# 默认首页重写存在已知冲突（/ 回源 403），改为 CDN 侧直接回源 index.html
resource "alicloud_cdn_domain_config" "studio_root_rewrite" {
  domain_name   = alicloud_cdn_domain_new.studio.domain_name
  function_name = "back_to_origin_url_rewrite"
  function_args {
    arg_name  = "source_url"
    arg_value = "^/$"
  }
  function_args {
    arg_name  = "target_url"
    arg_value = "/index.html"
  }
  function_args {
    arg_name  = "flag"
    arg_value = "break"
  }
}

# 强制 HTTPS：HTTP 请求 301 跳转 HTTPS（浏览器密码框安全警告消除）
resource "alicloud_cdn_domain_config" "studio_https_force" {
  domain_name   = alicloud_cdn_domain_new.studio.domain_name
  function_name = "https_force"
  function_args {
    arg_name  = "enable"
    arg_value = "on"
  }
}

# ── 账号级授权：CDN 回源私有 OSS（阿里云官方命名，幂等） ─────────────

# 自定义策略：OSS 只读（List/Get）
resource "alicloud_ram_policy" "cdn_private_oss" {
  policy_name     = "AliyunCDNAccessingPrivateOSSRolePolicy"
  description     = "用于CDN/DCDN回源私有OSS Bucket角色的授权策略，包含OSS的只读权限"
  policy_document = <<-EOT
    {
      "Version": "1",
      "Statement": [
        { "Action": ["oss:List*", "oss:Get*"], "Resource": "*", "Effect": "Allow" }
      ]
    }
  EOT
}

# 角色：信任 CDN 服务（cdn.aliyuncs.com 可 AssumeRole）
resource "alicloud_ram_role" "cdn_private_oss" {
  name        = "AliyunCDNAccessingPrivateOSSRole"
  description = "用于CDN回源私有OSS Bucket"
  document    = <<-EOT
    {
      "Statement": [
        {
          "Action": "sts:AssumeRole",
          "Effect": "Allow",
          "Principal": { "Service": ["cdn.aliyuncs.com"] }
        }
      ],
      "Version": "1"
    }
  EOT
}

# 策略绑定到角色
resource "alicloud_ram_role_policy_attachment" "cdn_private_oss" {
  role_name   = alicloud_ram_role.cdn_private_oss.name
  policy_name = alicloud_ram_policy.cdn_private_oss.policy_name
  policy_type = "Custom"
}

# ── DNS：CNAME 接入 ─────────────────────────────────────────────────

resource "alicloud_alidns_record" "studio" {
  domain_name = "quanttide.com"
  rr          = "health"
  type        = "CNAME"
  value       = alicloud_cdn_domain_new.studio.cname
  ttl         = 600
}

# ── studio.agent.cloud.quanttide.com（Flutter Web 客户端） ──────────────────

resource "alicloud_cdn_domain_new" "studio_web" {
  domain_name = "studio.agent.cloud.quanttide.com"
  cdn_type    = "web"

  # 源站：OSS 静态网站桶（studio 专用桶 qtcloud-agent-studio——上传目标一致）
  sources {
    content  = "qtcloud-agent-studio.oss-cn-hangzhou.aliyuncs.com"
    type     = "oss"
    port     = 80
    priority = 20
  }
}

# 私有回源开关（同 site）
resource "alicloud_cdn_domain_config" "studio_web_private_back" {
  domain_name   = alicloud_cdn_domain_new.studio_web.domain_name
  function_name = "l2_oss_key"
  function_args {
    arg_name  = "private_oss_auth"
    arg_value = "on"
  }
}

# 根路径改写为 /index.html（同 site）
resource "alicloud_cdn_domain_config" "studio_web_root_rewrite" {
  domain_name   = alicloud_cdn_domain_new.studio_web.domain_name
  function_name = "back_to_origin_url_rewrite"
  function_args {
    arg_name  = "source_url"
    arg_value = "^/$"
  }
  function_args {
    arg_name  = "target_url"
    arg_value = "/index.html"
  }
  function_args {
    arg_name  = "flag"
    arg_value = "break"
  }
}

# SPA 回退改写：Flutter Web 采用 Path URL 策略（无 #），直接访问/刷新
# 子路由（如 /record、/assessments）时回源 OSS 会 404，统一改写为
# /index.html；真实产物（入口/引导/JS/清单/图标及 assets、icons、
# canvaskit 目录）保持原样回源。
resource "alicloud_cdn_domain_config" "studio_web_spa_fallback" {
  domain_name   = alicloud_cdn_domain_new.studio_web.domain_name
  function_name = "back_to_origin_url_rewrite"
  function_args {
    arg_name  = "source_url"
    arg_value = "^/(?!index\\.html$|main\\.dart\\.js$|flutter\\.js$|flutter_bootstrap\\.js$|flutter_service_worker\\.js$|manifest\\.json$|version\\.json$|favicon\\.png$|assets/|icons/|canvaskit/).*"
  }
  function_args {
    arg_name  = "target_url"
    arg_value = "/index.html"
  }
  function_args {
    arg_name  = "flag"
    arg_value = "break"
  }
}

# 强制 HTTPS
resource "alicloud_cdn_domain_config" "studio_web_https_force" {
  domain_name   = alicloud_cdn_domain_new.studio_web.domain_name
  function_name = "https_force"
  function_args {
    arg_name  = "enable"
    arg_value = "on"
  }
}

# DNS：studio CNAME 接入
resource "alicloud_alidns_record" "studio_web" {
  domain_name = "quanttide.com"
  rr          = "studio.health"
  type        = "CNAME"
  value       = alicloud_cdn_domain_new.studio_web.cname
  ttl         = 600
}
