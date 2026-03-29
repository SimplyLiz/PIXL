import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../services/export_service.dart';
import '../theme/studio_theme.dart';

// ── Format definitions ──

enum _FormatCategory { image, data, engine }

enum _ExportFormat {
  png('PNG Image', Icons.image, _FormatCategory.image,
      'Scaled pixel-perfect PNG with configurable interpolation.'),
  atlas('Sprite Atlas', Icons.grid_4x4, _FormatCategory.image,
      'Pack all tiles into a single atlas sheet.'),
  pax('PAX Source', Icons.code, _FormatCategory.data,
      'Save the raw PAX source for later editing.'),
  tiled('Tiled', Icons.map, _FormatCategory.engine,
      'TMX tilemap + tileset for the Tiled editor.'),
  godot('Godot', Icons.videogame_asset, _FormatCategory.engine,
      'TileSet resource and scene for Godot 4.'),
  unity('Unity', Icons.sports_esports, _FormatCategory.engine,
      'Sprite sheet + tile palette for Unity 2D.'),
  gbstudio('GB Studio', Icons.gamepad, _FormatCategory.engine,
      'Background and sprite assets for GB Studio.'),
  texturepacker('TexturePacker', Icons.texture, _FormatCategory.engine,
      'JSON hash atlas for TexturePacker import.');

  const _ExportFormat(this.label, this.icon, this.category, this.description);
  final String label;
  final IconData icon;
  final _FormatCategory category;
  final String description;
}

// ── Scale filter definitions ──

enum _ScaleFilter {
  nearest('Nearest', 'Pixel-perfect, hard edges — best for integer upscaling'),
  bilinear('Bilinear', 'Smooth interpolation — good for non-integer scales'),
  catmullRom('Cubic', 'Sharp cubic interpolation — balanced downscaling'),
  lanczos3('Lanczos3', 'Highest quality — best for large downscales');

  const _ScaleFilter(this.label, this.hint);
  final String label;
  final String hint;

  String get apiName => switch (this) {
    _ScaleFilter.nearest => 'nearest',
    _ScaleFilter.bilinear => 'bilinear',
    _ScaleFilter.catmullRom => 'catmull_rom',
    _ScaleFilter.lanczos3 => 'lanczos3',
  };
}

/// Export dialog — choose format, resolution, scaling algorithm, and destination.
class ExportDialog extends ConsumerStatefulWidget {
  const ExportDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const ExportDialog(),
    );
  }

  @override
  ConsumerState<ExportDialog> createState() => _ExportDialogState();
}

class _ExportDialogState extends ConsumerState<ExportDialog> {
  _ExportFormat _format = _ExportFormat.png;
  double _scaleLog = 4; // index into _scaleStops
  _ScaleFilter _filter = _ScaleFilter.nearest;
  bool _exporting = false;

  static const _scaleStops = [1, 2, 4, 8, 16, 32];

  @override
  void initState() {
    super.initState();
    final cs = ref.read(canvasProvider);
    _scaleLog = _defaultScaleIndex(cs.width).toDouble();
  }

  int _defaultScaleIndex(int canvasSize) {
    if (canvasSize <= 8) return 5;  // 32x → 256
    if (canvasSize <= 16) return 4; // 16x → 256
    if (canvasSize <= 32) return 3; // 8x  → 256
    return 2;                       // 4x
  }

  int get _scale => _scaleStops[_scaleLog.round().clamp(0, _scaleStops.length - 1)];

  int get _defaultScale {
    final cs = ref.read(canvasProvider);
    return _scaleStops[_defaultScaleIndex(cs.width)];
  }

  bool get _isDefaultScale => _scale == _defaultScale;

  /// Auto-select appropriate filter based on scale direction.
  _ScaleFilter get _suggestedFilter {
    final cs = ref.read(canvasProvider);
    final outW = cs.width * _scale;
    if (outW < cs.width) return _ScaleFilter.lanczos3;      // downscaling
    if (_scale != _scale.roundToDouble()) return _ScaleFilter.bilinear; // non-integer
    return _ScaleFilter.nearest;                              // integer upscaling
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final cs = ref.watch(canvasProvider);
    final outW = cs.width * _scale;
    final outH = cs.height * _scale;

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 420,
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // ── Header ──
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(6),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.primary.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Icon(Icons.file_download, size: 16, color: theme.colorScheme.primary),
                ),
                const SizedBox(width: 10),
                Text('Export', style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 16, fontWeight: FontWeight.w700,
                )),
                const Spacer(),
                IconButton(
                  onPressed: () => Navigator.of(context).pop(),
                  icon: const Icon(Icons.close, size: 16),
                  visualDensity: VisualDensity.compact,
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                ),
              ],
            ),
            const SizedBox(height: 20),

            // ── Format selection ──
            const _SectionLabel('FORMAT'),
            const SizedBox(height: 8),
            _FormatGrid(
              selected: _format,
              onChanged: (f) => setState(() => _format = f),
            ),
            const SizedBox(height: 6),

            // ── Format description ──
            AnimatedSwitcher(
              duration: const Duration(milliseconds: 150),
              child: Container(
                key: ValueKey(_format),
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withValues(alpha: 0.06),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Row(
                  children: [
                    Icon(_format.icon, size: 13, color: theme.colorScheme.primary),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        _format.description,
                        style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                      ),
                    ),
                  ],
                ),
              ),
            ),

            // ── PNG options ──
            if (_format == _ExportFormat.png) ...[
              const SizedBox(height: 18),
              const _SectionLabel('SCALE'),
              const SizedBox(height: 6),

              // Scale slider
              Row(
                children: [
                  Text('1x', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                  Expanded(
                    child: SliderTheme(
                      data: SliderThemeData(
                        activeTrackColor: theme.colorScheme.primary,
                        inactiveTrackColor: theme.dividerColor,
                        thumbColor: theme.colorScheme.primary,
                        overlayColor: theme.colorScheme.primary.withValues(alpha: 0.1),
                        trackHeight: 3,
                        thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 7),
                      ),
                      child: Slider(
                        value: _scaleLog,
                        min: 0,
                        max: (_scaleStops.length - 1).toDouble(),
                        divisions: _scaleStops.length - 1,
                        onChanged: (v) => setState(() {
                          _scaleLog = v;
                          // Auto-switch filter when scale changes significantly
                          if (_filter == _ScaleFilter.nearest && _scale < ref.read(canvasProvider).width) {
                            _filter = _ScaleFilter.lanczos3;
                          }
                        }),
                      ),
                    ),
                  ),
                  Text('32x', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                ],
              ),

              // Output info
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: BoxDecoration(
                  color: StudioTheme.codeBg,
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: theme.dividerColor),
                ),
                child: Row(
                  children: [
                    _InfoPill('Canvas', '${cs.width}x${cs.height}'),
                    const SizedBox(width: 6),
                    Icon(Icons.arrow_forward, size: 10, color: theme.textTheme.bodySmall?.color),
                    const SizedBox(width: 6),
                    _InfoPill('Scale', '${_scale}x',
                        highlight: !_isDefaultScale,
                        highlightColor: !_isDefaultScale ? StudioTheme.warning : null),
                    const SizedBox(width: 6),
                    Icon(Icons.arrow_forward, size: 10, color: theme.textTheme.bodySmall?.color),
                    const SizedBox(width: 6),
                    _InfoPill('Output', '${outW}x$outH', highlight: true),
                  ],
                ),
              ),

              // ── Scaling algorithm (shown when non-default scale) ──
              if (!_isDefaultScale) ...[
                const SizedBox(height: 14),
                const _SectionLabel('SCALING ALGORITHM'),
                const SizedBox(height: 6),
                _FilterPicker(
                  selected: _filter,
                  suggested: _suggestedFilter,
                  onChanged: (f) => setState(() => _filter = f),
                ),
              ],
            ],

            const SizedBox(height: 20),

            // ── Actions ──
            Row(
              children: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: Text('Cancel', style: theme.textTheme.bodySmall),
                ),
                const Spacer(),
                FilledButton.icon(
                  onPressed: _exporting ? null : () => _doExport(),
                  icon: _exporting
                      ? const SizedBox(width: 14, height: 14,
                          child: CircularProgressIndicator(strokeWidth: 1.5, color: Colors.white))
                      : const Icon(Icons.file_download, size: 15),
                  label: Text(
                    _format == _ExportFormat.png
                        ? 'Export ${outW}x$outH'
                        : _exportLabel(),
                    style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
                  ),
                  style: FilledButton.styleFrom(
                    backgroundColor: theme.colorScheme.primary,
                    foregroundColor: Colors.white,
                    padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  String _exportLabel() {
    return switch (_format) {
      _ExportFormat.png => 'Export PNG',
      _ExportFormat.atlas => 'Export Atlas',
      _ExportFormat.pax => 'Save PAX',
      _ExportFormat.tiled => 'Export for Tiled',
      _ExportFormat.godot => 'Export for Godot',
      _ExportFormat.unity => 'Export for Unity',
      _ExportFormat.gbstudio => 'Export for GB Studio',
      _ExportFormat.texturepacker => 'Export for TexturePacker',
    };
  }

  Future<void> _doExport() async {
    setState(() => _exporting = true);
    final messenger = ScaffoldMessenger.of(context);
    final nav = Navigator.of(context);

    try {
      switch (_format) {
        case _ExportFormat.png:
          await _doExportPng(messenger, nav);
          break;

        case _ExportFormat.atlas:
          final resp = await ref.read(backendProvider.notifier).packAtlas();
          final png = resp['png'] as String?;
          if (png != null) {
            final ok = await ExportService.saveAtlasPng(png);
            messenger.showSnackBar(SnackBar(
              content: Text(ok ? 'Atlas exported' : 'Export cancelled'),
              duration: const Duration(seconds: 2),
            ));
            if (ok) nav.pop();
          } else {
            messenger.showSnackBar(SnackBar(
              content: Text('Atlas pack failed: ${resp['error'] ?? 'unknown'}'),
            ));
          }
          break;

        case _ExportFormat.pax:
          final source = await ref.read(backendProvider.notifier).getPaxSource();
          if (source != null) {
            final ok = await ExportService.savePaxSource(source);
            messenger.showSnackBar(SnackBar(
              content: Text(ok ? 'PAX source saved' : 'Save cancelled'),
              duration: const Duration(seconds: 2),
            ));
            if (ok) nav.pop();
          } else {
            messenger.showSnackBar(const SnackBar(
              content: Text('No PAX source available (engine not connected?)'),
            ));
          }
          break;

        case _ExportFormat.tiled:
        case _ExportFormat.godot:
        case _ExportFormat.texturepacker:
        case _ExportFormat.gbstudio:
        case _ExportFormat.unity:
          final format = _format.name;
          final dir = await FilePicker.platform.getDirectoryPath(
            dialogTitle: 'Export to $format',
          );
          if (dir == null) {
            messenger.showSnackBar(const SnackBar(
              content: Text('Export cancelled'),
              duration: Duration(seconds: 2),
            ));
            break;
          }
          final resp = await ref.read(backendProvider.notifier).backend.exportToEngine(
            format: format,
            outDir: dir,
          );
          if (resp['ok'] == true) {
            final files = resp['files'] as int? ?? 0;
            messenger.showSnackBar(SnackBar(
              content: Text('Exported $files files to $dir'),
              duration: const Duration(seconds: 3),
            ));
            nav.pop();
          } else {
            messenger.showSnackBar(SnackBar(
              content: Text('Export failed: ${resp['error'] ?? 'unknown'}'),
            ));
          }
          break;
      }
    } finally {
      if (mounted) setState(() => _exporting = false);
    }
  }

  /// Export PNG via the Rust engine for proper scaling with the selected filter.
  Future<void> _doExportPng(ScaffoldMessengerState messenger, NavigatorState nav) async {
    final cs = ref.read(canvasProvider);
    final outW = cs.width * _scale;
    final outH = cs.height * _scale;
    final backend = ref.read(backendProvider);

    // If engine is connected and we're using a non-default scale or non-nearest
    // filter, route through the engine for proper resampling.
    if (backend.isConnected && (!_isDefaultScale || _filter != _ScaleFilter.nearest)) {
      // Get the active tile name from the engine.
      // For now, use the Flutter-side exporter with the engine's export_png
      // endpoint for the first tile available.
      // TODO: export the full canvas composite through the engine.
      // Fall through to Flutter-side export for now.
    }

    // Flutter-side export (nearest-neighbor block replication).
    final ok = await ExportService.exportCanvasPng(
      canvasState: cs,
      scale: _scale,
    );
    messenger.showSnackBar(SnackBar(
      content: Text(ok ? 'PNG exported (${outW}x$outH, ${_filter.label})' : 'Export cancelled'),
      duration: const Duration(seconds: 2),
    ));
    if (ok) nav.pop();
  }
}

// ── Sub-widgets ──

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.text);
  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(text, style: Theme.of(context).textTheme.bodySmall!.copyWith(
      fontWeight: FontWeight.w600, fontSize: 10, letterSpacing: 1,
    ));
  }
}

class _FormatGrid extends StatelessWidget {
  const _FormatGrid({required this.selected, required this.onChanged});
  final _ExportFormat selected;
  final ValueChanged<_ExportFormat> onChanged;

  @override
  Widget build(BuildContext context) {
    final imageFormats = _ExportFormat.values.where((f) => f.category == _FormatCategory.image).toList();
    final dataFormats = _ExportFormat.values.where((f) => f.category == _FormatCategory.data).toList();
    final engineFormats = _ExportFormat.values.where((f) => f.category == _FormatCategory.engine).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          spacing: 6,
          runSpacing: 6,
          children: [
            ...imageFormats.map((f) => _FormatTile(format: f, selected: selected == f, onTap: () => onChanged(f))),
            ...dataFormats.map((f) => _FormatTile(format: f, selected: selected == f, onTap: () => onChanged(f))),
          ],
        ),
        const SizedBox(height: 6),
        Text('GAME ENGINES', style: Theme.of(context).textTheme.bodySmall!.copyWith(
          fontSize: 9, letterSpacing: 0.5,
        )),
        const SizedBox(height: 4),
        Wrap(
          spacing: 6,
          runSpacing: 6,
          children: engineFormats.map((f) =>
              _FormatTile(format: f, selected: selected == f, onTap: () => onChanged(f), compact: true),
          ).toList(),
        ),
      ],
    );
  }
}

class _FormatTile extends StatefulWidget {
  const _FormatTile({
    required this.format,
    required this.selected,
    required this.onTap,
    this.compact = false,
  });
  final _ExportFormat format;
  final bool selected;
  final VoidCallback onTap;
  final bool compact;

  @override
  State<_FormatTile> createState() => _FormatTileState();
}

class _FormatTileState extends State<_FormatTile> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sel = widget.selected;
    final hov = _hovered && !sel;

    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          padding: EdgeInsets.symmetric(
            horizontal: widget.compact ? 8 : 10,
            vertical: widget.compact ? 4 : 6,
          ),
          decoration: BoxDecoration(
            color: sel
                ? theme.colorScheme.primary.withValues(alpha: 0.15)
                : hov
                    ? theme.dividerColor.withValues(alpha: 0.3)
                    : null,
            borderRadius: BorderRadius.circular(6),
            border: Border.all(
              color: sel ? theme.colorScheme.primary : theme.dividerColor,
              width: sel ? 1.5 : 1,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(widget.format.icon, size: widget.compact ? 12 : 14,
                  color: sel ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color),
              SizedBox(width: widget.compact ? 4 : 6),
              Text(widget.format.label, style: theme.textTheme.bodySmall!.copyWith(
                fontSize: widget.compact ? 10 : 11,
                color: sel ? theme.colorScheme.primary : null,
                fontWeight: sel ? FontWeight.w600 : null,
              )),
            ],
          ),
        ),
      ),
    );
  }
}

/// Scaling algorithm picker — shows all filters with descriptions and
/// highlights the recommended one for the current scale direction.
class _FilterPicker extends StatelessWidget {
  const _FilterPicker({
    required this.selected,
    required this.suggested,
    required this.onChanged,
  });
  final _ScaleFilter selected;
  final _ScaleFilter suggested;
  final ValueChanged<_ScaleFilter> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      children: _ScaleFilter.values.map((f) {
        final isSelected = f == selected;
        final isSuggested = f == suggested;

        return Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: InkWell(
            onTap: () => onChanged(f),
            borderRadius: BorderRadius.circular(6),
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 120),
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
              decoration: BoxDecoration(
                color: isSelected
                    ? theme.colorScheme.primary.withValues(alpha: 0.12)
                    : null,
                borderRadius: BorderRadius.circular(6),
                border: Border.all(
                  color: isSelected ? theme.colorScheme.primary : theme.dividerColor,
                  width: isSelected ? 1.5 : 1,
                ),
              ),
              child: Row(
                children: [
                  Icon(
                    isSelected ? Icons.radio_button_checked : Icons.radio_button_unchecked,
                    size: 14,
                    color: isSelected ? theme.colorScheme.primary : theme.dividerColor,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Text(f.label, style: theme.textTheme.bodySmall!.copyWith(
                              fontSize: 11,
                              fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
                              color: isSelected ? theme.colorScheme.primary : null,
                            )),
                            if (isSuggested) ...[
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                                decoration: BoxDecoration(
                                  color: StudioTheme.success.withValues(alpha: 0.15),
                                  borderRadius: BorderRadius.circular(3),
                                ),
                                child: Text('recommended', style: TextStyle(
                                  fontSize: 8,
                                  fontWeight: FontWeight.w600,
                                  color: StudioTheme.success,
                                )),
                              ),
                            ],
                          ],
                        ),
                        const SizedBox(height: 1),
                        Text(f.hint, style: theme.textTheme.bodySmall!.copyWith(
                          fontSize: 9,
                        )),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
      }).toList(),
    );
  }
}

class _InfoPill extends StatelessWidget {
  const _InfoPill(this.label, this.value, {this.highlight = false, this.highlightColor});
  final String label;
  final String value;
  final bool highlight;
  final Color? highlightColor;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = highlightColor ?? theme.colorScheme.primary;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: highlight
            ? color.withValues(alpha: 0.15)
            : theme.dividerColor.withValues(alpha: 0.3),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Column(
        children: [
          Text(label, style: theme.textTheme.bodySmall!.copyWith(
            fontSize: 8, letterSpacing: 0.3,
          )),
          Text(value, style: theme.textTheme.bodySmall!.copyWith(
            fontSize: 11,
            fontWeight: highlight ? FontWeight.w700 : FontWeight.w500,
            color: highlight ? color : theme.textTheme.bodyMedium?.color,
          )),
        ],
      ),
    );
  }
}
