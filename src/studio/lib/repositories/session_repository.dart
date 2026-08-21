/// 会话数据访问（独立管理数据——直读 DSH 数据目录，不依赖 CLI）。
///
/// 数据源：`$DSH_HOME/sessions/`（默认 ~/.dsh，可用环境变量 `DSH_HOME` 覆盖），
/// 按工作区分组；cwd 从 `session_projcache.json` 还原。
library;

import 'dart:convert';
import 'dart:io';

import '../models/session.dart';

class SessionRepository {
  const SessionRepository();

  /// DSH 数据目录（环境变量 DSH_HOME 优先，默认 ~/.dsh）
  String get dshHome {
    final env = Platform.environment['DSH_HOME'];
    final home = Platform.environment['HOME'];
    return env ?? (home == null ? '.dsh' : '$home/.dsh');
  }

  /// 读取全部会话（按工作区分组）
  Future<List<SessionGroup>> loadAll() async {
    final sessionsDir = Directory('$dshHome/sessions');
    if (!sessionsDir.existsSync()) {
      return [];
    }

    final cwds = _projCacheCwds();
    final workspaces = sessionsDir
        .listSync()
        .whereType<Directory>()
        .toList()
      ..sort((a, b) => a.path.compareTo(b.path));

    final groups = <SessionGroup>[];
    for (final ws in workspaces) {
      final name = ws.path.split(Platform.pathSeparator).last;
      final items = ws
          .listSync()
          .whereType<Directory>()
          .map((d) {
            final sid = d.path.split(Platform.pathSeparator).last;
            return SessionItem(
              id: sid,
              sizeBytes: _dirSize(d.path),
              mtime: d.statSync().modified,
              cwd: cwds[sid],
            );
          })
          .toList()
        ..sort((a, b) => a.id.compareTo(b.id));

      groups.add(SessionGroup(
        workspace: name,
        decoded: _decodeWorkspace(name),
        items: items,
      ));
    }
    return groups;
  }

  /// session_projcache.json：session_id → 工作目录
  /// 路径：$DSH_HOME/storages/session_projcache.json，结构 /tables/sessions/{sid}/identity/cwd
  Map<String, String> _projCacheCwds() {
    final f = File('$dshHome/storages/session_projcache.json');
    if (!f.existsSync()) return {};
    try {
      final data = jsonDecode(f.readAsStringSync());
      final tables = (data is Map) ? data['tables'] : null;
      final sessions = (tables is Map) ? tables['sessions'] : null;
      if (sessions is! Map) return {};
      final map = <String, String>{};
      for (final entry in sessions.entries) {
        final val = entry.value;
        if (val is Map) {
          final identity = val['identity'];
          if (identity is Map && identity['cwd'] is String) {
            map[entry.key.toString()] = identity['cwd'] as String;
          }
        }
      }
      return map;
    } catch (_) {
      return {};
    }
  }

  /// 工作区目录名解码（对齐 CLI decode_workspace）：
  /// --tmp-- → 临时工作区；--path-- → 去双横线、'-' 还原为 '/'
  static String _decodeWorkspace(String name) {
    if (name == '--tmp--') return '临时工作区';
    final inner = name.startsWith('--') && name.endsWith('--')
        ? name.substring(2, name.length - 2)
        : name;
    if (inner.isEmpty) return name;
    return inner.replaceAll('-', '/');
  }

  /// 递归目录大小
  static int _dirSize(String path) {
    var total = 0;
    final dir = Directory(path);
    if (!dir.existsSync()) return 0;
    for (final e in dir.listSync(recursive: true, followLinks: false)) {
      if (e is File) {
        total += e.statSync().size;
      }
    }
    return total;
  }
}
