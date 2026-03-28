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
import '../color_picker_dialog.dart';
import '../sprite_preview_dialog.dart';
import 'backdrop_panel.dart';

enum _PanelTab { palette, style, generate, tiles }

/// Right panel — tabbed layout with vertical icon tab bar.
class ToolsPanel extends ConsumerStatefulWidget {
  const ToolsPanel({super.key});

  @override
  ConsumerState<ToolsPanel> createState() => _ToolsPanelState();
}

class _ToolsPanelState extends ConsumerState<ToolsPanel> {
  _PanelTab _activeTab = _PanelTab.palette;

  @override
  Widget build(BuildContext context) {
    final mode = ref.watch(editorModeProvider);
    final isTilemap = mode == EditorMode.tilemap;
    final isBackdrop = mode == EditorMode.backdrop;
    final theme = Theme.of(context);

    // Backdrop mode: show dedicated panel instead of tabs
    if (isBackdrop) {
      return Container(
        width: 220,
        decoration: StudioTheme.rightPanelDecoration,
        child: const BackdropPanel(),
      );
    }

    return Container(
      width: 220,
      decoration: StudioTheme.rightPanelDecoration,
      child: Row(
        children: [
          // Vertical tab bar (icon-only)
          Container(
            width: 32,
            decoration: BoxDecoration(
              border: Border(right: BorderSide(color: theme.dividerColor, width: 0.5)),
            ),
            child: Column(
              children: [
                const SizedBox(height: 4),
                _TabIcon(Icons.palette, 'Palette', _PanelTab.palette,
                    _activeTab, (t) => setState(() => _activeTab = t)),
                _TabIcon(Icons.brush, 'Style', _PanelTab.style,
                    _activeTab, (t) => setState(() => _activeTab = t)),
                _TabIcon(Icons.auto_awesome, 'Generate', _PanelTab.generate,
                    _activeTab, (t) => setState(() => _activeTab = t)),
                _TabIcon(Icons.grid_view, 'Tiles', _PanelTab.tiles,
                    _activeTab, (t) => setState(() => _activeTab = t)),
              ],
            ),
          ),
          // Tab content
          Expanded(
            child: SingleChildScrollView(
              padding: StudioTheme.panelPadding,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: _tabContent(isTilemap),
              ),
            ),
          ),
        ],
      ),
    );
  }

  List<Widget> _tabContent(bool isTilemap) {
    const gap = SizedBox(height: StudioTheme.sectionSpacing);
    return switch (_activeTab) {
      _PanelTab.palette => [
        if (isTilemap) ...[
          const _TilemapSizeSection(),
        ] else ...[
          const _PaletteSection(),
          gap,
          const _LayersSection(),
          gap,
          const _CanvasSizeSection(),
        ],
      ],
      _PanelTab.style => [
        const _StyleSection(),
      ],
      _PanelTab.generate => [
        const _QuickGenerateSection(),
        gap,
        const _BackendSection(),
      ],
      _PanelTab.tiles => [
        const _TileListSection(),
        gap,
        const _ValidationSection(),
      ],
    };
  }
}

class _TabIcon extends StatelessWidget {
  const _TabIcon(this.icon, this.tooltip, this.tab, this.active, this.onTap);

  final IconData icon;
  final String tooltip;
  final _PanelTab tab;
  final _PanelTab active;
  final ValueChanged<_PanelTab> onTap;

  @override
  Widget build(BuildContext context) {
    final isActive = tab == active;
    final theme = Theme.of(context);
    return Tooltip(
      message: tooltip,
      preferBelow: false,
      child: InkWell(
        onTap: () => onTap(tab),
        borderRadius: BorderRadius.circular(4),
        child: Container(
          width: 28,
          height: 28,
          margin: const EdgeInsets.symmetric(vertical: 2),
          decoration: BoxDecoration(
            color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.2) : null,
            borderRadius: BorderRadius.circular(4),
          ),
          child: Icon(icon, size: 15,
            color: isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color),
        ),
      ),
    );
  }
}

// ── Tilemap Size ──────────────────────────────────────────

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
                onTap: () async {
                  final color = await ColorPickerDialog.show(context, palette[cs.foregroundColorIndex]);
                  if (color != null) {
                    ref.read(paletteProvider.notifier).editColor(cs.foregroundColorIndex, color);
                  }
                },
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
    final systemPrompt = KnowledgeBase.buildSystemPrompt(
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

        // Stamps
        if (backend.isConnected) ...[
          const SizedBox(height: 12),
          Text('STAMPS', style: theme.textTheme.titleSmall),
          const SizedBox(height: 4),
          if (backend.stamps.isNotEmpty)
            Wrap(
              spacing: 4,
              runSpacing: 4,
              children: backend.stamps.map((name) {
                return Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(3),
                    border: Border.all(color: theme.dividerColor),
                  ),
                  child: Text(name, style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                );
              }).toList(),
            ),
          const SizedBox(height: 6),
          Text('Procedural patterns:', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
          const SizedBox(height: 4),
          Wrap(
            spacing: 4,
            runSpacing: 4,
            children: const [
              'brick_bond', 'checkerboard', 'diagonal', 'dither_bayer',
              'horizontal_stripe', 'dots', 'cross', 'noise',
            ].map((pattern) {
              return InkWell(
                onTap: () async {
                  // Generate stamp via backend CLI tool
                  final resp = await ref.read(backendProvider.notifier).backend.callTool(
                    'pixl_generate_stamps',
                    {'pattern': pattern, 'size': 4, 'fg': '#', 'bg': '+'},
                  );
                  if (resp.containsKey('error')) return;
                  ref.read(backendProvider.notifier).refreshTiles();
                },
                borderRadius: BorderRadius.circular(3),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(3),
                    color: theme.colorScheme.primary.withValues(alpha: 0.08),
                    border: Border.all(color: theme.colorScheme.primary.withValues(alpha: 0.3)),
                  ),
                  child: Text(pattern, style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 8, color: theme.colorScheme.primary,
                  )),
                ),
              );
            }).toList(),
          ),
        ],

        // Edge compatibility checker
        if (backend.isConnected && backend.tiles.length >= 2) ...[
          const SizedBox(height: 12),
          _EdgeChecker(tiles: backend.tiles),
        ],

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
                else if (_previewBytes != null) ...[
                  Image.memory(
                    _previewBytes!,
                    width: 96, height: 96,
                    filterQuality: FilterQuality.none,
                    fit: BoxFit.contain,
                  ),
                  const SizedBox(height: 4),
                  // 3x3 tiling preview
                  Text('Tiling:', style: theme.textTheme.bodySmall!.copyWith(fontSize: 8)),
                  const SizedBox(height: 2),
                  SizedBox(
                    width: 96,
                    height: 96,
                    child: GridView.count(
                      crossAxisCount: 3,
                      physics: const NeverScrollableScrollPhysics(),
                      padding: EdgeInsets.zero,
                      children: List.generate(9, (_) => Image.memory(
                        _previewBytes!,
                        filterQuality: FilterQuality.none,
                        fit: BoxFit.fill,
                      )),
                    ),
                  ),
                ]
                else
                  Text('No preview', style: theme.textTheme.bodySmall),
                const SizedBox(height: 4),
                // Edge classes + animation
                Builder(builder: (_) {
                  final tile = backend.tiles.where((t) => t.name == _selectedTile).firstOrNull;
                  if (tile == null) return const SizedBox.shrink();
                  return Column(
                    children: [
                      if (tile.edgeClasses != null)
                        Text(
                          'N:${tile.edgeClasses!['n'] ?? '?'} E:${tile.edgeClasses!['e'] ?? '?'} '
                          'S:${tile.edgeClasses!['s'] ?? '?'} W:${tile.edgeClasses!['w'] ?? '?'}',
                          style: theme.textTheme.bodySmall!.copyWith(fontSize: 8),
                        ),
                      if (tile.tags.isNotEmpty)
                        Padding(
                          padding: const EdgeInsets.only(top: 2),
                          child: Wrap(
                            spacing: 3,
                            children: tile.tags.map((t) => Text(
                              t,
                              style: theme.textTheme.bodySmall!.copyWith(fontSize: 7, color: theme.colorScheme.primary),
                            )).toList(),
                          ),
                        ),
                      // Animation preview button (for spriteset tiles)
                      if (tile.tags.any((t) => t.contains('sprite') || t.contains('anim')))
                        Padding(
                          padding: const EdgeInsets.only(top: 4),
                          child: InkWell(
                            onTap: () => SpritePreviewDialog.show(
                              context,
                              spriteset: tile.name,
                              sprite: 'default',
                            ),
                            borderRadius: BorderRadius.circular(4),
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                              decoration: BoxDecoration(
                                borderRadius: BorderRadius.circular(4),
                                border: Border.all(color: theme.dividerColor),
                              ),
                              child: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Icon(Icons.play_arrow, size: 12, color: theme.colorScheme.primary),
                                  const SizedBox(width: 4),
                                  Text('Play Animation', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                                ],
                              ),
                            ),
                          ),
                        ),
                    ],
                  );
                }),
              ],
            ),
          ),
        ],
      ],
    );
  }
}

// ── Edge Compatibility Checker ────────────────────────────

class _EdgeChecker extends ConsumerStatefulWidget {
  const _EdgeChecker({required this.tiles});
  final List<TileInfo> tiles;

  @override
  ConsumerState<_EdgeChecker> createState() => _EdgeCheckerState();
}

class _EdgeCheckerState extends ConsumerState<_EdgeChecker> {
  String? _tileA;
  String? _tileB;
  String _direction = 'east';
  Map<String, dynamic>? _result;
  bool _checking = false;

  Future<void> _check() async {
    if (_tileA == null || _tileB == null) return;
    setState(() {
      _checking = true;
      _result = null;
    });
    final resp = await ref.read(backendProvider.notifier).backend.checkEdgePair(
      _tileA!,
      _direction,
      _tileB!,
    );
    if (mounted) {
      setState(() {
        _result = resp;
        _checking = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final names = widget.tiles.map((t) => t.name).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('EDGE CHECK', style: theme.textTheme.titleSmall),
        const SizedBox(height: 4),
        Row(
          children: [
            Expanded(
              child: DropdownButton<String>(
                value: _tileA,
                hint: Text('Tile A', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                isExpanded: true,
                isDense: true,
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                items: names.map((n) => DropdownMenuItem(value: n, child: Text(n))).toList(),
                onChanged: (v) => setState(() => _tileA = v),
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 2),
              child: DropdownButton<String>(
                value: _direction,
                isDense: true,
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                items: ['north', 'east', 'south', 'west']
                    .map((d) => DropdownMenuItem(value: d, child: Text(d[0].toUpperCase())))
                    .toList(),
                onChanged: (v) => setState(() => _direction = v ?? 'east'),
              ),
            ),
            Expanded(
              child: DropdownButton<String>(
                value: _tileB,
                hint: Text('Tile B', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                isExpanded: true,
                isDense: true,
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                items: names.map((n) => DropdownMenuItem(value: n, child: Text(n))).toList(),
                onChanged: (v) => setState(() => _tileB = v),
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Row(
          children: [
            InkWell(
              onTap: (_tileA != null && _tileB != null && !_checking) ? _check : null,
              borderRadius: BorderRadius.circular(4),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(color: theme.dividerColor),
                ),
                child: _checking
                    ? const SizedBox(width: 10, height: 10, child: CircularProgressIndicator(strokeWidth: 1))
                    : Text('Check', style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
              ),
            ),
            const SizedBox(width: 8),
            if (_result != null)
              Icon(
                _result!['compatible'] == true ? Icons.check_circle : Icons.cancel,
                size: 14,
                color: _result!['compatible'] == true ? const Color(0xFF4caf50) : const Color(0xFFe05555),
              ),
            if (_result != null)
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.only(left: 4),
                  child: Text(
                    _result!['reason'] as String? ?? (_result!['compatible'] == true ? 'Compatible' : 'Incompatible'),
                    style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
          ],
        ),
      ],
    );
  }
}
