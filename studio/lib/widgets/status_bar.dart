import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/pixel_canvas.dart';
import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../providers/hover_provider.dart';
import '../providers/palette_provider.dart';
import '../theme/studio_theme.dart';

/// Bottom status bar — position, color, canvas info.
class StatusBar extends ConsumerWidget {
  const StatusBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = ref.watch(canvasProvider);
    final palette = ref.watch(paletteProvider);
    final theme = Theme.of(context);
    final style = theme.textTheme.bodySmall!.copyWith(fontSize: 11);

    final toolName = switch (cs.activeTool) {
      DrawingTool.pencil => 'Pencil',
      DrawingTool.eraser => 'Eraser',
      DrawingTool.bucket => 'Fill',
      DrawingTool.eyedropper => 'Eyedropper',
      DrawingTool.line => 'Line',
      DrawingTool.rect => 'Rect',
      DrawingTool.rectSelect => 'Select',
      DrawingTool.move => 'Move',
    };

    return Container(
      height: 24,
      decoration: StudioTheme.statusBarDecoration,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Row(
        children: [
          _PixelPosition(),
          _sep(),
          Text(cs.canvasSize.label, style: style),
          _sep(),
          Text(toolName, style: style),
          _sep(),
          Container(
            width: 10,
            height: 10,
            margin: const EdgeInsets.only(right: 4),
            decoration: BoxDecoration(
              color: palette[cs.foregroundColorIndex],
              border: Border.all(color: theme.dividerColor, width: 0.5),
            ),
          ),
          Text(
            '#${palette[cs.foregroundColorIndex].toARGB32().toRadixString(16).padLeft(8, '0').substring(2)}',
            style: style,
          ),
          _sep(),
          if (cs.symmetryMode != SymmetryMode.none)
            Text(
              'SYM: ${cs.symmetryMode.name.toUpperCase()}',
              style: style.copyWith(color: theme.colorScheme.primary),
            ),
          const Spacer(),
          _EngineIndicator(),
          _sep(),
          Text('PIXL Studio v1.0', style: style),
        ],
      ),
    );
  }

  Widget _sep() => const Padding(
        padding: EdgeInsets.symmetric(horizontal: 8),
        child: Text('|', style: TextStyle(color: StudioTheme.separatorColor, fontSize: 11)),
      );
}

class _PixelPosition extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hover = ref.watch(hoverProvider);
    final style = Theme.of(context).textTheme.bodySmall!.copyWith(fontSize: 11);
    return SizedBox(
      width: 48,
      child: Text(
        hover.hasPosition ? '${hover.x}, ${hover.y}' : '--, --',
        style: style.copyWith(
          color: hover.hasPosition ? null : StudioTheme.separatorColor,
        ),
      ),
    );
  }
}

class _EngineIndicator extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final status = ref.watch(backendProvider.select((s) => s.status));
    final tileCount = ref.watch(backendProvider.select((s) => s.tiles.length));
    final style = Theme.of(context).textTheme.bodySmall!.copyWith(fontSize: 11);

    final (color, label) = switch (status) {
      BackendStatus.connected => (StudioTheme.success, 'Engine OK ($tileCount tiles)'),
      BackendStatus.connecting => (StudioTheme.warning, 'Connecting...'),
      BackendStatus.error => (StudioTheme.error, 'Engine Error'),
      BackendStatus.disconnected => (StudioTheme.separatorColor, 'Offline'),
    };

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 6, height: 6,
          decoration: BoxDecoration(shape: BoxShape.circle, color: color),
        ),
        const SizedBox(width: 4),
        Text(label, style: style.copyWith(color: color)),
      ],
    );
  }
}
