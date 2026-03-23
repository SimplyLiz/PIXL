import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/palette.dart';
import '../../models/pixel_canvas.dart';
import '../../providers/backend_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/palette_provider.dart';
import '../../theme/studio_theme.dart';

/// Right panel — tools, palette, layers, tile info, validation.
class ToolsPanel extends ConsumerWidget {
  const ToolsPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      width: 220,
      decoration: StudioTheme.rightPanelDecoration,
      child: const SingleChildScrollView(
        padding: StudioTheme.panelPadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _ToolsSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _SymmetrySection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _PaletteSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _LayersSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _CanvasSizeSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _BackendSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _ValidationSection(),
            SizedBox(height: StudioTheme.sectionSpacing),
            _TileListSection(),
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
              onSelected: (name) => ref.read(paletteProvider.notifier).selectBuiltIn(name),
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
              onTap: () => ref.read(canvasProvider.notifier).setForegroundColor(i),
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
            Text(
              'FG: #${palette[cs.foregroundColorIndex].toARGB32().toRadixString(16).padLeft(8, '0').substring(2)}',
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

// ── Layers ─────────────────────────────────────────────────

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
                color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.15) : null,
                borderRadius: BorderRadius.circular(3),
              ),
              child: Row(
                children: [
                  InkWell(
                    onTap: () => ref.read(canvasProvider.notifier).toggleLayerVisibility(i),
                    child: Icon(
                      layer.visible ? Icons.visibility : Icons.visibility_off,
                      size: 14,
                      color: layer.visible ? null : theme.disabledColor,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text(layer.name, style: theme.textTheme.bodySmall!.copyWith(
                    color: isActive ? theme.colorScheme.primary : null,
                  )),
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

// ── Backend Connection ─────────────────────────────────────

class _BackendSection extends ConsumerWidget {
  const _BackendSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final backend = ref.watch(backendProvider);
    final theme = Theme.of(context);

    final (icon, color, label) = switch (backend.status) {
      BackendStatus.disconnected => (Icons.circle_outlined, const Color(0xFF888888), 'Disconnected'),
      BackendStatus.connecting => (Icons.sync, const Color(0xFFffaa00), 'Connecting...'),
      BackendStatus.connected => (Icons.check_circle, const Color(0xFF4caf50), 'Connected'),
      BackendStatus.error => (Icons.error, const Color(0xFFf44336), 'Error'),
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
                onTap: () => ref.read(backendProvider.notifier).connect(),
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
                color: const Color(0xFFf44336),
                fontSize: 10,
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
                color: const Color(0xFFf44336), fontSize: 10,
              )),
            ),
          for (final warn in _report!.warnings)
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Text(warn, style: theme.textTheme.bodySmall!.copyWith(
                color: const Color(0xFFffaa00), fontSize: 10,
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
            color: passed ? const Color(0xFF4caf50) : const Color(0xFFf44336),
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
              color: const Color(0xFF121220),
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
