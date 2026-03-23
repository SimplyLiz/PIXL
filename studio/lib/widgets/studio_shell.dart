import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import 'canvas/canvas_viewport.dart';
import 'panels/chat_panel.dart';
import 'panels/tools_panel.dart';
import 'status_bar.dart';
import 'top_bar.dart';

/// The main 3-panel layout: chat | canvas | tools.
class StudioShell extends ConsumerStatefulWidget {
  const StudioShell({super.key});

  @override
  ConsumerState<StudioShell> createState() => _StudioShellState();
}

class _StudioShellState extends ConsumerState<StudioShell> {
  @override
  void initState() {
    super.initState();
    // Connect to the PIXL backend on startup
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(backendProvider.notifier).connect();
    });
  }

  @override
  void deactivate() {
    // Use deactivate instead of dispose — ref is still valid here
    ref.read(backendProvider.notifier).disconnect();
    super.deactivate();
  }

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
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
    );
  }
}
