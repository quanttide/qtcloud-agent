/// 智能体注册中心数据访问（独立管理数据——直读文件系统，不依赖 CLI）。
///
/// 数据源：注册中心目录（默认 `data/profile/agents`，可用环境变量
/// `QTCLOUD_AGENT_REGISTRY` 覆盖）+ 本地 Agent 配置目录（~/.config/zed 等）。
library;

import 'dart:convert';
import 'dart:io';

import '../models/agent.dart';

class AgentRepository {
  const AgentRepository();

  /// 已知 Agent 本地配置目录映射（对齐 CLI agent_local_dir）
  static String? agentLocalDir(String name) {
    final home = Platform.environment['HOME'];
    if (home == null) return null;
    return switch (name) {
      'zed' => '$home/.config/zed',
      'opencode' => '$home/.config/opencode',
      'hermes' => '$home/.config/hermes',
      _ => null,
    };
  }

  static bool _skipFile(String name) =>
      name == 'README.md' || name == 'LICENSE' || name == '.gitkeep';

  /// 注册中心路径（环境变量 QTCLOUD_AGENT_REGISTRY 优先，默认 data/profile）
  String get registryPath {
    final env = Platform.environment['QTCLOUD_AGENT_REGISTRY'];
    return env ?? 'data/profile';
  }

  /// 读取注册中心下所有 Agent 的状态
  Future<List<AgentStatus>> loadAll() async {
    final registry = Directory('$registryPath/agents');
    if (!registry.existsSync()) {
      return [];
    }

    final entries = registry
        .listSync()
        .whereType<Directory>()
        .toList()
      ..sort((a, b) => a.path.compareTo(b.path));

    final result = <AgentStatus>[];
    for (final dir in entries) {
      result.add(await _loadAgent(dir));
    }
    return result;
  }

  Future<AgentStatus> _loadAgent(Directory dir) async {
    final name = dir.path.split(Platform.pathSeparator).last;
    final localDir = agentLocalDir(name);
    final localExists =
        localDir != null && Directory(localDir).existsSync();

    final files = <AgentFile>[];
    final fileEntries = dir
        .listSync()
        .whereType<File>()
        .toList()
      ..sort((a, b) => a.path.compareTo(b.path));

    for (final f in fileEntries) {
      final fname = f.path.split(Platform.pathSeparator).last;
      if (_skipFile(fname)) {
        files.add(AgentFile(name: fname, status: AgentFileStatus.skip));
        continue;
      }
      if (localDir == null) {
        files.add(AgentFile(name: fname, status: AgentFileStatus.unknown));
        continue;
      }
      final localPath = '$localDir/$fname';
      if (!File(localPath).existsSync()) {
        files.add(AgentFile(name: fname, status: AgentFileStatus.missing));
        continue;
      }
      // 内容对比（JSON 展平——对齐 CLI flatten_json）
      final regKeys = _flattenJsonFile(f);
      final localKeys = _flattenJsonFile(File(localPath));
      final status = _sameKeys(regKeys, localKeys)
          ? AgentFileStatus.ok
          : AgentFileStatus.diff;
      files.add(AgentFile(name: fname, status: status));
    }

    return AgentStatus(
      name: name,
      localDir: localDir,
      localExists: localExists,
      files: files,
    );
  }

  /// 读取 JSON(JSONC) 文件并展平为 key → 字符串 映射
  static Map<String, String> _flattenJsonFile(File f) {
    try {
      final raw = f.readAsStringSync();
      final cleaned = _stripJsonc(raw);
      final value = jsonDecode(cleaned);
      final out = <String, String>{};
      _flatten(value, '', out);
      return out;
    } catch (_) {
      return {};
    }
  }

  static void _flatten(dynamic value, String prefix, Map<String, String> out) {
    if (value is Map) {
      for (final entry in value.entries) {
        final key = prefix.isEmpty ? entry.key.toString() : '$prefix.${entry.key}';
        _flatten(entry.value, key, out);
      }
    } else if (value is List) {
      for (var i = 0; i < value.length; i++) {
        _flatten(value[i], '$prefix[$i]', out);
      }
    } else {
      out[prefix] = value.toString();
    }
  }

  /// 去除 JSONC 行注释（//）与尾逗号
  static String _stripJsonc(String raw) {
    final lines = raw.split('\n');
    final cleaned = <String>[];
    for (final line in lines) {
      final idx = line.indexOf('//');
      cleaned.add(idx >= 0 ? line.substring(0, idx) : line);
    }
    var out = cleaned.join('\n');
    out = out.replaceAll(RegExp(r',(\s*[}\]])'), r'$1');
    return out;
  }

  static bool _sameKeys(Map<String, String> a, Map<String, String> b) {
    if (a.length != b.length) return false;
    for (final entry in a.entries) {
      if (b[entry.key] != entry.value) return false;
    }
    return true;
  }
}
