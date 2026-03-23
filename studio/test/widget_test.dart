import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:pixl_studio/providers/backend_provider.dart';
import 'package:pixl_studio/theme/studio_theme.dart';
import 'package:pixl_studio/widgets/canvas/canvas_viewport.dart';
import 'package:pixl_studio/widgets/panels/chat_panel.dart';
import 'package:pixl_studio/widgets/panels/tools_panel.dart';
import 'package:pixl_studio/widgets/status_bar.dart';
import 'package:pixl_studio/widgets/top_bar.dart';

void main() {
  testWidgets('Studio shell renders without backend', (WidgetTester tester) async {
    // Build a simplified shell that doesn't auto-connect to backend
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          // Override backend to stay disconnected (default state)
          backendProvider.overrideWith((ref) => BackendNotifier()),
        ],
        child: MaterialApp(
          theme: StudioTheme.theme,
          home: const Scaffold(
            body: Column(
              children: [
                TopBar(),
                Expanded(
                  child: Row(
                    children: [
                      ChatPanel(),
                      Expanded(child: CanvasViewport()),
                      ToolsPanel(),
                    ],
                  ),
                ),
                StatusBar(),
              ],
            ),
          ),
        ),
      ),
    );

    await tester.pump(const Duration(milliseconds: 50));

    // Top bar renders
    expect(find.text('PIXL'), findsWidgets);
    expect(find.text(' STUDIO'), findsOneWidget);

    // Chat panel renders
    expect(find.text('AI EXPERT'), findsOneWidget);

    // Tools panel renders
    expect(find.text('TOOLS'), findsOneWidget);
    expect(find.text('PALETTE'), findsOneWidget);
    expect(find.text('LAYERS'), findsOneWidget);

    // Backend section shows disconnected
    expect(find.text('ENGINE'), findsOneWidget);
  });
}
