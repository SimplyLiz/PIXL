import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../providers/claude_provider.dart';
import '../services/llm_provider.dart';
import '../services/export_service.dart';
import '../theme/studio_theme.dart';
import 'settings_dialog.dart';
import 'shortcuts_dialog.dart';
import 'tilegroup_dialog.dart';
import 'wfc_dialog.dart';

Future<void> _openPaxFile(BuildContext context, WidgetRef ref) async {
  final messenger = ScaffoldMessenger.of(context);

  // Use FileType.any because .pax isn't a registered UTI on macOS —
  // FileType.custom with allowedExtensions silently shows no files.
  final result = await FilePicker.platform.pickFiles(
    dialogTitle: 'Open PAX File',
    type: FileType.any,
  );
  if (result == null || result.files.isEmpty) return;

  final path = result.files.single.path;
  if (path == null) return;

  if (!path.endsWith('.pax') && !path.endsWith('.pixl')) {
    messenger.showSnackBar(const SnackBar(
      content: Text('Please select a .pax or .pixl file'),
    ));
    return;
  }

  final source = await File(path).readAsString();
  final backend = ref.read(backendProvider);

  // If engine isn't connected, start it with this file
  if (!backend.isConnected) {
    messenger.showSnackBar(SnackBar(
      content: Text('Starting engine with ${path.split('/').last}...'),
      duration: const Duration(seconds: 4),
    ));
    await ref.read(backendProvider.notifier).connect(paxFile: path);

    if (!ref.read(backendProvider).isConnected) {
      messenger.showSnackBar(const SnackBar(
        content: Text(
          'Could not start engine. Make sure pixl is built:\n'
          'cd tool && cargo build --release',
        ),
        duration: Duration(seconds: 6),
      ));
      return;
    }

    messenger.showSnackBar(SnackBar(
      content: Text('Loaded ${path.split('/').last}'),
      duration: const Duration(seconds: 2),
    ));
    return;
  }

  // Engine already running — load source into session
  final resp = await ref.read(backendProvider.notifier).loadSource(source);
  if (resp.containsKey('error')) {
    messenger.showSnackBar(SnackBar(
      content: Text('Failed to load: ${resp['error']}'),
    ));
  } else {
    messenger.showSnackBar(SnackBar(
      content: Text('Loaded ${path.split('/').last}'),
      duration: const Duration(seconds: 2),
    ));
  }
}

/// Top menu bar with logo, actions, and canvas controls.
class TopBar extends ConsumerWidget {
  const TopBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = ref.watch(canvasProvider);
    final notifier = ref.read(canvasProvider.notifier);
    final theme = Theme.of(context);

    return Container(
      height: 40,
      decoration: StudioTheme.topBarDecoration,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Row(
        children: [
          // Logo
          Text(
            'PIXL',
            style: theme.textTheme.bodyMedium!.copyWith(
              fontWeight: FontWeight.w700,
              fontSize: 15,
              color: theme.colorScheme.primary,
              letterSpacing: 2,
            ),
          ),
          Text(
            ' STUDIO',
            style: theme.textTheme.bodySmall!.copyWith(
              fontSize: 11,
              letterSpacing: 1.5,
            ),
          ),
          const SizedBox(width: 24),

          // File actions
          _BarButton(label: 'New', icon: Icons.add, onTap: () => notifier.clearCanvas()),
          _BarButton(label: 'Open PAX', icon: Icons.folder_open, onTap: () => _openPaxFile(context, ref)),
          _ExportMenu(),
          const SizedBox(width: 8),
          _BarButton(
            label: 'Generate Tilegroup',
            icon: Icons.grid_view,
            onTap: () => TilegroupDialog.show(context),
          ),
          _BarButton(
            label: 'WFC Map',
            icon: Icons.auto_awesome_mosaic,
            onTap: () => WfcDialog.show(context),
          ),
          _BarButton(
            label: 'Settings',
            icon: Icons.settings,
            onTap: () => SettingsDialog.show(context),
          ),

          const Spacer(),

          // API key status
          _ApiKeyBadge(),
          const SizedBox(width: 12),
          // Canvas controls
          _BarButton(
            label: 'Grid',
            icon: cs.showGrid ? Icons.grid_on : Icons.grid_off,
            onTap: () => notifier.toggleGrid(),
            active: cs.showGrid,
          ),
          const SizedBox(width: 4),
          // Zoom controls
          _BarButton(
            label: 'Zoom Out',
            icon: Icons.remove,
            onTap: () => notifier.zoomOut(),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: Text(
              '${cs.zoomLevel.round()}x',
              style: theme.textTheme.bodySmall,
            ),
          ),
          _BarButton(
            label: 'Zoom In',
            icon: Icons.add,
            onTap: () => notifier.zoomIn(),
          ),
          const SizedBox(width: 12),

          // Undo/Redo
          _BarButton(
            label: 'Undo (Cmd+Z)',
            icon: Icons.undo,
            onTap: notifier.canUndo ? () => notifier.undo() : null,
          ),
          _BarButton(
            label: 'Redo (Cmd+Shift+Z)',
            icon: Icons.redo,
            onTap: notifier.canRedo ? () => notifier.redo() : null,
          ),
          const SizedBox(width: 8),
          _BarButton(
            label: 'Shortcuts (Cmd+/)',
            icon: Icons.help_outline,
            onTap: () => ShortcutsDialog.show(context),
          ),
        ],
      ),
    );
  }
}

class _ExportMenu extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return PopupMenuButton<String>(
      tooltip: 'Export',
      iconSize: 18,
      icon: const Icon(Icons.file_download, size: 18),
      onSelected: (value) async {
        final messenger = ScaffoldMessenger.of(context);
        switch (value) {
          case 'png':
            final cs = ref.read(canvasProvider);
            final ok = await ExportService.exportCanvasPng(canvasState: cs, scale: 4);
            messenger.showSnackBar(SnackBar(
              content: Text(ok ? 'PNG exported' : 'Export cancelled'),
              duration: const Duration(seconds: 2),
            ));
            break;
          case 'png8x':
            final cs = ref.read(canvasProvider);
            final ok = await ExportService.exportCanvasPng(canvasState: cs, scale: 8);
            messenger.showSnackBar(SnackBar(
              content: Text(ok ? 'PNG exported (8x)' : 'Export cancelled'),
              duration: const Duration(seconds: 2),
            ));
            break;
          case 'pax':
            final source = await ref.read(backendProvider.notifier).getPaxSource();
            if (source != null) {
              final ok = await ExportService.savePaxSource(source);
              messenger.showSnackBar(SnackBar(
                content: Text(ok ? 'PAX source saved' : 'Save cancelled'),
                duration: const Duration(seconds: 2),
              ));
            } else {
              messenger.showSnackBar(const SnackBar(
                content: Text('No PAX source available (engine not connected?)'),
              ));
            }
            break;
          case 'atlas':
            final resp = await ref.read(backendProvider.notifier).packAtlas();
            final png = resp['png'] as String?;
            if (png != null) {
              final ok = await ExportService.saveAtlasPng(png);
              messenger.showSnackBar(SnackBar(
                content: Text(ok ? 'Atlas exported' : 'Export cancelled'),
                duration: const Duration(seconds: 2),
              ));
            } else {
              messenger.showSnackBar(SnackBar(
                content: Text('Atlas pack failed: ${resp['error'] ?? 'unknown'}'),
              ));
            }
            break;
        }
      },
      itemBuilder: (_) => const [
        PopupMenuItem(value: 'png', child: Text('Export PNG (4x)', style: TextStyle(fontSize: 12))),
        PopupMenuItem(value: 'png8x', child: Text('Export PNG (8x)', style: TextStyle(fontSize: 12))),
        PopupMenuDivider(),
        PopupMenuItem(value: 'pax', child: Text('Save PAX Source', style: TextStyle(fontSize: 12))),
        PopupMenuItem(value: 'atlas', child: Text('Export Atlas', style: TextStyle(fontSize: 12))),
      ],
    );
  }
}

class _ApiKeyBadge extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final llm = ref.watch(claudeProvider);
    final theme = Theme.of(context);
    if (llm.hasApiKey) {
      // Show active provider name
      final providerShort = switch (llm.provider) {
        LlmProviderType.anthropic => 'Claude',
        LlmProviderType.openai => 'GPT',
        LlmProviderType.gemini => 'Gemini',
        LlmProviderType.ollama => 'Ollama',
      };
      return InkWell(
        onTap: () => SettingsDialog.show(context),
        borderRadius: BorderRadius.circular(4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.key, size: 12, color: Color(0xFF4caf50)),
            const SizedBox(width: 4),
            Text(providerShort, style: theme.textTheme.bodySmall!.copyWith(
              fontSize: 10, color: StudioTheme.success,
            )),
          ],
        ),
      );
    }
    return InkWell(
      onTap: () => SettingsDialog.show(context),
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: StudioTheme.warning),
        ),
        child: Text('Add API Key', style: theme.textTheme.bodySmall!.copyWith(
          fontSize: 9, color: StudioTheme.warning,
        )),
      ),
    );
  }
}

class _BarButton extends StatelessWidget {
  const _BarButton({
    required this.label,
    required this.icon,
    this.onTap,
    this.active = false,
  });

  final String label;
  final IconData icon;
  final VoidCallback? onTap;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Tooltip(
      message: label,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(4),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
          child: Icon(
            icon,
            size: 18,
            color: onTap == null
                ? theme.disabledColor
                : active
                    ? theme.colorScheme.primary
                    : null,
          ),
        ),
      ),
    );
  }
}
