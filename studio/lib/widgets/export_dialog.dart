import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../services/export_service.dart';
import '../theme/studio_theme.dart';

enum _ExportFormat { png, atlas, pax, tiled, godot, texturepacker, gbstudio, unity }

/// Export dialog — choose format, resolution, and destination.
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
  int _outputWidth = 256;
  int _outputHeight = 256;
  bool _exporting = false;

  @override
  void initState() {
    super.initState();
    _updateDefaultResolution();
  }

  void _updateDefaultResolution() {
    final cs = ref.read(canvasProvider);
    // Default to a sensible output: nearest power-of-two friendly size.
    // For pixel art, 256px is a good default for 16x16 tiles (16x scale).
    final defaultScale = _suggestedScale(cs.width);
    _outputWidth = cs.width * defaultScale;
    _outputHeight = cs.height * defaultScale;
  }

  int _suggestedScale(int canvasSize) {
    // Pick a default scale that gives a nice output size
    if (canvasSize <= 8) return 32;   // 8 → 256
    if (canvasSize <= 16) return 16;  // 16 → 256
    if (canvasSize <= 32) return 8;   // 32 → 256
    if (canvasSize <= 48) return 4;   // 48 → 192
    return 4;                         // 64 → 256
  }

  int get _currentScale {
    final cs = ref.read(canvasProvider);
    if (cs.width == 0) return 1;
    return (_outputWidth / cs.width).round().clamp(1, 64);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final cs = ref.watch(canvasProvider);
    final labelStyle = theme.textTheme.bodySmall!.copyWith(
      fontWeight: FontWeight.w600, fontSize: 11, letterSpacing: 0.5,
    );

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 400,
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.file_download, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Export', style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 16, fontWeight: FontWeight.w700,
                )),
                const Spacer(),
                InkWell(
                  onTap: () => Navigator.of(context).pop(),
                  child: const Icon(Icons.close, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 20),

            // Format
            Text('FORMAT', style: labelStyle),
            const SizedBox(height: 8),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [
                _formatChip('PNG Image', _ExportFormat.png, Icons.image),
                _formatChip('Atlas', _ExportFormat.atlas, Icons.grid_4x4),
                _formatChip('PAX Source', _ExportFormat.pax, Icons.code),
              ],
            ),
            const SizedBox(height: 6),
            Text('GAME ENGINE', style: labelStyle.copyWith(fontSize: 9, color: theme.textTheme.bodySmall?.color)),
            const SizedBox(height: 4),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [
                _formatChip('Tiled', _ExportFormat.tiled, Icons.map),
                _formatChip('Godot', _ExportFormat.godot, Icons.videogame_asset),
                _formatChip('Unity', _ExportFormat.unity, Icons.sports_esports),
                _formatChip('GB Studio', _ExportFormat.gbstudio, Icons.gamepad),
                _formatChip('TexturePacker', _ExportFormat.texturepacker, Icons.texture),
              ],
            ),
            const SizedBox(height: 16),

            // PNG resolution options
            if (_format == _ExportFormat.png) ...[
              Text('OUTPUT RESOLUTION', style: labelStyle),
              const SizedBox(height: 4),
              Text(
                'Canvas: ${cs.width}x${cs.height}px  •  Scale: ${_currentScale}x',
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
              ),
              const SizedBox(height: 8),
              // Preset buttons
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: _presetButtons(cs.width, cs.height),
              ),
              const SizedBox(height: 10),
              // Custom resolution
              Row(
                children: [
                  Expanded(
                    child: _resField('Width', _outputWidth, (v) {
                      setState(() {
                        _outputWidth = v;
                        // Maintain aspect ratio
                        _outputHeight = (v * cs.height / cs.width).round();
                      });
                    }),
                  ),
                  const Padding(
                    padding: EdgeInsets.symmetric(horizontal: 8),
                    child: Icon(Icons.close, size: 12),
                  ),
                  Expanded(
                    child: _resField('Height', _outputHeight, (v) {
                      setState(() {
                        _outputHeight = v;
                        _outputWidth = (v * cs.width / cs.height).round();
                      });
                    }),
                  ),
                  const SizedBox(width: 8),
                  Text('px', style: theme.textTheme.bodySmall!.copyWith(fontSize: 11)),
                ],
              ),
              const SizedBox(height: 16),
            ],

            // Export button
            SizedBox(
              width: double.infinity,
              child: ElevatedButton.icon(
                onPressed: _exporting ? null : () => _doExport(),
                icon: _exporting
                    ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 1.5, color: Colors.white))
                    : const Icon(Icons.file_download, size: 16),
                label: Text(_exportLabel(), style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
                style: ElevatedButton.styleFrom(
                  backgroundColor: theme.colorScheme.primary,
                  foregroundColor: Colors.white,
                  padding: const EdgeInsets.symmetric(vertical: 12),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _formatChip(String label, _ExportFormat format, IconData icon) {
    final theme = Theme.of(context);
    final isActive = _format == format;
    return InkWell(
      onTap: () => setState(() => _format = format),
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.15) : null,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
            color: isActive ? theme.colorScheme.primary : theme.dividerColor,
            width: isActive ? 1.5 : 1,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 14, color: isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color),
            const SizedBox(width: 6),
            Text(label, style: theme.textTheme.bodySmall!.copyWith(
              fontSize: 11,
              color: isActive ? theme.colorScheme.primary : null,
              fontWeight: isActive ? FontWeight.w600 : null,
            )),
          ],
        ),
      ),
    );
  }

  List<Widget> _presetButtons(int w, int h) {
    final scales = <int, String>{};
    // Build scale options that make sense for this canvas size
    for (final s in [1, 2, 4, 8, 16, 32]) {
      final outW = w * s;
      if (outW > 2048) break;
      scales[s] = '${outW}x${h * s}';
    }

    return scales.entries.map((e) {
      final isActive = _currentScale == e.key;
      final theme = Theme.of(context);
      return InkWell(
        onTap: () => setState(() {
          _outputWidth = w * e.key;
          _outputHeight = h * e.key;
        }),
        borderRadius: BorderRadius.circular(4),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: BoxDecoration(
            color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.2) : null,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(
              color: isActive ? theme.colorScheme.primary : theme.dividerColor,
            ),
          ),
          child: Column(
            children: [
              Text('${e.key}x', style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 10,
                fontWeight: isActive ? FontWeight.w700 : null,
                color: isActive ? theme.colorScheme.primary : null,
              )),
              Text(e.value, style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 8,
                color: isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color,
              )),
            ],
          ),
        ),
      );
    }).toList();
  }

  Widget _resField(String hint, int value, void Function(int) onChanged) {
    final theme = Theme.of(context);
    return TextField(
      controller: TextEditingController(text: '$value'),
      keyboardType: TextInputType.number,
      style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
      decoration: InputDecoration(
        labelText: hint,
        labelStyle: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
        isDense: true,
        contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(4),
          borderSide: BorderSide(color: theme.dividerColor),
        ),
      ),
      onChanged: (text) {
        final v = int.tryParse(text);
        if (v != null && v > 0) onChanged(v);
      },
    );
  }

  String _exportLabel() {
    return switch (_format) {
      _ExportFormat.png => 'Export PNG (${_outputWidth}x$_outputHeight)',
      _ExportFormat.atlas => 'Export Atlas',
      _ExportFormat.pax => 'Save PAX Source',
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
          final cs = ref.read(canvasProvider);
          final ok = await ExportService.exportCanvasPng(
            canvasState: cs,
            scale: _currentScale,
          );
          messenger.showSnackBar(SnackBar(
            content: Text(ok ? 'PNG exported (${_outputWidth}x$_outputHeight)' : 'Export cancelled'),
            duration: const Duration(seconds: 2),
          ));
          if (ok) nav.pop();
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
}
