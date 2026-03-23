import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/palette.dart';
import '../../models/pixel_canvas.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/palette_provider.dart';
import '../../theme/studio_theme.dart';

/// Right panel — tools, palette, layers, tile info.
class ToolsPanel extends ConsumerWidget {
  const ToolsPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      width: 200,
      decoration: StudioTheme.rightPanelDecoration,
      child: SingleChildScrollView(
        padding: StudioTheme.panelPadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: const [
            _ToolsSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _SymmetrySection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _PaletteSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _LayersSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _CanvasSizeSection(),
          ],
        ),
      ),
    );
  }
}

// -- Drawing Tools --

class _ToolsSection extends ConsumerWidget {
  const _ToolsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeTool = ref.watch(canvasProvider.select((s) => s.activeTool));
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('TOOLS', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: [
            _ToolButton(
              icon: Icons.edit,
              label: 'Pencil (B)',
              tool: DrawingTool.pencil,
              active: activeTool,
            ),
            _ToolButton(
              icon: Icons.auto_fix_high,
              label: 'Eraser (E)',
              tool: DrawingTool.eraser,
              active: activeTool,
            ),
            _ToolButton(
              icon: Icons.format_color_fill,
              label: 'Fill (G)',
              tool: DrawingTool.bucket,
              active: activeTool,
            ),
            _ToolButton(
              icon: Icons.colorize,
              label: 'Eyedropper (I)',
              tool: DrawingTool.eyedropper,
              active: activeTool,
            ),
            _ToolButton(
              icon: Icons.crop_square,
              label: 'Select',
              tool: DrawingTool.rectSelect,
              active: activeTool,
            ),
            _ToolButton(
              icon: Icons.open_with,
              label: 'Move',
              tool: DrawingTool.move,
              active: activeTool,
            ),
          ],
        ),
      ],
    );
  }
}

class _ToolButton extends ConsumerWidget {
  const _ToolButton({
    required this.icon,
    required this.label,
    required this.tool,
    required this.active,
  });

  final IconData icon;
  final String label;
  final DrawingTool tool;
  final DrawingTool active;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isActive = tool == active;
    final theme = Theme.of(context);

    return Tooltip(
      message: label,
      child: InkWell(
        onTap: () => ref.read(canvasProvider.notifier).setTool(tool),
        borderRadius: BorderRadius.circular(4),
        child: Container(
          width: 36,
          height: 36,
          decoration: BoxDecoration(
            color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.3) : null,
            borderRadius: BorderRadius.circular(4),
            border: isActive
                ? Border.all(color: theme.colorScheme.primary, width: 1)
                : Border.all(color: Colors.transparent),
          ),
          child: Icon(
            icon,
            size: 18,
            color: isActive ? theme.colorScheme.primary : null,
          ),
        ),
      ),
    );
  }
}

// -- Symmetry --

class _SymmetrySection extends ConsumerWidget {
  const _SymmetrySection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(canvasProvider.select((s) => s.symmetryMode));
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('SYMMETRY', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        Wrap(
          spacing: 4,
          children: SymmetryMode.values.map((m) {
            final isActive = m == mode;
            final label = switch (m) {
              SymmetryMode.none => 'Off',
              SymmetryMode.horizontal => 'H',
              SymmetryMode.vertical => 'V',
              SymmetryMode.both => 'H+V',
            };
            return InkWell(
              onTap: () => ref.read(canvasProvider.notifier).setSymmetry(m),
              borderRadius: BorderRadius.circular(4),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.3) : null,
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: isActive ? theme.colorScheme.primary : theme.dividerColor,
                  ),
                ),
                child: Text(
                  label,
                  style: theme.textTheme.bodySmall!.copyWith(
                    color: isActive ? theme.colorScheme.primary : null,
                    fontSize: 11,
                  ),
                ),
              ),
            );
          }).toList(),
        ),
      ],
    );
  }
}

// -- Palette --

class _PaletteSection extends ConsumerWidget {
  const _PaletteSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = ref.watch(paletteProvider);
    final cs = ref.watch(canvasProvider);
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('PALETTE', style: theme.textTheme.titleSmall),
            const Spacer(),
            // Palette selector dropdown
            PopupMenuButton<String>(
              tooltip: 'Switch palette',
              iconSize: 16,
              icon: const Icon(Icons.palette, size: 14),
              onSelected: (name) =>
                  ref.read(paletteProvider.notifier).selectBuiltIn(name),
              itemBuilder: (_) => BuiltInPalettes.all
                  .map((p) => PopupMenuItem(
                        value: p.name,
                        child: Text(p.name, style: const TextStyle(fontSize: 12)),
                      ))
                  .toList(),
            ),
          ],
        ),
        const SizedBox(height: 6),
        Text(palette.name, style: theme.textTheme.bodySmall),
        const SizedBox(height: 4),
        Wrap(
          spacing: 3,
          runSpacing: 3,
          children: List.generate(palette.length, (i) {
            final color = palette[i];
            final isFg = i == cs.foregroundColorIndex;
            final isBg = i == cs.backgroundColorIndex;
            return GestureDetector(
              onTap: () => ref.read(canvasProvider.notifier).setForegroundColor(i),
              onSecondaryTap: () =>
                  ref.read(canvasProvider.notifier).setBackgroundColor(i),
              child: Container(
                width: 22,
                height: 22,
                decoration: BoxDecoration(
                  color: color,
                  borderRadius: BorderRadius.circular(2),
                  border: Border.all(
                    color: isFg
                        ? Colors.white
                        : isBg
                            ? theme.colorScheme.secondary
                            : theme.dividerColor,
                    width: isFg || isBg ? 2 : 1,
                  ),
                ),
                child: color.alpha == 0
                    ? CustomPaint(painter: _TransparentPainter())
                    : null,
              ),
            );
          }),
        ),
        const SizedBox(height: 4),
        Row(
          children: [
            Container(
              width: 16,
              height: 16,
              color: palette[cs.foregroundColorIndex],
            ),
            const SizedBox(width: 4),
            Text(
              'FG: #${palette[cs.foregroundColorIndex].value.toRadixString(16).padLeft(8, '0').substring(2)}',
              style: theme.textTheme.bodySmall,
            ),
          ],
        ),
      ],
    );
  }
}

class _TransparentPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final s = size.width / 2;
    final light = Paint()..color = const Color(0xFFcccccc);
    final dark = Paint()..color = const Color(0xFF888888);
    canvas.drawRect(Rect.fromLTWH(0, 0, s, s), light);
    canvas.drawRect(Rect.fromLTWH(s, 0, s, s), dark);
    canvas.drawRect(Rect.fromLTWH(0, s, s, s), dark);
    canvas.drawRect(Rect.fromLTWH(s, s, s, s), light);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

// -- Layers --

class _LayersSection extends ConsumerWidget {
  const _LayersSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = ref.watch(canvasProvider);
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('LAYERS', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        ...List.generate(cs.layers.length, (i) {
          final layer = cs.layers[i];
          final isActive = i == cs.activeLayerIndex;
          return InkWell(
            onTap: () => ref.read(canvasProvider.notifier).setActiveLayer(i),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
              margin: const EdgeInsets.only(bottom: 2),
              decoration: BoxDecoration(
                color: isActive
                    ? theme.colorScheme.primary.withValues(alpha: 0.15)
                    : null,
                borderRadius: BorderRadius.circular(3),
              ),
              child: Row(
                children: [
                  InkWell(
                    onTap: () => ref
                        .read(canvasProvider.notifier)
                        .toggleLayerVisibility(i),
                    child: Icon(
                      layer.visible ? Icons.visibility : Icons.visibility_off,
                      size: 14,
                      color: layer.visible ? null : theme.disabledColor,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text(
                    layer.name,
                    style: theme.textTheme.bodySmall!.copyWith(
                      color: isActive ? theme.colorScheme.primary : null,
                    ),
                  ),
                ],
              ),
            ),
          );
        }),
      ],
    );
  }
}

// -- Canvas Size --

class _CanvasSizeSection extends ConsumerWidget {
  const _CanvasSizeSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final current = ref.watch(canvasProvider.select((s) => s.canvasSize));
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('CANVAS', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: CanvasSize.values.map((size) {
            final isActive = size == current;
            return InkWell(
              onTap: () =>
                  ref.read(canvasProvider.notifier).setCanvasSize(size),
              borderRadius: BorderRadius.circular(4),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
                decoration: BoxDecoration(
                  color: isActive
                      ? theme.colorScheme.primary.withValues(alpha: 0.3)
                      : null,
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: isActive
                        ? theme.colorScheme.primary
                        : theme.dividerColor,
                  ),
                ),
                child: Text(
                  size.label,
                  style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 10,
                    color: isActive ? theme.colorScheme.primary : null,
                  ),
                ),
              ),
            );
          }).toList(),
        ),
      ],
    );
  }
}
