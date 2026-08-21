/// 智能体注册中心模型。
library;

/// 单个注册文件的状态
enum AgentFileStatus { ok, diff, missing, skip, unknown }

/// 注册文件条目
class AgentFile {
  const AgentFile({required this.name, required this.status});

  final String name;
  final AgentFileStatus status;

  String get statusLabel => switch (status) {
        AgentFileStatus.ok => '一致',
        AgentFileStatus.diff => '差异',
        AgentFileStatus.missing => '本地缺失',
        AgentFileStatus.skip => '文档',
        AgentFileStatus.unknown => '未知',
      };
}

/// 单个 Agent 的状态（注册中心 vs 本地）
class AgentStatus {
  const AgentStatus({
    required this.name,
    required this.localDir,
    required this.localExists,
    required this.files,
  });

  final String name;
  final String? localDir;
  final bool localExists;
  final List<AgentFile> files;

  int get issueCount => files
      .where((f) =>
          f.status == AgentFileStatus.diff ||
          f.status == AgentFileStatus.missing)
      .length;
}
