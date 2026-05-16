var auditData = [
  {
    "time": "14:30:05",
    "agent": "drd-agent",
    "tokens": 8230,
    "trigger": "平台/docs/drd/支付设计.md (finalized)",
    "files": [
      { "icon": "\uD83D\uDCC4", "name": "支付设计.md", "tokens": 3200, "status": "ok" },
      { "icon": "\uD83D\uDCC4", "name": "架构决策.md", "tokens": 1800, "status": "ok" },
      { "icon": "\uD83D\uDCC4", "name": "drd-rules.yaml", "tokens": 930, "status": "ok" },
      { "icon": "\uD83D\uDCC4", "name": "平台技术选型.md", "tokens": 2300, "status": "waste" }
    ],
    "eventData": "2,300",
    "boundary": "✅ 未跨域",
    "json": "{\n  \"id\": \"evt-001\",\n  \"type\": \"drd.finalized\",\n  \"source\": \"/平台/docs/drd\",\n  \"subject\": \"支付设计.md\",\n  \"data\": {\n    \"status\": \"final\",\n    \"summary\": \"支付服务采用策略模式，支持微信/支付宝/银联\",\n    \"key_decisions\": [\"策略模式\", \"异步对账\", \"T+1 结算\"]\n  }\n}",
    "wasteRate": 38
  },
  {
    "time": "14:28:12",
    "agent": "qa-agent",
    "tokens": 12450,
    "trigger": "事件 data (design.change.request)",
    "files": [
      { "icon": "\uD83D\uDCC4", "name": "QA-反馈.md", "tokens": 2100, "status": "ok" },
      { "icon": "\uD83D\uDCE8", "name": "事件 data (issue+suggestion)", "tokens": 1400, "status": "ok" },
      { "icon": "\uD83D\uDCC4", "name": "支付设计.md", "tokens": 4200, "status": "waste" },
      { "icon": "\uD83D\uDCC4", "name": "账户体系设计.md", "tokens": 3800, "status": "waste" }
    ],
    "eventData": "1,400",
    "boundary": "🔴 越权: 尝试加载平台/src/main.py 被拒绝",
    "json": "{\n  \"id\": \"evt-002\",\n  \"type\": \"design.change.request\",\n  \"source\": \"/平台/docs/qa\",\n  \"subject\": \"QA-反馈.md\",\n  \"data\": {\n    \"issue\": \"支付设计的对账时序图缺少失败分支\",\n    \"severity\": \"major\",\n    \"suggestion\": \"补充异常重试和最终一致性说明\",\n    \"related_doc\": \"/平台/docs/drd/支付设计.md\"\n  }\n}",
    "wasteRate": 67,
    "warn": true,
    "violation": true
  },
  {
    "time": "14:15:33",
    "agent": "drd-agent",
    "tokens": 5620,
    "trigger": "平台/docs/drd/架构决策.md (created)",
    "files": [
      { "icon": "\uD83D\uDCC4", "name": "架构决策.md", "tokens": 2800, "status": "ok" },
      { "icon": "\uD83D\uDCC4", "name": "drd-template.md", "tokens": 1500, "status": "low" },
      { "icon": "\uD83D\uDCC4", "name": "drd-rules.yaml", "tokens": 820, "status": "ok" }
    ],
    "eventData": "1,200",
    "boundary": "✅ 未跨域",
    "json": "{\n  \"id\": \"evt-003\",\n  \"type\": \"drd.created\",\n  \"source\": \"/平台/docs/drd\",\n  \"subject\": \"架构决策.md\",\n  \"data\": {\n    \"status\": \"draft\",\n    \"summary\": \"支付服务决策记录\"\n  }\n}",
    "wasteRate": 27
  }
];
