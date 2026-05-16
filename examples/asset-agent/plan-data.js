var agentConfigs = {
  "drd-agent": {
    "rules": ["平台/docs/drd/*.md", "平台/docs/drd/.events/*.json"],
    "preview": [
      "\uD83D\uDCC4 平台/docs/drd/支付设计.md",
      "\uD83D\uDCC4 平台/docs/drd/架构决策.md",
      "\uD83D\uDCC4 平台/docs/drd/账户体系设计.md",
      "\uD83D\uDCC4 平台/docs/drd/.events/evt-001.json",
      "\uD83D\uDCC4 平台/docs/drd/.events/evt-002.json"
    ],
    "count": 10
  },
  "qa-agent": {
    "rules": ["平台/docs/qa/*.md", "平台/docs/qa/.events/*.json"],
    "preview": [
      "\uD83D\uDCC4 平台/docs/qa/QA-反馈-20260516.md",
      "\uD83D\uDCC4 平台/docs/qa/QA-报告-20260515.md",
      "\uD83D\uDCC4 平台/docs/qa/QA-检查清单.md",
      "\uD83D\uDCC4 平台/docs/qa/.events/evt-001.json",
      "\uD83D\uDCC4 平台/docs/qa/.events/evt-002.json"
    ],
    "count": 12
  },
  "code-agent": {
    "rules": ["平台/src/*.py", "平台/src/.events/*.json"],
    "preview": [
      "\uD83D\uDCC4 平台/src/main.py",
      "\uD83D\uDCC4 平台/src/payment.py",
      "\uD83D\uDCC4 平台/src/utils.py",
      "\uD83D\uDCC4 平台/src/.events/evt-001.json"
    ],
    "count": 8
  },
  "test-agent": {
    "rules": ["平台/test/*.py", "平台/test/.events/*.json"],
    "preview": [
      "\uD83D\uDCC4 平台/test/test_payment.py",
      "\uD83D\uDCC4 平台/test/test_account.py",
      "\uD83D\uDCC4 平台/test/.events/evt-001.json"
    ],
    "count": 6
  }
};
