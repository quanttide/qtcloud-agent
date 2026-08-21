/// 会话模型。
library;

/// 单个会话条目
class SessionItem {
  const SessionItem({
    required this.id,
    required this.sizeBytes,
    required this.mtime,
    required this.cwd,
  });

  final String id;
  final int sizeBytes;
  final DateTime? mtime;
  final String? cwd;

  String get sizeLabel {
    if (sizeBytes < 1024) return '$sizeBytes B';
    if (sizeBytes < 1024 * 1024) return '${(sizeBytes / 1024).toStringAsFixed(1)} KB';
    return '${(sizeBytes / 1024 / 1024).toStringAsFixed(1)} MB';
  }

  String get mtimeLabel =>
      mtime == null ? '-' : mtime!.toLocal().toString().substring(0, 16);
}

/// 工作区会话组
class SessionGroup {
  const SessionGroup({
    required this.workspace,
    required this.decoded,
    required this.items,
  });

  final String workspace;
  final String decoded;
  final List<SessionItem> items;

  int get totalSessions => items.length;
  int get totalSize => items.fold(0, (sum, i) => sum + i.sizeBytes);
}
