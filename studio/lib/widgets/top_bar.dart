import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/pixel_canvas.dart' show EditorMode;
import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../providers/claude_provider.dart';
import '../providers/tilemap_provider.dart';
import '../services/llm_provider.dart';
import '../services/export_service.dart';
import '../theme/studio_theme.dart';
import 'settings_dialog.dart';
import 'shortcuts_dialog.dart';
import 'tilegroup_dialog.dart';
import 'training_dialog.dart';
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
  await ExportService.setLastFilePath(path);
  final backend = ref.read(backendProvider);

  // If engine isn't connected, start it with this file
  if (!backend.isConnected) {
    messenger.showSnackBar(SnackBar(
      content: Text('Starting engine with ${path.split('/').last}...'),
      duration: const Duration(seconds: 4),
    ));
    final service = ref.read(claudeProvider.notifier).service;
    final isLocal = service.provider == LlmProviderType.pixlLocal;
    await ref.read(backendProvider.notifier).connect(
      paxFile: path,
      model: isLocal ? service.pixlModel : null,
      adapter: isLocal && service.hasPixlAdapter ? service.pixlAdapter : null,
    );

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
          const SizedBox(width: 16),

          // Mode toggle
          _ModeToggle(),
          const SizedBox(width: 16),

          // File actions
          _BarButton(label: 'New', icon: Icons.add, onTap: () => notifier.clearCanvas()),
          _BarButton(label: 'Open PAX', icon: Icons.folder_open, onTap: () => _openPaxFile(context, ref)),
          _RecentFilesMenu(),
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
            label: 'Training',
            icon: Icons.model_training,
            onTap: () => TrainingDialog.show(context),
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
          _BlueprintToggle(),
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
          case 'tiled':
          case 'godot':
          case 'texturepacker':
          case 'gbstudio':
          case 'unity':
            final dir = await FilePicker.platform.getDirectoryPath(
              dialogTitle: 'Export to $value',
            );
            if (dir == null) {
              messenger.showSnackBar(const SnackBar(
                content: Text('Export cancelled'),
                duration: Duration(seconds: 2),
              ));
              break;
            }
            final resp = await ref.read(backendProvider.notifier).backend.exportToEngine(
              format: value,
              outDir: dir,
            );
            if (resp['ok'] == true) {
              final files = resp['files'] as int? ?? 0;
              messenger.showSnackBar(SnackBar(
                content: Text('Exported $files files to $dir'),
                duration: const Duration(seconds: 3),
              ));
            } else {
              messenger.showSnackBar(SnackBar(
                content: Text('Export failed: ${resp['error'] ?? 'unknown'}'),
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
        PopupMenuDivider(),
        PopupMenuItem(value: 'tiled', child: Text('Export for Tiled', style: TextStyle(fontSize: 12))),
        PopupMenuItem(value: 'godot', child: Text('Export for Godot', style: TextStyle(fontSize: 12))),
        PopupMenuItem(value: 'texturepacker', child: Text('Export for TexturePacker', style: TextStyle(fontSize: 12))),
        PopupMenuItem(value: 'gbstudio', child: Text('Export for GB Studio', style: TextStyle(fontSize: 12))),
        PopupMenuItem(value: 'unity', child: Text('Export for Unity', style: TextStyle(fontSize: 12))),
      ],
    );
  }
}

class _RecentFilesMenu extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return FutureBuilder<List<String>>(
      future: ExportService.getRecentFiles(),
      builder: (context, snapshot) {
        final files = snapshot.data ?? [];
        if (files.isEmpty) return const SizedBox.shrink();

        return PopupMenuButton<String>(
          tooltip: 'Recent Files',
          iconSize: 18,
          icon: const Icon(Icons.history, size: 16),
          onSelected: (path) async {
            final file = File(path);
            if (!await file.exists()) return;
            final source = await file.readAsString();
            final backend = ref.read(backendProvider);

            if (!backend.isConnected) {
              final service = ref.read(claudeProvider.notifier).service;
              final isLocal = service.provider == LlmProviderType.pixlLocal;
              await ref.read(backendProvider.notifier).connect(
                paxFile: path,
                model: isLocal ? service.pixlModel : null,
                adapter: isLocal && service.hasPixlAdapter ? service.pixlAdapter : null,
              );
            } else {
              await ref.read(backendProvider.notifier).loadSource(source);
            }
            await ExportService.setLastFilePath(path);
          },
          itemBuilder: (_) => files.map((path) {
            final name = path.split('/').last;
            return PopupMenuItem(
              value: path,
              child: Text(name, style: const TextStyle(fontSize: 12)),
            );
          }).toList(),
        );
      },
    );
  }
}

class _BlueprintToggle extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final bp = ref.watch(blueprintProvider);
    final isActive = bp != null;

    return _BarButton(
      label: 'Blueprint Guide',
      icon: Icons.accessibility_new,
      active: isActive,
      onTap: () async {
        if (isActive) {
          ref.read(blueprintProvider.notifier).state = null;
        } else {
          final cs = ref.read(canvasProvider);
          final resp = await ref.read(backendProvider.notifier).backend.getBlueprint(
            width: cs.width,
            height: cs.height,
          );
          final landmarks = resp['landmarks'] as List<dynamic>?;
          if (landmarks != null) {
            ref.read(blueprintProvider.notifier).state =
                landmarks.map((l) => Map<String, dynamic>.from(l as Map)).toList();
          }
        }
      },
    );
  }
}

class _ModeToggle extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(editorModeProvider);
    final theme = Theme.of(context);

    Widget modeButton(EditorMode m, IconData icon, String label) {
      final isActive = mode == m;
      return InkWell(
        onTap: () => ref.read(editorModeProvider.notifier).state = m,
        borderRadius: BorderRadius.circular(4),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: BoxDecoration(
            color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.2) : null,
            borderRadius: BorderRadius.circular(4),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 14,
                color: isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color),
              const SizedBox(width: 4),
              Text(label, style: TextStyle(
                fontSize: 10,
                fontWeight: isActive ? FontWeight.w700 : null,
                color: isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color,
              )),
            ],
          ),
        ),
      );
    }

    return Container(
      padding: const EdgeInsets.all(2),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: theme.dividerColor),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          modeButton(EditorMode.pixel, Icons.edit, 'Pixel'),
          modeButton(EditorMode.tilemap, Icons.grid_view, 'Tilemap'),
        ],
      ),
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
        LlmProviderType.pixlLocal => 'LoRA',
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
