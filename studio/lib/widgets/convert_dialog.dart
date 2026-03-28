import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../theme/studio_theme.dart';

/// Dialog for converting AI-generated images to true 1:1 pixel art.
class ConvertDialog extends ConsumerStatefulWidget {
  const ConvertDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const ConvertDialog(),
    );
  }

  @override
  ConsumerState<ConvertDialog> createState() => _ConvertDialogState();
}

class _ConvertDialogState extends ConsumerState<ConvertDialog> {
  String? _inputPath;
  String? _outDir;
  bool _converting = false;
  Map<String, dynamic>? _result;
  String? _error;

  Future<void> _pickInput() async {
    final result = await FilePicker.platform.pickFiles(
      dialogTitle: 'Select Image to Convert',
      type: FileType.image,
    );
    if (result != null && result.files.isNotEmpty) {
      setState(() {
        _inputPath = result.files.single.path;
        _result = null;
        _error = null;
      });
    }
  }

  Future<void> _pickOutput() async {
    final dir = await FilePicker.platform.getDirectoryPath(
      dialogTitle: 'Output Directory',
    );
    if (dir != null) {
      setState(() => _outDir = dir);
    }
  }

  Future<void> _convert() async {
    if (_inputPath == null) return;

    setState(() {
      _converting = true;
      _result = null;
      _error = null;
    });

    final backend = ref.read(backendProvider.notifier).backend;
    final resp = await backend.convertSprite(
      inputPath: _inputPath!,
      outDir: _outDir,
    );

    if (mounted) {
      setState(() {
        _converting = false;
        if (resp['ok'] == true) {
          _result = resp;
        } else {
          _error = resp['error']?.toString() ?? 'Unknown error';
        }
      });
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
        width: 480,
        padding: const EdgeInsets.all(20),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header
              Row(
                children: [
                  Icon(Icons.auto_fix_high, size: 18, color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Text('Convert to Pixel Art',
                      style: theme.textTheme.bodyMedium!.copyWith(
                        fontSize: 16,
                        fontWeight: FontWeight.w700,
                      )),
                  const Spacer(),
                  InkWell(
                    onTap: () => Navigator.of(context).pop(),
                    child: const Icon(Icons.close, size: 18),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                'Convert AI-generated images to true 1:1 pixel art. '
                'Produces 3 resolution presets: small (128px, 16 colors), '
                'medium (160px, 32 colors), large (256px, 48 colors).',
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 11, height: 1.4),
              ),
              const SizedBox(height: 16),

              // Input file
              Text('INPUT', style: theme.textTheme.titleSmall),
              const SizedBox(height: 6),
              InkWell(
                onTap: _pickInput,
                borderRadius: BorderRadius.circular(6),
                child: Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: _inputPath != null
                          ? theme.colorScheme.primary.withValues(alpha: 0.5)
                          : theme.dividerColor,
                    ),
                  ),
                  child: Row(
                    children: [
                      Icon(
                        _inputPath != null ? Icons.image : Icons.add_photo_alternate,
                        size: 18,
                        color: _inputPath != null ? theme.colorScheme.primary : theme.dividerColor,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _inputPath?.split('/').last ?? 'Select an image...',
                          style: theme.textTheme.bodySmall!.copyWith(
                            fontSize: 12,
                            color: _inputPath != null ? null : theme.dividerColor,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (_inputPath != null)
                        Icon(Icons.check_circle, size: 14, color: StudioTheme.success),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 12),

              // Output directory
              Text('OUTPUT DIRECTORY', style: theme.textTheme.titleSmall),
              const SizedBox(height: 6),
              InkWell(
                onTap: _pickOutput,
                borderRadius: BorderRadius.circular(6),
                child: Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: theme.dividerColor),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.folder_outlined, size: 18, color: theme.dividerColor),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _outDir ?? 'Default: pixl_convert/ next to input',
                          style: theme.textTheme.bodySmall!.copyWith(
                            fontSize: 12,
                            color: _outDir != null ? null : theme.dividerColor,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 8),

              // Presets info
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: theme.dividerColor.withValues(alpha: 0.15),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: theme.dividerColor),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Output structure:', style: theme.textTheme.bodySmall!.copyWith(
                      fontWeight: FontWeight.w600, fontSize: 11,
                    )),
                    const SizedBox(height: 4),
                    Text(
                      'originals/  \u2014 copy of input\n'
                      'small/      \u2014 128px wide, 16 colors\n'
                      'medium/     \u2014 160px wide, 32 colors\n'
                      'large/      \u2014 256px wide, 48 colors',
                      style: theme.textTheme.bodySmall!.copyWith(
                        fontSize: 10,
                        fontFamily: 'monospace',
                        height: 1.5,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),

              // Convert button
              if (!backend.isConnected)
                _infoBox('Engine not connected. Start the engine first.',
                    Icons.warning_amber_rounded, StudioTheme.error, theme)
              else
                Row(
                  children: [
                    ElevatedButton.icon(
                      onPressed: (_converting || _inputPath == null)
                          ? null
                          : _convert,
                      icon: _converting
                          ? const SizedBox(
                              width: 14,
                              height: 14,
                              child: CircularProgressIndicator(strokeWidth: 1.5, color: Colors.white),
                            )
                          : const Icon(Icons.auto_fix_high, size: 14),
                      label: Text(_converting ? 'Converting...' : 'Convert'),
                      style: ElevatedButton.styleFrom(
                        backgroundColor: theme.colorScheme.primary,
                        foregroundColor: Colors.white,
                        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
                        textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                      ),
                    ),
                  ],
                ),

              // Result
              if (_result != null) ...[
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
                      Row(
                        children: [
                          const Icon(Icons.check_circle, size: 14, color: StudioTheme.success),
                          const SizedBox(width: 6),
                          Text('Conversion complete!', style: theme.textTheme.bodySmall!.copyWith(
                            fontSize: 12, fontWeight: FontWeight.w600, color: StudioTheme.success,
                          )),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(
                        'Original: ${_result!['original_size']}\n'
                        'Output: ${_result!['out_dir']}',
                        style: theme.textTheme.bodySmall!.copyWith(fontSize: 10, height: 1.4),
                      ),
                      if (_result!['presets'] is List)
                        ...(_result!['presets'] as List).map((p) => Padding(
                          padding: const EdgeInsets.only(top: 2),
                          child: Text(
                            '  ${p['preset']}: ${p['size']} (${p['colors']} colors)',
                            style: theme.textTheme.bodySmall!.copyWith(
                              fontSize: 10, fontFamily: 'monospace',
                            ),
                          ),
                        )),
                    ],
                  ),
                ),
              ],

              // Error
              if (_error != null) ...[
                const SizedBox(height: 12),
                _infoBox(_error!, Icons.error_outline, StudioTheme.error, theme),
              ],
            ],
          ),
        ),
      ),
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
          Expanded(
              child: Text(text,
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 11))),
        ],
      ),
    );
  }
}
