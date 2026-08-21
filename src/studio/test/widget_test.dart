import 'package:flutter_test/flutter_test.dart';

import 'package:qtcloud_agent_studio/main.dart';

void main() {
  testWidgets('应用启动渲染三职能导航（智能体/会话/配置）', (tester) async {
    await tester.pumpWidget(const AgentStudioApp());
    await tester.pumpAndSettle();

    // 侧边导航三职能
    expect(find.text('会话'), findsWidgets);
    expect(find.text('技能'), findsWidgets);
    expect(find.text('智能体'), findsWidgets);
  });
}
