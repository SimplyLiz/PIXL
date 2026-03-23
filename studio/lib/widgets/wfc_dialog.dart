import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/chat_provider.dart';
import '../theme/studio_theme.dart';

/// Dialog for WFC map generation using the backend /api/narrate endpoint.
class WfcDialog extends ConsumerStatefulWidget {
  const WfcDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const WfcDialog(),
    );
  }

  @override
  ConsumerState<WfcDialog> createState() => _WfcDialogState();
}

class _WfcDialogState extends ConsumerState<WfcDialog> {
  int _width = 12;
  int _height = 8;
  int _seed = 42;
  final _rulesController = TextEditingController(text: 'border:wall\nfill:floor');

  bool _generating = false;
  Uint8List? _previewBytes;
  String? _error;

  Future<void> _generate() async {
    final backend = ref.read(backendProvider);
    if (!backend.isConnected) {
      setState(() => _error = 'Engine not connected');
      return;
    }

    final rules = _rulesController.text
        .split('\n')
        .map((l) => l.trim())
        .where((l) => l.isNotEmpty)
        .toList();

    if (rules.isEmpty) {
      setState(() => _error = 'Add at least one rule');
      return;
    }

    setState(() {
      _generating = true;
      _error = null;
      _previewBytes = null;
    });

    final resp = await ref.read(backendProvider.notifier).backend.narrateMap(
      width: _width,
      height: _height,
      rules: rules,
      seed: _seed,
    );

    if (resp.containsKey('error')) {
      setState(() {
        _generating = false;
        _error = resp['error'] as String;
      });
      return;
    }

    // Extract preview image if available
    final preview = resp['preview'] as String?;
    if (preview != null) {
      setState(() {
        _previewBytes = base64Decode(preview);
        _generating = false;
      });
    } else {
      // Show the map as text if no image preview
      final mapText = resp['map'] as String? ?? resp.toString();
      ref.read(chatProvider.notifier).addAssistantMessage(
        '**WFC Map** (${_width}x$_height, seed $_seed):\n\n```\n$mapText\n```',
      );
      setState(() => _generating = false);
    }
  }

  @override
  void dispose() {
    _rulesController.dispose();
    super.dispose();
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
        width: 500,
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.auto_awesome_mosaic, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('WFC Map Generator', style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 16, fontWeight: FontWeight.w700,
                )),
                const Spacer(),
                InkWell(
                  onTap: () => Navigator.of(context).pop(),
                  child: const Icon(Icons.close, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Dimensions
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('WIDTH', style: theme.textTheme.titleSmall),
                      const SizedBox(height: 4),
                      _NumberInput(
                        value: _width,
                        min: 4, max: 64,
                        onChanged: (v) => setState(() => _width = v),
                        enabled: !_generating,
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('HEIGHT', style: theme.textTheme.titleSmall),
                      const SizedBox(height: 4),
                      _NumberInput(
                        value: _height,
                        min: 4, max: 64,
                        onChanged: (v) => setState(() => _height = v),
                        enabled: !_generating,
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('SEED', style: theme.textTheme.titleSmall),
                      const SizedBox(height: 4),
                      _NumberInput(
                        value: _seed,
                        min: 0, max: 99999,
                        onChanged: (v) => setState(() => _seed = v),
                        enabled: !_generating,
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Rules
            Text('RULES (one per line)', style: theme.textTheme.titleSmall),
            const SizedBox(height: 4),
            TextField(
              controller: _rulesController,
              enabled: !_generating,
              style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
              maxLines: 4,
              decoration: InputDecoration(
                hintText: 'border:wall\nfill:floor\nscatter:moss:0.1',
                hintStyle: theme.textTheme.bodySmall,
                isDense: true,
                contentPadding: const EdgeInsets.all(10),
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(4),
                  borderSide: StudioTheme.panelBorder,
                ),
                focusedBorder: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(4),
                  borderSide: BorderSide(color: theme.colorScheme.primary),
                ),
              ),
            ),
            const SizedBox(height: 4),
            Text(
              'Predicates: border:<tile>, fill:<tile>, scatter:<tile>:<density>, '
              'path:<tile>:<from>:<to>',
              style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
            ),

            // Preview
            if (_previewBytes != null) ...[
              const SizedBox(height: 16),
              Center(
                child: Container(
                  constraints: const BoxConstraints(maxHeight: 300),
                  decoration: BoxDecoration(
                    border: Border.all(color: theme.dividerColor),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(4),
                    child: Image.memory(
                      _previewBytes!,
                      filterQuality: FilterQuality.none,
                      fit: BoxFit.contain,
                    ),
                  ),
                ),
              ),
            ],

            // Error
            if (_error != null) ...[
              const SizedBox(height: 8),
              Text(_error!, style: theme.textTheme.bodySmall!.copyWith(
                color: const Color(0xFFf44336), fontSize: 11,
              )),
            ],

            const SizedBox(height: 20),

            // Actions
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                if (!backend.isConnected)
                  Text('Engine offline', style: theme.textTheme.bodySmall!.copyWith(
                    color: const Color(0xFFf44336),
                  ))
                else if (_generating)
                  const Row(
                    children: [
                      SizedBox(
                        width: 16, height: 16,
                        child: CircularProgressIndicator(strokeWidth: 1.5),
                      ),
                      SizedBox(width: 8),
                      Text('Generating...', style: TextStyle(fontSize: 12)),
                    ],
                  )
                else ...[
                  TextButton(
                    onPressed: () {
                      _seed++;
                      _generate();
                    },
                    child: const Text('Randomize', style: TextStyle(fontSize: 12)),
                  ),
                  const SizedBox(width: 8),
                  ElevatedButton(
                    onPressed: _generate,
                    style: ElevatedButton.styleFrom(
                      backgroundColor: theme.colorScheme.primary,
                      foregroundColor: Colors.white,
                      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                      textStyle: const TextStyle(fontSize: 12, fontWeight: FontWeight.w700),
                    ),
                    child: const Text('Generate Map'),
                  ),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _NumberInput extends StatelessWidget {
  const _NumberInput({
    required this.value,
    required this.min,
    required this.max,
    required this.onChanged,
    this.enabled = true,
  });
  final int value;
  final int min;
  final int max;
  final ValueChanged<int> onChanged;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        InkWell(
          onTap: enabled && value > min ? () => onChanged(value - 1) : null,
          child: Container(
            width: 24, height: 24,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(4),
              border: Border.all(color: theme.dividerColor),
            ),
            child: const Icon(Icons.remove, size: 14),
          ),
        ),
        Expanded(
          child: Center(
            child: Text(
              '$value',
              style: theme.textTheme.bodyMedium!.copyWith(fontSize: 13),
            ),
          ),
        ),
        InkWell(
          onTap: enabled && value < max ? () => onChanged(value + 1) : null,
          child: Container(
            width: 24, height: 24,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(4),
              border: Border.all(color: theme.dividerColor),
            ),
            child: const Icon(Icons.add, size: 14),
          ),
        ),
      ],
    );
  }
}
