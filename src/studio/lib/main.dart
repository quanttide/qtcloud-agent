import 'package:flutter/material.dart';

import 'models/agent.dart';
import 'models/session.dart';
import 'repositories/agent_repository.dart';
import 'repositories/session_repository.dart';

void main() {
  runApp(const AgentStudioApp());
}

class AgentStudioApp extends StatelessWidget {
  const AgentStudioApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '量潮智能体云',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
        useMaterial3: true,
      ),
      home: const ShellScreen(),
    );
  }
}

/// 应用外壳：三职能导航（会话 / 技能 / 智能体）
class ShellScreen extends StatefulWidget {
  const ShellScreen({super.key});

  @override
  State<ShellScreen> createState() => _ShellScreenState();
}

class _ShellScreenState extends State<ShellScreen> {
  int _index = 0;

  static const _titles = ['会话', '技能', '智能体'];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(_titles[_index])),
      body: Row(
        children: [
          NavigationRail(
            selectedIndex: _index,
            onDestinationSelected: (i) => setState(() => _index = i),
            labelType: NavigationRailLabelType.all,
            destinations: const [
              NavigationRailDestination(
                icon: Icon(Icons.forum_outlined),
                selectedIcon: Icon(Icons.forum),
                label: Text('会话'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.extension_outlined),
                selectedIcon: Icon(Icons.extension),
                label: Text('技能'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.smart_toy_outlined),
                selectedIcon: Icon(Icons.smart_toy),
                label: Text('智能体'),
              ),
            ],
          ),
          const VerticalDivider(thickness: 1, width: 1),
          Expanded(
            child: switch (_index) {
              0 => const SessionsScreen(),
              1 => const SkillsScreen(),
              _ => const AgentsScreen(),
            },
          ),
        ],
      ),
    );
  }
}

// ─── 会话页（直读 DSH 数据目录） ───

class SessionsScreen extends StatefulWidget {
  const SessionsScreen({super.key});

  @override
  State<SessionsScreen> createState() => _SessionsScreenState();
}

class _SessionsScreenState extends State<SessionsScreen> {
  final _repo = const SessionRepository();
  late Future<List<SessionGroup>> _future;

  @override
  void initState() {
    super.initState();
    _future = _repo.loadAll();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: FutureBuilder<List<SessionGroup>>(
        future: _future,
        builder: (context, snap) {
          if (snap.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }
          final groups = snap.data ?? [];
          if (groups.isEmpty) {
            return Text('无会话（DSH 数据目录 ${_repo.dshHome} 无 sessions/）');
          }
          return ListView(
            children: [
              for (final g in groups) ...[
                ListTile(
                  leading: const Icon(Icons.folder_outlined),
                  title: Text(g.decoded),
                  subtitle: Text(
                      '${g.totalSessions} 个会话 · ${_formatSize(g.totalSize)}'),
                ),
                for (final s in g.items)
                  ListTile(
                    dense: true,
                    leading: const Icon(Icons.forum_outlined, size: 18),
                    title: Text(s.id),
                    subtitle: Text(
                        '${s.sizeLabel} · ${s.mtimeLabel}${s.cwd == null ? '' : ' · ${s.cwd}'}'),
                  ),
                const Divider(),
              ],
            ],
          );
        },
      ),
    );
  }

  static String _formatSize(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / 1024 / 1024).toStringAsFixed(1)} MB';
  }
}

// ─── 技能页（分析会话 → 制作技能的最小循环） ───

class SkillsScreen extends StatelessWidget {
  const SkillsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('技能',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          const Text('从会话中提取可复用的技能资产（skill / 上下文 / 知识 / harness）。'),
          const SizedBox(height: 16),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: const [
                  Text('待建设：分析会话 → 制作技能的最小循环',
                      style: TextStyle(fontWeight: FontWeight.bold)),
                  SizedBox(height: 8),
                  Text('· 导入会话（DSH 格式）\n'
                      '· 标注与解读\n'
                      '· 提取 skill / 上下文 / 知识\n'
                      '· 二次开发（脱敏后成 CLI 等）'),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ─── 智能体页（注册中心 vs 本地配置） ───

class AgentsScreen extends StatefulWidget {
  const AgentsScreen({super.key});

  @override
  State<AgentsScreen> createState() => _AgentsScreenState();
}

class _AgentsScreenState extends State<AgentsScreen> {
  final _repo = const AgentRepository();
  late Future<List<AgentStatus>> _future;

  @override
  void initState() {
    super.initState();
    _future = _repo.loadAll();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: FutureBuilder<List<AgentStatus>>(
        future: _future,
        builder: (context, snap) {
          if (snap.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }
          final agents = snap.data ?? [];
          if (agents.isEmpty) {
            return const Text('无注册智能体（注册中心 agents/ 目录为空）');
          }
          return ListView(
            children: [
              for (final a in agents)
                Card(
                  margin: const EdgeInsets.symmetric(vertical: 4),
                  child: ExpansionTile(
                    leading: Icon(
                      a.localExists ? Icons.smart_toy : Icons.error_outline,
                      color: a.localExists ? null : Colors.orange,
                    ),
                    title: Text(a.name),
                    subtitle: Text(
                      a.localExists
                          ? '${a.files.length} 个配置 · ${a.issueCount} 处差异'
                          : '本地目录缺失',
                    ),
                    children: [
                      for (final f in a.files)
                        ListTile(
                          dense: true,
                          leading: _statusIcon(f.status),
                          title: Text(f.name),
                          trailing: Text(f.statusLabel),
                        ),
                    ],
                  ),
                ),
            ],
          );
        },
      ),
    );
  }

  static Widget _statusIcon(AgentFileStatus s) {
    return switch (s) {
      AgentFileStatus.ok => const Icon(Icons.check_circle_outline,
          color: Colors.green, size: 18),
      AgentFileStatus.diff => const Icon(Icons.sync_problem,
          color: Colors.orange, size: 18),
      AgentFileStatus.missing => const Icon(Icons.error_outline,
          color: Colors.red, size: 18),
      AgentFileStatus.skip => const Icon(Icons.description_outlined,
          color: Colors.grey, size: 18),
      AgentFileStatus.unknown => const Icon(Icons.help_outline,
          color: Colors.grey, size: 18),
    };
  }
}
