import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../theme/studio_theme.dart';

/// Dialog for importing images as PAX backdrops and previewing them.
class BackdropDialog extends ConsumerStatefulWidget {
  const BackdropDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const BackdropDialog(),
    );
  }

  @override
  ConsumerState<BackdropDialog> createState() => _BackdropDialogState();
}

class _BackdropDialogState extends ConsumerState<BackdropDialog> {
  // ── Import tab state ──
  String? _inputPath;
  String? _sceneName;
  int _colors = 32;
  bool _importing = false;
  Map<String, dynamic>? _importResult;

  // ── Render tab state ──
  String? _paxPath;
  String? _backdropName;
  int _frames = 0;
  int _scale = 4;
  bool _rendering = false;
  Map<String, dynamic>? _renderResult;
  String? _previewBase64;

  String? _error;
  int _tabIndex = 0;

  Future<void> _pickInput() async {
    final result = await FilePicker.platform.pickFiles(
      dialogTitle: 'Select Image to Import',
      type: FileType.image,
    );
    if (result != null && result.files.isNotEmpty) {
      setState(() {
        _inputPath = result.files.single.path;
        _importResult = null;
        _error = null;
      });
    }
  }

  Future<void> _pickPax() async {
    final result = await FilePicker.platform.pickFiles(
      dialogTitle: 'Select PAX File',
      type: FileType.any,
    );
    if (result != null && result.files.isNotEmpty) {
      final path = result.files.single.path;
      if (path != null && (path.endsWith('.pax') || path.endsWith('.pixl'))) {
        setState(() {
          _paxPath = path;
          _renderResult = null;
          _previewBase64 = null;
          _error = null;
        });
      }
    }
  }

  Future<void> _import() async {
    if (_inputPath == null) return;
    setState(() {
      _importing = true;
      _importResult = null;
      _error = null;
    });

    try {
      final backend = ref.read(backendProvider.notifier).backend;
      final resp = await backend.backdropImport(
        inputPath: _inputPath!,
        name: _sceneName ?? 'scene',
        colors: _colors,
      );

      if (mounted) {
        setState(() {
          _importing = false;
          if (resp['ok'] == true) {
            _importResult = resp;
            _paxPath = resp['path'] as String?;
            _backdropName = _sceneName ?? 'scene';
          } else {
            _error = resp['error']?.toString() ?? 'Unknown error';
          }
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _importing = false;
          _error = 'Import failed: $e';
        });
      }
    }
  }

  Future<void> _render() async {
    if (_paxPath == null || _backdropName == null) return;
    setState(() {
      _rendering = true;
      _renderResult = null;
      _previewBase64 = null;
      _error = null;
    });

    try {
      final backend = ref.read(backendProvider.notifier).backend;
      final resp = await backend.backdropRender(
        filePath: _paxPath!,
        name: _backdropName!,
        frames: _frames,
        scale: _scale,
      );

      if (mounted) {
        setState(() {
          _rendering = false;
          if (resp['ok'] == true) {
            _renderResult = resp;
            _previewBase64 = (resp['png_base64'] ?? resp['gif_base64']) as String?;
          } else {
            _error = resp['error']?.toString() ?? 'Unknown error';
          }
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _rendering = false;
          _error = 'Render failed: $e';
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final backend = ref.watch(backendProvider);

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 520,
        height: 600,
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.landscape, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Backdrop', style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 16, fontWeight: FontWeight.w700,
                )),
                const Spacer(),
                InkWell(
                  onTap: () => Navigator.of(context).pop(),
                  child: const Icon(Icons.close, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 12),

            // Tab bar
            Row(
              children: [
                _TabButton(
                  label: 'Import',
                  icon: Icons.file_upload,
                  active: _tabIndex == 0,
                  onTap: () => setState(() => _tabIndex = 0),
                ),
                const SizedBox(width: 8),
                _TabButton(
                  label: 'Render',
                  icon: Icons.image,
                  active: _tabIndex == 1,
                  onTap: () => setState(() => _tabIndex = 1),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Tab content
            Expanded(
              child: SingleChildScrollView(
                child: _tabIndex == 0 ? _buildImportTab(theme, backend) : _buildRenderTab(theme, backend),
              ),
            ),

            // Error
            if (_error != null) ...[
              const SizedBox(height: 8),
              _infoBox(_error!, Icons.error_outline, StudioTheme.error, theme),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildImportTab(ThemeData theme, BackendState backend) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          'Import an image as a tile-decomposed PAX backdrop with '
          'extended palettes, animation zones, and procedural effects.',
          style: theme.textTheme.bodySmall!.copyWith(fontSize: 11, height: 1.4),
        ),
        const SizedBox(height: 12),

        // Input file
        Text('INPUT IMAGE', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        _FilePickerRow(
          path: _inputPath,
          placeholder: 'Select an image...',
          icon: Icons.add_photo_alternate,
          onTap: _pickInput,
          theme: theme,
        ),
        const SizedBox(height: 12),

        // Scene name
        Text('SCENE NAME', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        SizedBox(
          height: 36,
          child: TextField(
            style: theme.textTheme.bodySmall!.copyWith(fontSize: 12),
            decoration: InputDecoration(
              hintText: 'scene',
              isDense: true,
              contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(6)),
            ),
            onChanged: (v) => _sceneName = v.isEmpty ? null : v,
          ),
        ),
        const SizedBox(height: 12),

        // Colors slider
        Text('PALETTE COLORS: $_colors', style: theme.textTheme.titleSmall),
        const SizedBox(height: 4),
        Slider(
          value: _colors.toDouble(),
          min: 8,
          max: 48,
          divisions: 10,
          label: '$_colors',
          onChanged: (v) => setState(() => _colors = v.round()),
        ),
        const SizedBox(height: 12),

        // Import button
        if (!backend.isConnected)
          _infoBox('Engine not connected.', Icons.warning_amber_rounded, StudioTheme.error, theme)
        else
          ElevatedButton.icon(
            onPressed: (_importing || _inputPath == null) ? null : _import,
            icon: _importing
                ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 1.5, color: Colors.white))
                : const Icon(Icons.file_upload, size: 14),
            label: Text(_importing ? 'Importing...' : 'Import as Backdrop'),
            style: ElevatedButton.styleFrom(
              backgroundColor: theme.colorScheme.primary,
              foregroundColor: Colors.white,
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),

        // Import result
        if (_importResult != null) ...[
          const SizedBox(height: 12),
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: StudioTheme.success.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: StudioTheme.success.withValues(alpha: 0.3)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(children: [
                  const Icon(Icons.check_circle, size: 14, color: StudioTheme.success),
                  const SizedBox(width: 6),
                  Text('Import complete!', style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 12, fontWeight: FontWeight.w600, color: StudioTheme.success,
                  )),
                ]),
                const SizedBox(height: 6),
                Text(
                  '${_importResult!['unique_tiles']} unique tiles '
                  '(${_importResult!['cols']}x${_importResult!['rows']} grid)\n'
                  'PAX: ${((_importResult!['pax_size_bytes'] as int) / 1024).toStringAsFixed(1)} KB\n'
                  'Saved: ${_importResult!['path']}',
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 10, fontFamily: 'monospace', height: 1.4),
                ),
              ],
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildRenderTab(ThemeData theme, BackendState backend) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          'Render a backdrop from a PAX file. Static PNG or animated GIF '
          'with procedural zone effects.',
          style: theme.textTheme.bodySmall!.copyWith(fontSize: 11, height: 1.4),
        ),
        const SizedBox(height: 12),

        // PAX file
        Text('PAX FILE', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        _FilePickerRow(
          path: _paxPath,
          placeholder: 'Select a .pax file...',
          icon: Icons.description,
          onTap: _pickPax,
          theme: theme,
        ),
        const SizedBox(height: 12),

        // Backdrop name
        Text('BACKDROP NAME', style: theme.textTheme.titleSmall),
        const SizedBox(height: 6),
        SizedBox(
          height: 36,
          child: TextField(
            style: theme.textTheme.bodySmall!.copyWith(fontSize: 12),
            decoration: InputDecoration(
              hintText: 'scene',
              isDense: true,
              contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(6)),
            ),
            onChanged: (v) => setState(() => _backdropName = v.isEmpty ? null : v),
          ),
        ),
        const SizedBox(height: 12),

        // Frames + scale
        Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('FRAMES (0=static)', style: theme.textTheme.titleSmall),
                  const SizedBox(height: 4),
                  SizedBox(
                    height: 36,
                    child: TextField(
                      style: theme.textTheme.bodySmall!.copyWith(fontSize: 12),
                      decoration: InputDecoration(
                        hintText: '0',
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                        border: OutlineInputBorder(borderRadius: BorderRadius.circular(6)),
                      ),
                      keyboardType: TextInputType.number,
                      onChanged: (v) => setState(() => _frames = int.tryParse(v) ?? 0),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('SCALE', style: theme.textTheme.titleSmall),
                  const SizedBox(height: 4),
                  SizedBox(
                    height: 36,
                    child: TextField(
                      style: theme.textTheme.bodySmall!.copyWith(fontSize: 12),
                      decoration: InputDecoration(
                        hintText: '4',
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                        border: OutlineInputBorder(borderRadius: BorderRadius.circular(6)),
                      ),
                      keyboardType: TextInputType.number,
                      onChanged: (v) => setState(() => _scale = int.tryParse(v) ?? 4),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),

        // Render button
        if (!backend.isConnected)
          _infoBox('Engine not connected.', Icons.warning_amber_rounded, StudioTheme.error, theme)
        else
          ElevatedButton.icon(
            onPressed: (_rendering || _paxPath == null || _backdropName == null) ? null : _render,
            icon: _rendering
                ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 1.5, color: Colors.white))
                : const Icon(Icons.image, size: 14),
            label: Text(_rendering ? 'Rendering...' : (_frames > 0 ? 'Render GIF' : 'Render PNG')),
            style: ElevatedButton.styleFrom(
              backgroundColor: theme.colorScheme.primary,
              foregroundColor: Colors.white,
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),

        // Preview
        if (_previewBase64 != null) ...[
          const SizedBox(height: 12),
          Container(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: theme.dividerColor),
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(6),
              child: Image.memory(
                base64Decode(_previewBase64!),
                filterQuality: FilterQuality.none,
                fit: BoxFit.contain,
              ),
            ),
          ),
          if (_renderResult != null)
            Padding(
              padding: const EdgeInsets.only(top: 6),
              child: Text(
                _renderResult!['size'] != null
                    ? 'Size: ${_renderResult!['size']}'
                    : '${_renderResult!['frames']} frames',
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
              ),
            ),
        ],
      ],
    );
  }

  Widget _infoBox(String text, IconData icon, Color color, ThemeData theme) {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: theme.dividerColor),
      ),
      child: Row(
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 8),
          Expanded(child: Text(text, style: theme.textTheme.bodySmall!.copyWith(fontSize: 11))),
        ],
      ),
    );
  }
}

class _TabButton extends StatelessWidget {
  const _TabButton({
    required this.label,
    required this.icon,
    required this.active,
    required this.onTap,
  });

  final String label;
  final IconData icon;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: active ? theme.colorScheme.primary.withValues(alpha: 0.15) : null,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
            color: active ? theme.colorScheme.primary.withValues(alpha: 0.4) : theme.dividerColor,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 14, color: active ? theme.colorScheme.primary : theme.dividerColor),
            const SizedBox(width: 6),
            Text(label, style: TextStyle(
              fontSize: 11,
              fontWeight: active ? FontWeight.w700 : null,
              color: active ? theme.colorScheme.primary : null,
            )),
          ],
        ),
      ),
    );
  }
}

class _FilePickerRow extends StatelessWidget {
  const _FilePickerRow({
    required this.path,
    required this.placeholder,
    required this.icon,
    required this.onTap,
    required this.theme,
  });

  final String? path;
  final String placeholder;
  final IconData icon;
  final VoidCallback onTap;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
            color: path != null
                ? theme.colorScheme.primary.withValues(alpha: 0.5)
                : theme.dividerColor,
          ),
        ),
        child: Row(
          children: [
            Icon(
              path != null ? Icons.check_circle : icon,
              size: 18,
              color: path != null ? StudioTheme.success : theme.dividerColor,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                path?.split('/').last ?? placeholder,
                style: theme.textTheme.bodySmall!.copyWith(
                  fontSize: 12,
                  color: path != null ? null : theme.dividerColor,
                ),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
