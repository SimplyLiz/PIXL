import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/claude_provider.dart';
import '../services/llm_provider.dart';
import 'canvas/canvas_viewport.dart';
import 'canvas/variant_strip.dart';
import 'panels/chat_panel.dart';
import 'panels/tools_panel.dart';
import 'status_bar.dart';
import 'top_bar.dart';
import 'training_dialog.dart';

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
    // Initialize LLM settings first, then connect backend with inference config
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final llmNotifier = ref.read(claudeProvider.notifier);
      await llmNotifier.init();
      final service = llmNotifier.service;
      // Pass model/adapter if PIXL Local is the active provider
      final isLocal = service.provider == LlmProviderType.pixlLocal;
      ref.read(backendProvider.notifier).connect(
        model: isLocal ? service.pixlModel : null,
        adapter: isLocal && service.hasPixlAdapter ? service.pixlAdapter : null,
      );

      // Show auto-learn opt-in dialog on first launch
      if (mounted) {
        AutoLearnOptInDialog.showIfNeeded(context, ref);
      }
    });
  }

  // Backend cleanup is handled by BackendNotifier.dispose() when the
  // ProviderScope is torn down — no need to disconnect in deactivate/dispose
  // here, which would fire too eagerly on dialog navigation.

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
                Expanded(
                  child: Column(
                    children: [
                      Expanded(child: CanvasViewport()),
                      VariantStrip(),
                    ],
                  ),
                ),
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
