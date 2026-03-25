import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/palette.dart';
import '../../models/pixel_canvas.dart';
import '../../providers/backend_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/claude_provider.dart';
import '../../providers/chat_provider.dart';
import '../../providers/tilemap_provider.dart';
import '../../services/llm_provider.dart';
import '../../providers/palette_provider.dart';
import '../../providers/style_provider.dart';
import '../../services/knowledge_base.dart';
import '../../theme/studio_theme.dart';
import '../../utils/grid_parser.dart';

/// Right panel — tools, palette, layers, tile info, validation.
class ToolsPanel extends ConsumerWidget {
  const ToolsPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(editorModeProvider);
    final isTilemap = mode == EditorMode.tilemap;

    return Container(
      width: 220,
      decoration: StudioTheme.rightPanelDecoration,
      child: SingleChildScrollView(
        padding: StudioTheme.panelPadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (isTilemap) ...[
              const _TilemapToolsSection(),
              const SizedBox(height: StudioTheme.sectionSpacing),
              const _TilemapSizeSection(),
            ] else ...[
              const _ToolsSection(),
              const SizedBox(height: StudioTheme.sectionSpacing),
              const _SymmetrySection(),
              const SizedBox(height: StudioTheme.sectionSpacing),
              const _PaletteSection(),
              const SizedBox(height: StudioTheme.sectionSpacing),
              const _LayersSection(),
              const SizedBox(height: StudioTheme.sectionSpacing),
              const _CanvasSizeSection(),
            ],
            const SizedBox(height: StudioTheme.sectionSpacing),
            const _StyleSection(),
            const SizedBox(height: StudioTheme.sectionSpacing),
            const _QuickGenerateSection(),
            const SizedBox(height: StudioTheme.sectionSpacing),
            const _BackendSection(),
            const SizedBox(height: StudioTheme.sectionSpacing),
            const _ValidationSection(),
            const SizedBox(height: StudioTheme.sectionSpacing),
            const _TileListSection(),
          ],
        ),
      ),
    );
  }
}

// ── Drawing Tools ──────────────────────────────────────────

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
            _ToolButton(icon: Icons.edit, label: 'Pencil (B)', tool: DrawingTool.pencil, active: activeTool),
            _ToolButton(icon: Icons.auto_fix_high, label: 'Eraser (E)', tool: DrawingTool.eraser, active: activeTool),
            _ToolButton(icon: Icons.format_color_fill, label: 'Fill (G)', tool: DrawingTool.bucket, active: activeTool),
            _ToolButton(icon: Icons.colorize, label: 'Eyedropper (I)', tool: DrawingTool.eyedropper, active: activeTool),
            _ToolButton(icon: Icons.crop_square, label: 'Select', tool: DrawingTool.rectSelect, active: activeTool),
            _ToolButton(icon: Icons.open_with, label: 'Move', tool: DrawingTool.move, active: activeTool),
          ],
        ),
      ],
    );
  }
}

class _ToolButton extends ConsumerWidget {
  const _ToolButton({required this.icon, required this.label, required this.tool, required this.active});
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
          width: 36, height: 36,
          decoration: BoxDecoration(
            color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.3) : null,
            borderRadius: BorderRadius.circular(4),
            border: isActive
                ? Border.all(color: theme.colorScheme.primary, width: 1)
                : Border.all(color: Colors.transparent),
          ),
          child: Icon(icon, size: 18, color: isActive ? theme.colorScheme.primary : null),
        ),
      ),
    );
  }
}

// ── Tilemap Tools ─────────────────────────────────────────

class _TilemapToolsSection extends ConsumerWidget {
  const _TilemapToolsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeTool = ref.watch(tilemapProvider.select((s) => s.activeTool));
    final theme = Theme.of(context);

    Widget btn(IconData icon, String label, TilemapTool tool) {
      final isActive = tool == activeTool;
      return Tooltip(
        message: label,
        child: InkWell(
          onTap: () => ref.read(tilemapProvider.notifier).setTool(tool),
          borderRadius: BorderRadius.circular(4),
          child: Container(
            width: 36, height: 36,
            decoration: BoxDecoration(
              color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.3) : null,
              borderRadius: BorderRadius.circular(4),
              border: isActive
                  ? Border.all(color: theme.colorScheme.primary, width: 1)
                  : Border.all(color: Colors.transparent),
            ),
            child: Icon(icon, size: 18, color: isActive ? theme.colorScheme.primary : null),
          ),
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('TILEMAP TOOLS', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: [
            btn(Icons.grid_view, 'Stamp (T)', TilemapTool.stamp),
            btn(Icons.auto_fix_high, 'Eraser (E)', TilemapTool.eraser),
            btn(Icons.format_color_fill, 'Fill (G)', TilemapTool.bucket),
            btn(Icons.colorize, 'Eyedropper (I)', TilemapTool.eyedropper),
          ],
        ),
      ],
    );
  }
}

class _TilemapSizeSection extends ConsumerStatefulWidget {
  const _TilemapSizeSection();

  @override
  ConsumerState<_TilemapSizeSection> createState() => _TilemapSizeSectionState();
}

class _TilemapSizeSectionState extends ConsumerState<_TilemapSizeSection> {
  late TextEditingController _wCtrl;
  late TextEditingController _hCtrl;

  @override
  void initState() {
    super.initState();
    final ts = ref.read(tilemapProvider);
    _wCtrl = TextEditingController(text: '${ts.gridWidth}');
    _hCtrl = TextEditingController(text: '${ts.gridHeight}');
  }

  @override
  void dispose() {
    _wCtrl.dispose();
    _hCtrl.dispose();
    super.dispose();
  }

  void _apply() {
    final w = int.tryParse(_wCtrl.text) ?? 12;
    final h = int.tryParse(_hCtrl.text) ?? 8;
    ref.read(tilemapProvider.notifier).resize(
      w.clamp(2, 64),
      h.clamp(2, 64),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final ts = ref.watch(tilemapProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('MAP SIZE', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        Row(
          children: [
            SizedBox(
              width: 50,
              child: TextField(
                controller: _wCtrl,
                keyboardType: TextInputType.number,
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 12),
                decoration: InputDecoration(
                  labelText: 'W',
                  labelStyle: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                  isDense: true,
                  contentPadding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(4)),
                ),
                onSubmitted: (_) => _apply(),
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4),
              child: Text('x', style: theme.textTheme.bodySmall),
            ),
            SizedBox(
              width: 50,
              child: TextField(
                controller: _hCtrl,
                keyboardType: TextInputType.number,
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 12),
                decoration: InputDecoration(
                  labelText: 'H',
                  labelStyle: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                  isDense: true,
                  contentPadding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(4)),
                ),
                onSubmitted: (_) => _apply(),
              ),
            ),
            const SizedBox(width: 6),
            InkWell(
              onTap: _apply,
              borderRadius: BorderRadius.circular(4),
              child: Container(
                padding: const EdgeInsets.all(6),
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(color: theme.dividerColor),
                ),
                child: const Icon(Icons.check, size: 14),
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Text(
          '${ts.gridWidth} x ${ts.gridHeight} tiles',
          style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
        ),
      ],
    );
  }
}

// ── Symmetry ───────────────────────────────────────────────

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
                child: Text(label, style: theme.textTheme.bodySmall!.copyWith(
                  color: isActive ? theme.colorScheme.primary : null, fontSize: 11,
                )),
              ),
            );
          }).toList(),
        ),
      ],
    );
  }
}

// ── Palette ────────────────────────────────────────────────

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
            PopupMenuButton<String>(
              tooltip: 'Switch palette',
              iconSize: 16,
              icon: const Icon(Icons.palette, size: 14),
              onSelected: (name) {
                ref.read(paletteProvider.notifier).selectBuiltIn(name);
                final newPalette = ref.read(paletteProvider);
                ref.read(canvasProvider.notifier).clampColorIndices(newPalette.length);
              },
              itemBuilder: (_) => BuiltInPalettes.all
                  .map((p) => PopupMenuItem(value: p.name, child: Text(p.name, style: const TextStyle(fontSize: 12))))
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
              onTap: () {
                if (HardwareKeyboard.instance.isShiftPressed) {
                  ref.read(canvasProvider.notifier).setBackgroundColor(i);
                } else {
                  ref.read(canvasProvider.notifier).setForegroundColor(i);
                }
              },
              onSecondaryTap: () => ref.read(canvasProvider.notifier).setBackgroundColor(i),
              child: Container(
                width: 22, height: 22,
                decoration: BoxDecoration(
                  color: color,
                  borderRadius: BorderRadius.circular(2),
                  border: Border.all(
                    color: isFg ? Colors.white : isBg ? theme.colorScheme.secondary : theme.dividerColor,
                    width: isFg || isBg ? 2 : 1,
                  ),
                ),
                child: color.a == 0 ? CustomPaint(painter: _TransparentPainter()) : null,
              ),
            );
          }),
        ),
        const SizedBox(height: 4),
        Row(
          children: [
            Container(width: 16, height: 16, color: palette[cs.foregroundColorIndex]),
            const SizedBox(width: 4),
            Expanded(
              child: Text(
                'FG: #${palette[cs.foregroundColorIndex].toARGB32().toRadixString(16).padLeft(8, '0').substring(2)}',
                style: theme.textTheme.bodySmall,
              ),
            ),
            // Add color
            Tooltip(
              message: 'Add color',
              child: InkWell(
                onTap: () => ref.read(paletteProvider.notifier).addColor(
                  palette[cs.foregroundColorIndex],
                ),
                borderRadius: BorderRadius.circular(4),
                child: const Padding(
                  padding: EdgeInsets.all(2),
                  child: Icon(Icons.add, size: 12),
                ),
              ),
            ),
            // Remove selected
            Tooltip(
              message: 'Remove selected color',
              child: InkWell(
                onTap: palette.length > 2
                    ? () {
                        ref.read(paletteProvider.notifier).removeColor(cs.foregroundColorIndex);
                        ref.read(canvasProvider.notifier).clampColorIndices(palette.length - 1);
                      }
                    : null,
                borderRadius: BorderRadius.circular(4),
                child: Padding(
                  padding: const EdgeInsets.all(2),
                  child: Icon(Icons.remove, size: 12,
                    color: palette.length > 2 ? null : theme.disabledColor),
                ),
              ),
            ),
            // Edit hex
            Tooltip(
              message: 'Edit hex color',
              child: InkWell(
                onTap: () => _showHexEditor(context, ref, cs.foregroundColorIndex, palette),
                borderRadius: BorderRadius.circular(4),
                child: const Padding(
                  padding: EdgeInsets.all(2),
                  child: Icon(Icons.edit, size: 12),
                ),
              ),
            ),
          ],
        ),
        Text(
          'Click = FG  |  Shift/Right = BG',
          style: theme.textTheme.bodySmall!.copyWith(fontSize: 8, color: StudioTheme.separatorColor),
        ),
      ],
    );
  }

  static void _showHexEditor(BuildContext context, WidgetRef ref, int index, PixlPalette palette) {
    final color = palette[index];
    final hex = color.toARGB32().toRadixString(16).padLeft(8, '0').substring(2);
    final controller = TextEditingController(text: hex);

    showDialog(
      context: context,
      builder: (ctx) {
        final theme = Theme.of(ctx);
        return Dialog(
          backgroundColor: theme.cardColor,
          child: Container(
            width: 240,
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Edit Color', style: theme.textTheme.bodyMedium!.copyWith(fontWeight: FontWeight.w700)),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Text('#', style: theme.textTheme.bodyMedium),
                    const SizedBox(width: 4),
                    Expanded(
                      child: TextField(
                        controller: controller,
                        style: theme.textTheme.bodyMedium!.copyWith(fontSize: 13),
                        maxLength: 6,
                        decoration: const InputDecoration(
                          isDense: true,
                          counterText: '',
                          contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    TextButton(
                      onPressed: () => Navigator.of(ctx).pop(),
                      child: const Text('Cancel', style: TextStyle(fontSize: 12)),
                    ),
                    const SizedBox(width: 8),
                    ElevatedButton(
                      onPressed: () {
                        final hexVal = int.tryParse(controller.text, radix: 16);
                        if (hexVal != null) {
                          ref.read(paletteProvider.notifier).editColor(
                            index,
                            Color(0xFF000000 | hexVal),
                          );
                        }
                        Navigator.of(ctx).pop();
                      },
                      child: const Text('Apply', style: TextStyle(fontSize: 12)),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _TransparentPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final s = size.width / 2;
    final light = Paint()..color = const Color(0xFFcccccc);
    final dark = Paint()..color = StudioTheme.separatorColor;
    canvas.drawRect(Rect.fromLTWH(0, 0, s, s), light);
    canvas.drawRect(Rect.fromLTWH(s, 0, s, s), dark);
    canvas.drawRect(Rect.fromLTWH(0, s, s, s), dark);
    canvas.drawRect(Rect.fromLTWH(s, s, s, s), light);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

// ── Layers ─────────────────────────────────────────────────

class _LayersSection extends ConsumerWidget {
  const _LayersSection();

  static const _layerRoles = ['background', 'terrain', 'walls', 'platform', 'foreground', 'effects'];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = ref.watch(canvasProvider);
    final notifier = ref.read(canvasProvider.notifier);
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('LAYERS', style: theme.textTheme.titleSmall),
            const Spacer(),
            Tooltip(
              message: 'Add layer',
              child: InkWell(
                onTap: () {
                  final n = cs.layers.length + 1;
                  notifier.addLayer('Layer $n');
                },
                borderRadius: BorderRadius.circular(4),
                child: const Padding(
                  padding: EdgeInsets.all(2),
                  child: Icon(Icons.add, size: 14),
                ),
              ),
            ),
            Tooltip(
              message: 'Remove active layer',
              child: InkWell(
                onTap: cs.layers.length > 1
                    ? () => notifier.removeLayer(cs.activeLayerIndex)
                    : null,
                borderRadius: BorderRadius.circular(4),
                child: Padding(
                  padding: const EdgeInsets.all(2),
                  child: Icon(Icons.remove, size: 14,
                    color: cs.layers.length > 1 ? null : theme.disabledColor),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        ...List.generate(cs.layers.length, (i) {
          final layer = cs.layers[i];
          final isActive = i == cs.activeLayerIndex;
          return InkWell(
            onTap: () => notifier.setActiveLayer(i),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
              margin: const EdgeInsets.only(bottom: 2),
              decoration: BoxDecoration(
                color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.15) : null,
                borderRadius: BorderRadius.circular(3),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      InkWell(
                        onTap: () => notifier.toggleLayerVisibility(i),
                        child: Icon(
                          layer.visible ? Icons.visibility : Icons.visibility_off,
                          size: 14,
                          color: layer.visible ? null : theme.disabledColor,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Text(layer.name, style: theme.textTheme.bodySmall!.copyWith(
                          color: isActive ? theme.colorScheme.primary : null,
                        )),
                      ),
                      // Reorder buttons
                      if (isActive) ...[
                        InkWell(
                          onTap: i > 0 ? () => notifier.moveLayerUp(i) : null,
                          child: Icon(Icons.arrow_upward, size: 10,
                            color: i > 0 ? null : theme.disabledColor),
                        ),
                        InkWell(
                          onTap: i < cs.layers.length - 1
                              ? () => notifier.moveLayerDown(i)
                              : null,
                          child: Icon(Icons.arrow_downward, size: 10,
                            color: i < cs.layers.length - 1 ? null : theme.disabledColor),
                        ),
                      ],
                    ],
                  ),
                  // Layer properties (only for active layer)
                  if (isActive) ...[
                    const SizedBox(height: 4),
                    // Opacity slider
                    Row(
                      children: [
                        Text('Opacity', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                        Expanded(
                          child: SliderTheme(
                            data: SliderThemeData(
                              thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 5),
                              trackHeight: 2,
                              activeTrackColor: theme.colorScheme.primary,
                              thumbColor: theme.colorScheme.primary,
                              inactiveTrackColor: theme.dividerColor,
                            ),
                            child: Slider(
                              value: layer.opacity,
                              onChanged: (v) => notifier.setLayerOpacity(i, v),
                            ),
                          ),
                        ),
                        SizedBox(
                          width: 28,
                          child: Text(
                            '${(layer.opacity * 100).round()}%',
                            style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                          ),
                        ),
                      ],
                    ),
                    // Blend mode + target layer row
                    Row(
                      children: [
                        // Blend mode
                        Expanded(
                          child: PopupMenuButton<BlendMode>(
                            tooltip: 'Blend mode',
                            initialValue: layer.blendMode,
                            onSelected: (mode) => notifier.setLayerBlendMode(i, mode),
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                              decoration: BoxDecoration(
                                borderRadius: BorderRadius.circular(3),
                                border: Border.all(color: theme.dividerColor),
                              ),
                              child: Text(
                                layer.blendMode.name,
                                style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                              ),
                            ),
                            itemBuilder: (_) => BlendMode.values
                                .map((m) => PopupMenuItem(
                                      value: m,
                                      child: Text(m.name, style: const TextStyle(fontSize: 11)),
                                    ))
                                .toList(),
                          ),
                        ),
                        const SizedBox(width: 4),
                        // Target layer
                        Expanded(
                          child: PopupMenuButton<String>(
                            tooltip: 'Target tilemap layer',
                            onSelected: (tl) => notifier.setLayerTargetLayer(
                              i,
                              tl == 'none' ? null : tl,
                            ),
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                              decoration: BoxDecoration(
                                borderRadius: BorderRadius.circular(3),
                                border: Border.all(color: theme.dividerColor),
                              ),
                              child: Text(
                                layer.targetLayer ?? 'layer...',
                                style: theme.textTheme.bodySmall!.copyWith(
                                  fontSize: 9,
                                  color: layer.targetLayer != null ? null : theme.disabledColor,
                                ),
                              ),
                            ),
                            itemBuilder: (_) => [
                              const PopupMenuItem(
                                value: 'none',
                                child: Text('(none)', style: TextStyle(fontSize: 11)),
                              ),
                              ..._layerRoles.map((r) => PopupMenuItem(
                                    value: r,
                                    child: Text(r, style: const TextStyle(fontSize: 11)),
                                  )),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
          );
        }),
      ],
    );
  }
}

// ── Canvas Size ────────────────────────────────────────────

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
              onTap: () => ref.read(canvasProvider.notifier).setCanvasSize(size),
              borderRadius: BorderRadius.circular(4),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
                decoration: BoxDecoration(
                  color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.3) : null,
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: isActive ? theme.colorScheme.primary : theme.dividerColor,
                  ),
                ),
                child: Text(size.label, style: theme.textTheme.bodySmall!.copyWith(
                  fontSize: 10,
                  color: isActive ? theme.colorScheme.primary : null,
                )),
              ),
            );
          }).toList(),
        ),
      ],
    );
  }
}

// ── Style System ───────────────────────────────────────────

class _StyleSection extends ConsumerWidget {
  const _StyleSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final style = ref.watch(styleProvider);
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('STYLE', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),

        // Theme selector
        Text('Theme', style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
        const SizedBox(height: 4),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: AvailableThemes.themes.map((entry) {
            final (id, label) = entry;
            final isActive = style.theme == id;
            return _Chip(
              label: label,
              active: isActive,
              onTap: () => ref.read(styleProvider.notifier).setTheme(id),
            );
          }).toList(),
        ),
        const SizedBox(height: 8),

        // Mood chips
        Text('Mood', style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
        const SizedBox(height: 4),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: Mood.values.map((m) => _Chip(
            label: m.name,
            active: style.mood == m,
            onTap: () => ref.read(styleProvider.notifier).setMood(m),
          )).toList(),
        ),
        const SizedBox(height: 8),

        // Outline chips
        Text('Outline', style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
        const SizedBox(height: 4),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: OutlineStyle.values.map((o) {
            final label = switch (o) {
              OutlineStyle.none => 'none',
              OutlineStyle.selfOutline => 'self',
              OutlineStyle.dropShadow => 'shadow',
              OutlineStyle.selective => 'selective',
            };
            return _Chip(
              label: label,
              active: style.outline == o,
              onTap: () => ref.read(styleProvider.notifier).setOutline(o),
            );
          }).toList(),
        ),
        const SizedBox(height: 8),

        // Dithering chips
        Text('Dithering', style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
        const SizedBox(height: 4),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: Dithering.values.map((d) => _Chip(
            label: d.name,
            active: style.dithering == d,
            onTap: () => ref.read(styleProvider.notifier).setDithering(d),
          )).toList(),
        ),

        // Style summary
        const SizedBox(height: 8),
        Container(
          padding: const EdgeInsets.all(6),
          decoration: BoxDecoration(
            color: StudioTheme.recessedBg,
            borderRadius: BorderRadius.circular(4),
          ),
          child: Text(
            style.toPromptFragment(),
            style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
          ),
        ),
      ],
    );
  }
}

class _Chip extends StatelessWidget {
  const _Chip({required this.label, required this.active, required this.onTap});
  final String label;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
        decoration: BoxDecoration(
          color: active ? theme.colorScheme.primary.withValues(alpha: 0.25) : null,
          borderRadius: BorderRadius.circular(4),
          border: Border.all(
            color: active ? theme.colorScheme.primary : theme.dividerColor,
          ),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 10,
            color: active ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color,
          ),
        ),
      ),
    );
  }
}

// ── Quick Generate ─────────────────────────────────────────

class _QuickGenerateSection extends ConsumerStatefulWidget {
  const _QuickGenerateSection();

  @override
  ConsumerState<_QuickGenerateSection> createState() => _QuickGenerateSectionState();
}

class _QuickGenerateSectionState extends ConsumerState<_QuickGenerateSection> {
  final _controller = TextEditingController();
  bool _generating = false;

  Future<void> _generate() async {
    final prompt = _controller.text.trim();
    if (prompt.isEmpty) return;

    final backend = ref.read(backendProvider);
    final claude = ref.read(claudeProvider);
    if (!backend.isConnected || !claude.hasApiKey) return;

    setState(() => _generating = true);
    final chat = ref.read(chatProvider.notifier);
    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';
    final style = ref.read(styleProvider);

    chat.addUserMessage(prompt);

    // Get enriched context
    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: prompt,
      size: sizeStr,
    );
    final backendCtx = ctx['system_prompt'] as String? ?? '';
    final userPrompt = ctx['user_prompt'] as String? ?? prompt;
    final systemPrompt = await KnowledgeBase.buildSystemPrompt(
      backendContext: backendCtx,
      styleFragment: style.toPromptFragment(),
    );

    final resp = await ref.read(claudeProvider.notifier).generateTile(
      systemPrompt: systemPrompt,
      userPrompt: userPrompt,
    );

    if (!resp.isError) {
      final grid = extractGrid(resp.content);
      if (grid != null) {
        final tileName = generateTileName(prompt);
        await ref.read(backendProvider.notifier).createTile(
          name: tileName,
          palette: ctx['palette'] as String? ?? 'default',
          size: sizeStr,
          grid: grid,
        );
        chat.addAssistantMessage('Generated **`$tileName`** ($sizeStr)');
      } else {
        chat.addAssistantMessage('Could not extract grid from response.');
      }
    } else {
      chat.addAssistantMessage('Error: ${resp.errorMessage}');
    }

    _controller.clear();
    setState(() => _generating = false);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final backend = ref.watch(backendProvider);
    final claude = ref.watch(claudeProvider);
    final enabled = backend.isConnected && claude.hasApiKey && !_generating;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('GENERATE', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        TextField(
          controller: _controller,
          enabled: enabled,
          style: theme.textTheme.bodyMedium!.copyWith(fontSize: 11),
          maxLines: 2,
          minLines: 1,
          decoration: InputDecoration(
            hintText: enabled ? 'wall tile, moss...' : 'Connect engine + API key',
            hintStyle: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
            isDense: true,
            contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(4),
              borderSide: StudioTheme.panelBorder,
            ),
            focusedBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(4),
              borderSide: BorderSide(color: theme.colorScheme.primary),
            ),
          ),
          onSubmitted: (_) => _generate(),
        ),
        const SizedBox(height: 4),
        SizedBox(
          width: double.infinity,
          child: InkWell(
            onTap: enabled ? _generate : null,
            borderRadius: BorderRadius.circular(4),
            child: Container(
              padding: const EdgeInsets.symmetric(vertical: 6),
              decoration: BoxDecoration(
                color: enabled
                    ? theme.colorScheme.primary.withValues(alpha: 0.2)
                    : null,
                borderRadius: BorderRadius.circular(4),
                border: Border.all(
                  color: enabled ? theme.colorScheme.primary : theme.dividerColor,
                ),
              ),
              child: Center(
                child: _generating
                    ? const SizedBox(
                        width: 12, height: 12,
                        child: CircularProgressIndicator(strokeWidth: 1.5),
                      )
                    : Text(
                        'Generate Tile',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w700,
                          color: enabled ? theme.colorScheme.primary : theme.disabledColor,
                        ),
                      ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

// ── Backend Connection ─────────────────────────────────────

class _BackendSection extends ConsumerWidget {
  const _BackendSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final backend = ref.watch(backendProvider);
    final theme = Theme.of(context);

    final (icon, color, label) = switch (backend.status) {
      BackendStatus.disconnected => (Icons.circle_outlined, StudioTheme.separatorColor, 'Disconnected'),
      BackendStatus.connecting => (Icons.sync, StudioTheme.warning, 'Connecting...'),
      BackendStatus.connected => (Icons.check_circle, StudioTheme.success, 'Connected'),
      BackendStatus.error => (Icons.error, StudioTheme.error, 'Error'),
    };

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('ENGINE', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        Row(
          children: [
            Icon(icon, size: 12, color: color),
            const SizedBox(width: 6),
            Expanded(
              child: Text(label, style: theme.textTheme.bodySmall!.copyWith(color: color)),
            ),
            if (backend.status == BackendStatus.disconnected ||
                backend.status == BackendStatus.error)
              InkWell(
                onTap: () {
                  final service = ref.read(claudeProvider.notifier).service;
                  final isLocal = service.provider == LlmProviderType.pixlLocal;
                  ref.read(backendProvider.notifier).connect(
                    model: isLocal ? service.pixlModel : null,
                    adapter: isLocal && service.hasPixlAdapter ? service.pixlAdapter : null,
                  );
                },
                borderRadius: BorderRadius.circular(4),
                child: const Padding(
                  padding: EdgeInsets.all(4),
                  child: Icon(Icons.refresh, size: 14),
                ),
              ),
          ],
        ),
        if (backend.errorMessage != null)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(
              backend.errorMessage!,
              style: theme.textTheme.bodySmall!.copyWith(
                color: StudioTheme.error,
                fontSize: 10,
              ),
            ),
          ),
        // Actionable guidance when not connected
        if (backend.status == BackendStatus.disconnected ||
            backend.status == BackendStatus.error)
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Container(
              padding: const EdgeInsets.all(6),
              decoration: BoxDecoration(
                color: StudioTheme.recessedBg,
                borderRadius: BorderRadius.circular(4),
                border: Border.all(color: StudioTheme.separatorColor),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Open a .pax file to auto-start the engine, or run manually:',
                    style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                  ),
                  const SizedBox(height: 4),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
                    decoration: BoxDecoration(
                      color: StudioTheme.codeBg,
                      borderRadius: BorderRadius.circular(3),
                    ),
                    child: SelectableText(
                      'cd tool && cargo run -- serve --port 3742',
                      style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                    ),
                  ),
                ],
              ),
            ),
          ),
        if (backend.sessionTheme != null)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(
              'Theme: ${backend.sessionTheme}',
              style: theme.textTheme.bodySmall,
            ),
          ),
      ],
    );
  }
}

// ── Validation ─────────────────────────────────────────────

class _ValidationSection extends ConsumerStatefulWidget {
  const _ValidationSection();

  @override
  ConsumerState<_ValidationSection> createState() => _ValidationSectionState();
}

class _ValidationSectionState extends ConsumerState<_ValidationSection> {
  ValidationReport? _report;
  bool _loading = false;

  Future<void> _runValidation() async {
    if (!ref.read(backendProvider).isConnected) return;
    setState(() => _loading = true);
    final report = await ref.read(backendProvider.notifier).validate(checkEdges: true);
    setState(() {
      _report = report;
      _loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final backend = ref.watch(backendProvider);
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('VALIDATION', style: theme.textTheme.titleSmall),
            const Spacer(),
            InkWell(
              onTap: backend.isConnected && !_loading ? _runValidation : null,
              borderRadius: BorderRadius.circular(4),
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: _loading
                    ? const SizedBox(
                        width: 12, height: 12,
                        child: CircularProgressIndicator(strokeWidth: 1.5),
                      )
                    : Icon(
                        Icons.play_arrow,
                        size: 14,
                        color: backend.isConnected ? null : theme.disabledColor,
                      ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        if (!backend.isConnected)
          Text('Connect engine to validate', style: theme.textTheme.bodySmall)
        else if (_report == null)
          Text('Press play to validate', style: theme.textTheme.bodySmall)
        else ...[
          _CheckRow(label: 'Valid', passed: _report!.valid),
          if (_report!.edgeCompat != null)
            _CheckRow(label: 'Edge compatibility', passed: _report!.edgeCompat!),
          if (_report!.paletteCompliant != null)
            _CheckRow(label: 'Palette compliance', passed: _report!.paletteCompliant!),
          if (_report!.sizeCorrect != null)
            _CheckRow(label: 'Size correct', passed: _report!.sizeCorrect!),
          for (final err in _report!.errors)
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Text(err, style: theme.textTheme.bodySmall!.copyWith(
                color: StudioTheme.error, fontSize: 10,
              )),
            ),
          for (final warn in _report!.warnings)
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Text(warn, style: theme.textTheme.bodySmall!.copyWith(
                color: StudioTheme.warning, fontSize: 10,
              )),
            ),
        ],
      ],
    );
  }
}

class _CheckRow extends StatelessWidget {
  const _CheckRow({required this.label, required this.passed});
  final String label;
  final bool passed;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 2),
      child: Row(
        children: [
          Icon(
            passed ? Icons.check_circle : Icons.cancel,
            size: 12,
            color: passed ? StudioTheme.success : StudioTheme.error,
          ),
          const SizedBox(width: 6),
          Text(label, style: Theme.of(context).textTheme.bodySmall!.copyWith(fontSize: 11)),
        ],
      ),
    );
  }
}

// ── Tile List ──────────────────────────────────────────────

class _TileListSection extends ConsumerStatefulWidget {
  const _TileListSection();

  @override
  ConsumerState<_TileListSection> createState() => _TileListSectionState();
}

class _TileListSectionState extends ConsumerState<_TileListSection> {
  String? _selectedTile;
  Uint8List? _previewBytes;
  bool _loadingPreview = false;

  Future<void> _selectTile(String name) async {
    setState(() {
      _selectedTile = name;
      _previewBytes = null;
      _loadingPreview = true;
    });

    final b64 = await ref.read(backendProvider.notifier).renderTile(name);
    if (b64 != null && mounted) {
      setState(() {
        _previewBytes = base64Decode(b64);
        _loadingPreview = false;
      });
    } else if (mounted) {
      setState(() => _loadingPreview = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final backend = ref.watch(backendProvider);
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('TILES', style: theme.textTheme.titleSmall),
            const Spacer(),
            if (backend.isConnected)
              InkWell(
                onTap: () => ref.read(backendProvider.notifier).refreshTiles(),
                borderRadius: BorderRadius.circular(4),
                child: const Padding(
                  padding: EdgeInsets.all(4),
                  child: Icon(Icons.refresh, size: 14),
                ),
              ),
          ],
        ),
        const SizedBox(height: 6),
        if (!backend.isConnected)
          Text('Connect engine to see tiles', style: theme.textTheme.bodySmall)
        else if (backend.tiles.isEmpty)
          Text('No tiles in session', style: theme.textTheme.bodySmall)
        else
          ...backend.tiles.map((tile) {
            final isSelected = tile.name == _selectedTile;
            return InkWell(
              onTap: () => _selectTile(tile.name),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
                margin: const EdgeInsets.only(bottom: 2),
                decoration: BoxDecoration(
                  color: isSelected ? theme.colorScheme.primary.withValues(alpha: 0.15) : null,
                  borderRadius: BorderRadius.circular(3),
                ),
                child: Row(
                  children: [
                    // Inline preview thumbnail
                    if (tile.previewBytes != null)
                      Padding(
                        padding: const EdgeInsets.only(right: 6),
                        child: Image.memory(
                          tile.previewBytes!,
                          width: 20, height: 20,
                          filterQuality: FilterQuality.none,
                        ),
                      ),
                    Expanded(
                      child: Text(
                        tile.name,
                        style: theme.textTheme.bodySmall!.copyWith(
                          color: isSelected ? theme.colorScheme.primary : null,
                          fontSize: 11,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    if (tile.size != null)
                      Text(tile.size!, style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                  ],
                ),
              ),
            );
          }),

        // Render preview for selected tile
        if (_selectedTile != null) ...[
          const SizedBox(height: 8),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: StudioTheme.canvasBg,
              borderRadius: BorderRadius.circular(4),
              border: Border.all(color: theme.dividerColor),
            ),
            child: Column(
              children: [
                Text(
                  _selectedTile!,
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                ),
                const SizedBox(height: 4),
                if (_loadingPreview)
                  const SizedBox(
                    width: 24, height: 24,
                    child: CircularProgressIndicator(strokeWidth: 1.5),
                  )
                else if (_previewBytes != null)
                  Image.memory(
                    _previewBytes!,
                    width: 128, height: 128,
                    filterQuality: FilterQuality.none,
                    fit: BoxFit.contain,
                  )
                else
                  Text('No preview', style: theme.textTheme.bodySmall),
              ],
            ),
          ),
        ],
      ],
    );
  }
}
