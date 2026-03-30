import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/scanner_provider.dart';
import '../theme/studio_theme.dart';
import 'llm_provider_settings.dart';

/// General settings dialog — hub for all Studio configuration.
class SettingsDialog extends ConsumerWidget {
  const SettingsDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const SettingsDialog(),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: StudioTheme.panelBorder,
      ),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: MediaQuery.of(context).size.height * 0.8,
          maxWidth: 460,
        ),
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header
              Row(
                children: [
                  Icon(Icons.settings, size: 18, color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Text('Settings', style: theme.textTheme.bodyMedium!.copyWith(
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

              // Settings sections
              _SettingsRow(
                icon: Icons.smart_toy,
                label: 'LLM Provider',
                description: 'API keys, model selection, Ollama, LoRA',
                onTap: () {
                  Navigator.of(context).pop();
                  LlmProviderSettings.show(context);
                },
              ),

              const SizedBox(height: 8),

              _SettingsRow(
                icon: Icons.folder_special,
                label: 'Training Data Directories',
                description: 'Where PIXL looks for datasets and adapters',
                onTap: () {
                  Navigator.of(context).pop();
                  _DatasetDirsDialog.show(context);
                },
              ),

              const SizedBox(height: 16),

              // Done
              SizedBox(
                width: double.infinity,
                child: ElevatedButton(
                  onPressed: () => Navigator.of(context).pop(),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: theme.colorScheme.primary,
                    foregroundColor: Colors.white,
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                  ),
                  child: const Text('Done'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SettingsRow extends StatelessWidget {
  const _SettingsRow({
    required this.icon,
    required this.label,
    required this.description,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final String description;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: theme.dividerColor),
        ),
        child: Row(
          children: [
            Icon(icon, size: 18, color: theme.colorScheme.primary),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(label, style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 12, fontWeight: FontWeight.w600,
                  )),
                  const SizedBox(height: 2),
                  Text(description, style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 10,
                  )),
                ],
              ),
            ),
            Icon(Icons.chevron_right, size: 18, color: theme.dividerColor),
          ],
        ),
      ),
    );
  }
}

/// Dialog to manage training data directories.
class _DatasetDirsDialog extends ConsumerStatefulWidget {
  const _DatasetDirsDialog();

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const _DatasetDirsDialog(),
    );
  }

  @override
  ConsumerState<_DatasetDirsDialog> createState() => _DatasetDirsDialogState();
}

class _DatasetDirsDialogState extends ConsumerState<_DatasetDirsDialog> {
  final _controller = TextEditingController();
  int? _editingIndex;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dirs = ref.watch(scannerProvider).datasetDirs;

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 480,
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.folder_special, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Training Data Directories', style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 16, fontWeight: FontWeight.w700,
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
              'PIXL scans these directories (including subdirectories) for training datasets.',
              style: theme.textTheme.bodySmall!.copyWith(
                color: theme.colorScheme.onSurface.withValues(alpha: 0.6),
              ),
            ),
            const SizedBox(height: 16),

            // Directory list
            ...List.generate(dirs.length, (i) {
              final dir = dirs[i];
              final isEditing = _editingIndex == i;

              if (isEditing) {
                return Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _controller,
                          autofocus: true,
                          style: theme.textTheme.bodySmall!.copyWith(fontFamily: 'monospace'),
                          decoration: InputDecoration(
                            isDense: true,
                            border: OutlineInputBorder(borderRadius: BorderRadius.circular(6)),
                            contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
                          ),
                          onSubmitted: (_) => _finishEdit(i),
                        ),
                      ),
                      const SizedBox(width: 4),
                      IconButton(
                        icon: const Icon(Icons.check, size: 16),
                        onPressed: () => _finishEdit(i),
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
                      ),
                    ],
                  ),
                );
              }

              return Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: theme.dividerColor),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.folder, size: 14,
                        color: theme.colorScheme.onSurface.withValues(alpha: 0.5)),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(dir,
                          style: theme.textTheme.bodySmall!.copyWith(fontFamily: 'monospace'),
                          overflow: TextOverflow.ellipsis),
                      ),
                      InkWell(
                        onTap: () => _startEdit(i, dir),
                        child: Icon(Icons.edit, size: 14,
                          color: theme.colorScheme.onSurface.withValues(alpha: 0.4)),
                      ),
                      const SizedBox(width: 4),
                      InkWell(
                        onTap: dirs.length > 1 ? () => _removeDir(dir) : null,
                        child: Icon(Icons.close, size: 14,
                          color: dirs.length > 1
                            ? theme.colorScheme.error.withValues(alpha: 0.6)
                            : theme.dividerColor),
                      ),
                    ],
                  ),
                ),
              );
            }),

            const SizedBox(height: 8),

            // Add directory
            Row(
              children: [
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: _addDirManual,
                    icon: const Icon(Icons.add, size: 14),
                    label: const Text('Add path', style: TextStyle(fontSize: 12)),
                    style: OutlinedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(vertical: 8),
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: _browseDirPicker,
                    icon: const Icon(Icons.folder_open, size: 14),
                    label: const Text('Browse', style: TextStyle(fontSize: 12)),
                    style: OutlinedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(vertical: 8),
                    ),
                  ),
                ),
              ],
            ),

            const SizedBox(height: 16),

            SizedBox(
              width: double.infinity,
              child: ElevatedButton(
                onPressed: () => Navigator.of(context).pop(),
                style: ElevatedButton.styleFrom(
                  backgroundColor: theme.colorScheme.primary,
                  foregroundColor: Colors.white,
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                ),
                child: const Text('Done'),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _startEdit(int index, String current) {
    setState(() {
      _editingIndex = index;
      _controller.text = current;
    });
  }

  void _finishEdit(int index) {
    final newDir = _controller.text.trim();
    if (newDir.isNotEmpty) {
      final oldDir = ref.read(scannerProvider).datasetDirs[index];
      ref.read(scannerProvider.notifier).updateDatasetDir(oldDir, newDir);
    }
    setState(() => _editingIndex = null);
  }

  void _removeDir(String dir) {
    ref.read(scannerProvider.notifier).removeDatasetDir(dir);
  }

  void _addDirManual() {
    setState(() {
      ref.read(scannerProvider.notifier).addDatasetDir('');
      _editingIndex = ref.read(scannerProvider).datasetDirs.length - 1;
      _controller.text = '';
    });
  }

  Future<void> _browseDirPicker() async {
    final result = await FilePicker.platform.getDirectoryPath(
      dialogTitle: 'Select training data directory',
    );
    if (result != null) {
      ref.read(scannerProvider.notifier).addDatasetDir(result);
    }
  }
}
