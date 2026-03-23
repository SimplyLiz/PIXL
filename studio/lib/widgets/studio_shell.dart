import 'package:flutter/material.dart';

import 'canvas/canvas_viewport.dart';
import 'panels/chat_panel.dart';
import 'panels/tools_panel.dart';
import 'status_bar.dart';
import 'top_bar.dart';

/// The main 3-panel layout: chat | canvas | tools.
class StudioShell extends StatelessWidget {
  const StudioShell({super.key});

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
