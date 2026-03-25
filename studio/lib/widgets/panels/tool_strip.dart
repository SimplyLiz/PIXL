import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/pixel_canvas.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/tilemap_provider.dart';
import '../../theme/studio_theme.dart';

/// Vertical tool strip — thin toolbar between chat and canvas.
/// Shows drawing tools for pixel mode, stamp tools for tilemap mode.
class ToolStrip extends ConsumerWidget {
  const ToolStrip({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(editorModeProvider);
    final theme = Theme.of(context);

    return Container(
      width: 40,
      decoration: BoxDecoration(
        color: theme.cardColor,
        border: const Border(
          right: StudioTheme.panelBorder,
        ),
      ),
      child: Column(
        children: [
          const SizedBox(height: 8),
          if (mode == EditorMode.tilemap)
            _TilemapTools()
          else
            _PixelTools(),
        ],
      ),
    );
  }
}

class _PixelTools extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeTool = ref.watch(canvasProvider.select((s) => s.activeTool));
    final symmetry = ref.watch(canvasProvider.select((s) => s.symmetryMode));
    final notifier = ref.read(canvasProvider.notifier);

    return Column(
      children: [
        _StripButton(
          icon: Icons.edit,
          tooltip: 'Pencil (B)',
          active: activeTool == DrawingTool.pencil,
          onTap: () => notifier.setTool(DrawingTool.pencil),
        ),
        _StripButton(
          icon: Icons.auto_fix_high,
          tooltip: 'Eraser (E)',
          active: activeTool == DrawingTool.eraser,
          onTap: () => notifier.setTool(DrawingTool.eraser),
        ),
        _StripButton(
          icon: Icons.format_color_fill,
          tooltip: 'Fill (G)',
          active: activeTool == DrawingTool.bucket,
          onTap: () => notifier.setTool(DrawingTool.bucket),
        ),
        _StripButton(
          icon: Icons.colorize,
          tooltip: 'Eyedropper (I)',
          active: activeTool == DrawingTool.eyedropper,
          onTap: () => notifier.setTool(DrawingTool.eyedropper),
        ),
        _StripButton(
          icon: Icons.show_chart,
          tooltip: 'Line (L)',
          active: activeTool == DrawingTool.line,
          onTap: () => notifier.setTool(DrawingTool.line),
        ),
        _StripButton(
          icon: Icons.crop_square,
          tooltip: 'Rect (R)',
          active: activeTool == DrawingTool.rect,
          onTap: () => notifier.setTool(DrawingTool.rect),
        ),
        const _Divider(),
        _StripButton(
          icon: Icons.select_all,
          tooltip: 'Select (S)',
          active: activeTool == DrawingTool.rectSelect,
          onTap: () => notifier.setTool(DrawingTool.rectSelect),
        ),
        _StripButton(
          icon: Icons.open_with,
          tooltip: 'Move',
          active: activeTool == DrawingTool.move,
          onTap: () => notifier.setTool(DrawingTool.move),
        ),
        const _Divider(),
        // Symmetry quick-toggle
        _StripButton(
          icon: _symmetryIcon(symmetry),
          tooltip: 'Symmetry: ${symmetry.name}',
          active: symmetry != SymmetryMode.none,
          onTap: () {
            const cycle = [
              SymmetryMode.none,
              SymmetryMode.horizontal,
              SymmetryMode.vertical,
              SymmetryMode.both,
            ];
            final next = cycle[(cycle.indexOf(symmetry) + 1) % cycle.length];
            notifier.setSymmetry(next);
          },
        ),
      ],
    );
  }

  IconData _symmetryIcon(SymmetryMode mode) => switch (mode) {
    SymmetryMode.none => Icons.border_clear,
    SymmetryMode.horizontal => Icons.border_vertical,
    SymmetryMode.vertical => Icons.border_horizontal,
    SymmetryMode.both => Icons.border_all,
  };
}

class _TilemapTools extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeTool = ref.watch(tilemapProvider.select((s) => s.activeTool));
    final notifier = ref.read(tilemapProvider.notifier);

    return Column(
      children: [
        _StripButton(
          icon: Icons.grid_view,
          tooltip: 'Stamp (T)',
          active: activeTool == TilemapTool.stamp,
          onTap: () => notifier.setTool(TilemapTool.stamp),
        ),
        _StripButton(
          icon: Icons.auto_fix_high,
          tooltip: 'Eraser (E)',
          active: activeTool == TilemapTool.eraser,
          onTap: () => notifier.setTool(TilemapTool.eraser),
        ),
        _StripButton(
          icon: Icons.format_color_fill,
          tooltip: 'Fill (G)',
          active: activeTool == TilemapTool.bucket,
          onTap: () => notifier.setTool(TilemapTool.bucket),
        ),
        _StripButton(
          icon: Icons.colorize,
          tooltip: 'Eyedropper (I)',
          active: activeTool == TilemapTool.eyedropper,
          onTap: () => notifier.setTool(TilemapTool.eyedropper),
        ),
      ],
    );
  }
}

class _StripButton extends StatelessWidget {
  const _StripButton({
    required this.icon,
    required this.tooltip,
    required this.active,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Tooltip(
      message: tooltip,
      preferBelow: false,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(4),
        child: Container(
          width: 32,
          height: 32,
          margin: const EdgeInsets.symmetric(vertical: 1),
          decoration: BoxDecoration(
            color: active ? theme.colorScheme.primary.withValues(alpha: 0.25) : null,
            borderRadius: BorderRadius.circular(4),
            border: active
                ? Border.all(color: theme.colorScheme.primary, width: 1)
                : null,
          ),
          child: Icon(icon, size: 16,
            color: active ? theme.colorScheme.primary : theme.iconTheme.color),
        ),
      ),
    );
  }
}

class _Divider extends StatelessWidget {
  const _Divider();

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
      child: Container(
        height: 1,
        color: Theme.of(context).dividerColor,
      ),
    );
  }
}
