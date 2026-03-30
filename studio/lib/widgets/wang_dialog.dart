import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';

/// Dialog for generating a complete Wang tileset for terrain transitions.
class WangDialog extends ConsumerStatefulWidget {
  const WangDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (ctx) => const WangDialog(),
    );
  }

  @override
  ConsumerState<WangDialog> createState() => _WangDialogState();
}

class _WangDialogState extends ConsumerState<WangDialog> {
  final _terrainAController = TextEditingController(text: 'grass');
  final _terrainBController = TextEditingController(text: 'water');
  String _method = 'dual_grid';
  int _size = 16;
  bool _generating = false;
  String? _result;

  @override
  void dispose() {
    _terrainAController.dispose();
    _terrainBController.dispose();
    super.dispose();
  }

  Future<void> _generate() async {
    setState(() {
      _generating = true;
      _result = null;
    });

    final resp = await ref.read(backendProvider.notifier).generateWang(
      terrainA: _terrainAController.text.trim(),
      terrainB: _terrainBController.text.trim(),
      method: _method,
      size: _size,
    );

    if (mounted) {
      setState(() {
        _generating = false;
        if (resp != null) {
          final count = resp['tiles_created'] ?? 0;
          _result = 'Created $count tiles (${_method == 'dual_grid' ? 'dual grid' : 'blob 47'})';
        } else {
          _result = 'Generation failed';
        }
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AlertDialog(
      title: const Text('Generate Wang Tileset'),
      content: SizedBox(
        width: 340,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Generate all transition tiles between two terrain types '
              'with correct edge classes for WFC.',
              style: theme.textTheme.bodySmall,
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _terrainAController,
                    decoration: const InputDecoration(
                      labelText: 'Terrain A',
                      hintText: 'grass',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  child: Icon(Icons.arrow_forward, size: 16, color: theme.hintColor),
                ),
                Expanded(
                  child: TextField(
                    controller: _terrainBController,
                    decoration: const InputDecoration(
                      labelText: 'Terrain B',
                      hintText: 'water',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: DropdownButtonFormField<String>(
                    value: _method,
                    decoration: const InputDecoration(
                      labelText: 'Method',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    items: const [
                      DropdownMenuItem(value: 'dual_grid', child: Text('Dual Grid (15 tiles)')),
                      DropdownMenuItem(value: 'blob_47', child: Text('Blob 47 (47 tiles)')),
                    ],
                    onChanged: (v) => setState(() => _method = v ?? 'dual_grid'),
                  ),
                ),
                const SizedBox(width: 8),
                SizedBox(
                  width: 80,
                  child: DropdownButtonFormField<int>(
                    value: _size,
                    decoration: const InputDecoration(
                      labelText: 'Size',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    items: const [
                      DropdownMenuItem(value: 8, child: Text('8px')),
                      DropdownMenuItem(value: 16, child: Text('16px')),
                      DropdownMenuItem(value: 32, child: Text('32px')),
                    ],
                    onChanged: (v) => setState(() => _size = v ?? 16),
                  ),
                ),
              ],
            ),
            if (_result != null) ...[
              const SizedBox(height: 12),
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: _result!.contains('failed')
                      ? theme.colorScheme.errorContainer
                      : theme.colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Row(
                  children: [
                    Icon(
                      _result!.contains('failed') ? Icons.error_outline : Icons.check_circle,
                      size: 16,
                      color: _result!.contains('failed')
                          ? theme.colorScheme.error
                          : theme.colorScheme.primary,
                    ),
                    const SizedBox(width: 8),
                    Expanded(child: Text(_result!, style: theme.textTheme.bodySmall)),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(_result != null ? 'Done' : 'Cancel'),
        ),
        if (_result == null)
          FilledButton.icon(
            onPressed: _generating ? null : _generate,
            icon: _generating
                ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2))
                : const Icon(Icons.grid_on, size: 16),
            label: Text(_generating ? 'Generating...' : 'Generate'),
          ),
      ],
    );
  }
}
