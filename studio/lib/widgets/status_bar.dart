import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/pixel_canvas.dart';
import '../providers/canvas_provider.dart';
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
      DrawingTool.rectSelect => 'Select',
      DrawingTool.move => 'Move',
    };

    return Container(
      height: 24,
      decoration: StudioTheme.statusBarDecoration,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Row(
        children: [
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
          Text('PIXL Studio v0.1', style: style),
        ],
      ),
    );
  }

  Widget _sep() => const Padding(
        padding: EdgeInsets.symmetric(horizontal: 8),
        child: Text('|', style: TextStyle(color: Color(0xFF3a3a5e), fontSize: 11)),
      );
}
