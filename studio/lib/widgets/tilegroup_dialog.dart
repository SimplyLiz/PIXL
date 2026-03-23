import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../providers/chat_provider.dart';
import '../providers/claude_provider.dart';
import '../providers/style_provider.dart';
import '../theme/studio_theme.dart';
import '../utils/grid_parser.dart';

/// Standard 4-bit autotile variant names.
const _autotileVariants = [
  'solid',
  'top',
  'bottom',
  'left',
  'right',
  'top_left',
  'top_right',
  'bottom_left',
  'bottom_right',
  'horizontal',
  'vertical',
  'inner_top_left',
  'inner_top_right',
  'inner_bottom_left',
  'inner_bottom_right',
];

/// Dialog for generating a full tilegroup (all autotile variants).
class TilegroupDialog extends ConsumerStatefulWidget {
  const TilegroupDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      barrierDismissible: false,
      builder: (_) => const TilegroupDialog(),
    );
  }

  @override
  ConsumerState<TilegroupDialog> createState() => _TilegroupDialogState();
}

class _TilegroupDialogState extends ConsumerState<TilegroupDialog> {
  final _nameController = TextEditingController(text: 'wall');
  final _descController = TextEditingController(text: 'dungeon stone wall');

  bool _generating = false;
  int _completedCount = 0;
  int _failedCount = 0;
  String _currentVariant = '';
  final Map<String, _VariantStatus> _variantStatuses = {};

  // Subset of variants to generate
  final Set<String> _selectedVariants = {..._autotileVariants};

  Future<void> _generate() async {
    final backend = ref.read(backendProvider);
    final claude = ref.read(claudeProvider);

    if (!backend.isConnected) {
      _showError('Engine not connected');
      return;
    }
    if (!claude.hasApiKey) {
      _showError('No API key configured');
      return;
    }

    final baseName = _nameController.text.trim();
    final description = _descController.text.trim();
    if (baseName.isEmpty) {
      _showError('Enter a tile group name');
      return;
    }

    setState(() {
      _generating = true;
      _completedCount = 0;
      _failedCount = 0;
      _variantStatuses.clear();
      for (final v in _selectedVariants) {
        _variantStatuses[v] = _VariantStatus.pending;
      }
    });

    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';
    final style = ref.read(styleProvider);

    for (final variant in _selectedVariants) {
      if (!_generating) break; // cancelled

      setState(() {
        _currentVariant = variant;
        _variantStatuses[variant] = _VariantStatus.generating;
      });

      final tileName = '${baseName}_$variant';
      final prompt =
          'Generate a $sizeStr pixel art tile: $description. '
          'This is the "$variant" variant of the "$baseName" autotile group. '
          '${style.toPromptFragment()}. '
          'The tile must be edge-compatible with other variants in the group.';

      // Get enriched context
      final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
        prompt: prompt,
        size: sizeStr,
      );

      if (ctx.containsKey('error')) {
        setState(() {
          _variantStatuses[variant] = _VariantStatus.failed;
          _failedCount++;
        });
        continue;
      }

      // Generate via Claude
      final resp = await ref.read(claudeProvider.notifier).generateTile(
        systemPrompt: ctx['system_prompt'] as String? ?? '',
        userPrompt: ctx['user_prompt'] as String? ?? prompt,
      );

      if (resp.isError) {
        setState(() {
          _variantStatuses[variant] = _VariantStatus.failed;
          _failedCount++;
        });
        continue;
      }

      // Extract grid
      final grid = extractGrid(resp.content);
      if (grid == null) {
        setState(() {
          _variantStatuses[variant] = _VariantStatus.failed;
          _failedCount++;
        });
        continue;
      }

      // Create tile via backend
      final createResp = await ref.read(backendProvider.notifier).createTile(
        name: tileName,
        palette: ctx['palette'] as String? ?? 'default',
        size: sizeStr,
        grid: grid,
        tags: [baseName, 'autotile', variant],
      );

      if (createResp.containsKey('error')) {
        setState(() {
          _variantStatuses[variant] = _VariantStatus.failed;
          _failedCount++;
        });
      } else {
        setState(() {
          _variantStatuses[variant] = _VariantStatus.done;
          _completedCount++;
        });
      }
    }

    setState(() => _generating = false);

    // Refresh tile list
    ref.read(backendProvider.notifier).refreshTiles();

    // Log to chat
    ref.read(chatProvider.notifier).addAssistantMessage(
      '**Tilegroup "$baseName"** complete: '
      '$_completedCount generated, $_failedCount failed '
      '(${_selectedVariants.length} total variants)',
    );
  }

  void _cancel() {
    setState(() => _generating = false);
  }

  void _showError(String msg) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(msg), duration: const Duration(seconds: 2)),
    );
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final total = _selectedVariants.length;

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
                Icon(Icons.grid_view, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Generate Tilegroup', style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 16, fontWeight: FontWeight.w700,
                )),
                const Spacer(),
                if (!_generating)
                  InkWell(
                    onTap: () => Navigator.of(context).pop(),
                    child: const Icon(Icons.close, size: 18),
                  ),
              ],
            ),
            const SizedBox(height: 16),

            // Name + description
            Text('GROUP NAME', style: theme.textTheme.titleSmall),
            const SizedBox(height: 4),
            TextField(
              controller: _nameController,
              enabled: !_generating,
              style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
              decoration: _inputDecoration(theme, 'e.g. wall, floor, grass'),
            ),
            const SizedBox(height: 12),
            Text('DESCRIPTION', style: theme.textTheme.titleSmall),
            const SizedBox(height: 4),
            TextField(
              controller: _descController,
              enabled: !_generating,
              style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
              maxLines: 2,
              decoration: _inputDecoration(theme, 'e.g. dark stone dungeon wall with moss'),
            ),
            const SizedBox(height: 16),

            // Variant selection
            Text('VARIANTS ($total selected)', style: theme.textTheme.titleSmall),
            const SizedBox(height: 6),
            Wrap(
              spacing: 4,
              runSpacing: 4,
              children: _autotileVariants.map((v) {
                final isSelected = _selectedVariants.contains(v);
                final status = _variantStatuses[v];

                Color borderColor = theme.dividerColor;
                Color? bgColor;
                if (status == _VariantStatus.done) {
                  borderColor = const Color(0xFF4caf50);
                  bgColor = const Color(0xFF4caf50).withValues(alpha: 0.15);
                } else if (status == _VariantStatus.failed) {
                  borderColor = const Color(0xFFf44336);
                  bgColor = const Color(0xFFf44336).withValues(alpha: 0.15);
                } else if (status == _VariantStatus.generating) {
                  borderColor = theme.colorScheme.primary;
                  bgColor = theme.colorScheme.primary.withValues(alpha: 0.15);
                } else if (isSelected) {
                  borderColor = theme.colorScheme.primary;
                }

                return InkWell(
                  onTap: _generating
                      ? null
                      : () => setState(() {
                            if (isSelected) {
                              _selectedVariants.remove(v);
                            } else {
                              _selectedVariants.add(v);
                            }
                          }),
                  borderRadius: BorderRadius.circular(4),
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
                    decoration: BoxDecoration(
                      color: bgColor,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(color: borderColor),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        if (status == _VariantStatus.generating)
                          const Padding(
                            padding: EdgeInsets.only(right: 4),
                            child: SizedBox(
                              width: 8, height: 8,
                              child: CircularProgressIndicator(strokeWidth: 1),
                            ),
                          ),
                        if (status == _VariantStatus.done)
                          const Padding(
                            padding: EdgeInsets.only(right: 4),
                            child: Icon(Icons.check, size: 10, color: Color(0xFF4caf50)),
                          ),
                        if (status == _VariantStatus.failed)
                          const Padding(
                            padding: EdgeInsets.only(right: 4),
                            child: Icon(Icons.close, size: 10, color: Color(0xFFf44336)),
                          ),
                        Text(v, style: TextStyle(
                          fontSize: 9,
                          color: isSelected || status != null
                              ? theme.colorScheme.primary
                              : theme.textTheme.bodySmall?.color,
                        )),
                      ],
                    ),
                  ),
                );
              }).toList(),
            ),

            // Progress
            if (_generating) ...[
              const SizedBox(height: 16),
              LinearProgressIndicator(
                value: (_completedCount + _failedCount) / total,
                backgroundColor: theme.dividerColor,
              ),
              const SizedBox(height: 6),
              Text(
                'Generating: $_currentVariant ($_completedCount/$total)',
                style: theme.textTheme.bodySmall,
              ),
            ],

            const SizedBox(height: 20),

            // Actions
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                if (_generating)
                  TextButton(
                    onPressed: _cancel,
                    child: const Text('Cancel', style: TextStyle(fontSize: 12)),
                  )
                else ...[
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text('Close', style: TextStyle(fontSize: 12)),
                  ),
                  const SizedBox(width: 8),
                  ElevatedButton(
                    onPressed: _selectedVariants.isNotEmpty ? _generate : null,
                    style: ElevatedButton.styleFrom(
                      backgroundColor: theme.colorScheme.primary,
                      foregroundColor: Colors.white,
                      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                      textStyle: const TextStyle(fontSize: 12, fontWeight: FontWeight.w700),
                    ),
                    child: Text('Generate $total Tiles'),
                  ),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  InputDecoration _inputDecoration(ThemeData theme, String hint) {
    return InputDecoration(
      hintText: hint,
      hintStyle: theme.textTheme.bodySmall,
      isDense: true,
      contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(4),
        borderSide: StudioTheme.panelBorder,
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(4),
        borderSide: BorderSide(color: theme.colorScheme.primary),
      ),
    );
  }
}

enum _VariantStatus { pending, generating, done, failed }
