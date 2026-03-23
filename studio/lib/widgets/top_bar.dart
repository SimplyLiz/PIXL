import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../providers/claude_provider.dart';
import '../services/export_service.dart';
import '../theme/studio_theme.dart';
import 'settings_dialog.dart';

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
          _ExportMenu(),
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
            onTap: () => notifier.setZoom(cs.zoomLevel - 2),
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
            onTap: () => notifier.setZoom(cs.zoomLevel + 2),
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
    final hasKey = ref.watch(claudeProvider.select((s) => s.hasApiKey));
    final theme = Theme.of(context);
    if (hasKey) {
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.key, size: 12, color: Color(0xFF4caf50)),
          const SizedBox(width: 4),
          Text('AI', style: theme.textTheme.bodySmall!.copyWith(
            fontSize: 10, color: const Color(0xFF4caf50),
          )),
        ],
      );
    }
    return InkWell(
      onTap: () => SettingsDialog.show(context),
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: const Color(0xFFffaa00)),
        ),
        child: Text('Add API Key', style: theme.textTheme.bodySmall!.copyWith(
          fontSize: 9, color: const Color(0xFFffaa00),
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
