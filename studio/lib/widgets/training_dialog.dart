import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/claude_provider.dart';
import '../theme/studio_theme.dart';

/// First-launch dialog that explains auto-learn and asks the user to opt in.
/// Shown once — the "asked" flag persists so it won't show again.
class AutoLearnOptInDialog extends ConsumerWidget {
  const AutoLearnOptInDialog({super.key});

  static Future<void> showIfNeeded(BuildContext context, WidgetRef ref) async {
    final notifier = ref.read(claudeProvider.notifier);
    if (notifier.autoLearnAsked) return;

    await showDialog(
      context: context,
      barrierDismissible: false,
      builder: (_) => const AutoLearnOptInDialog(),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 440,
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header with icon
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.primary.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(Icons.auto_awesome, size: 20, color: theme.colorScheme.primary),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Improve PIXL with your art',
                    style: theme.textTheme.bodyMedium!.copyWith(
                      fontSize: 16,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Explanation
            Text(
              'PIXL can learn from every tile you accept. When auto-learn is enabled:',
              style: theme.textTheme.bodySmall!.copyWith(fontSize: 12, height: 1.5),
            ),
            const SizedBox(height: 12),

            _BulletPoint(
              icon: Icons.check_circle_outline,
              color: const Color(0xFF4caf50),
              text: 'Accepted tiles become training data for the local LoRA model',
              theme: theme,
            ),
            const SizedBox(height: 6),
            _BulletPoint(
              icon: Icons.trending_up,
              color: theme.colorScheme.primary,
              text: 'Future generations better match your style and preferences',
              theme: theme,
            ),
            const SizedBox(height: 6),
            _BulletPoint(
              icon: Icons.lock_outline,
              color: const Color(0xFF7e57c2),
              text: 'All data stays on your machine — nothing is sent to the cloud',
              theme: theme,
            ),
            const SizedBox(height: 6),
            _BulletPoint(
              icon: Icons.tune,
              color: const Color(0xFF0277bd),
              text: 'You can retrain anytime from Settings > Training',
              theme: theme,
            ),

            const SizedBox(height: 20),

            // Info box
            Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: theme.colorScheme.primary.withValues(alpha: 0.08),
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: theme.colorScheme.primary.withValues(alpha: 0.2)),
              ),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(Icons.info_outline, size: 14, color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'You can change this anytime in Settings. Auto-learn works best '
                      'with the PIXL LoRA provider, but training data is collected '
                      'regardless of which provider you use.',
                      style: theme.textTheme.bodySmall!.copyWith(fontSize: 10, height: 1.4),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 20),

            // Buttons
            Row(
              children: [
                Expanded(
                  child: OutlinedButton(
                    onPressed: () async {
                      await ref.read(claudeProvider.notifier).markAutoLearnAsked();
                      if (context.mounted) Navigator.of(context).pop();
                    },
                    style: OutlinedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(vertical: 12),
                      side: BorderSide(color: theme.dividerColor),
                    ),
                    child: const Text('Not now', style: TextStyle(fontSize: 12)),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  flex: 2,
                  child: ElevatedButton.icon(
                    onPressed: () async {
                      await ref.read(claudeProvider.notifier).setAutoLearn(true);
                      await ref.read(claudeProvider.notifier).markAutoLearnAsked();
                      if (context.mounted) Navigator.of(context).pop();
                    },
                    icon: const Icon(Icons.auto_awesome, size: 16),
                    label: const Text('Enable Auto-Learn'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: theme.colorScheme.primary,
                      foregroundColor: Colors.white,
                      padding: const EdgeInsets.symmetric(vertical: 12),
                      textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                    ),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _BulletPoint extends StatelessWidget {
  const _BulletPoint({
    required this.icon,
    required this.color,
    required this.text,
    required this.theme,
  });

  final IconData icon;
  final Color color;
  final String text;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 14, color: color),
        const SizedBox(width: 8),
        Expanded(
          child: Text(text, style: theme.textTheme.bodySmall!.copyWith(fontSize: 11, height: 1.3)),
        ),
      ],
    );
  }
}

// ── Training Management Dialog ───────────────────────────────

/// Full training management dialog — stats, export, adapter info.
class TrainingDialog extends ConsumerStatefulWidget {
  const TrainingDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const TrainingDialog(),
    );
  }

  @override
  ConsumerState<TrainingDialog> createState() => _TrainingDialogState();
}

class _TrainingDialogState extends ConsumerState<TrainingDialog> {
  Map<String, dynamic>? _stats;
  Map<String, dynamic>? _feedbackStats;
  bool _loading = true;
  bool _exporting = false;
  String? _exportResult;

  @override
  void initState() {
    super.initState();
    _loadStats();
  }

  Future<void> _loadStats() async {
    setState(() => _loading = true);
    final backend = ref.read(backendProvider.notifier).backend;
    final results = await Future.wait([
      backend.trainingStats(),
      backend.feedbackStats(),
    ]);
    if (mounted) {
      setState(() {
        _stats = results[0];
        _feedbackStats = results[1];
        _loading = false;
      });
    }
  }

  Future<void> _exportTraining() async {
    setState(() {
      _exporting = true;
      _exportResult = null;
    });
    final resp = await ref.read(backendProvider.notifier).backend.exportTraining();
    if (mounted) {
      setState(() {
        _exporting = false;
        if (resp['ok'] == true) {
          _exportResult = 'Exported ${resp['exported']} training pairs';
        } else {
          _exportResult = 'Export failed: ${resp['error'] ?? 'unknown'}';
        }
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final llm = ref.watch(claudeProvider);
    final notifier = ref.read(claudeProvider.notifier);
    final theme = Theme.of(context);
    final backend = ref.watch(backendProvider);

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 460,
        padding: const EdgeInsets.all(20),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header
              Row(
                children: [
                  Icon(Icons.model_training, size: 18, color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Text('Training', style: theme.textTheme.bodyMedium!.copyWith(
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

              // Auto-learn toggle
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: llm.autoLearn
                      ? theme.colorScheme.primary.withValues(alpha: 0.08)
                      : null,
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(
                    color: llm.autoLearn
                        ? theme.colorScheme.primary.withValues(alpha: 0.3)
                        : theme.dividerColor,
                  ),
                ),
                child: Row(
                  children: [
                    Icon(
                      llm.autoLearn ? Icons.auto_awesome : Icons.auto_awesome_outlined,
                      size: 18,
                      color: llm.autoLearn ? theme.colorScheme.primary : theme.dividerColor,
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Auto-Learn', style: theme.textTheme.bodySmall!.copyWith(
                            fontWeight: FontWeight.w600, fontSize: 12,
                          )),
                          Text(
                            'Accepted tiles are saved as training data for future LoRA fine-tuning.',
                            style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                          ),
                        ],
                      ),
                    ),
                    Switch(
                      value: llm.autoLearn,
                      onChanged: (v) => notifier.setAutoLearn(v),
                      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),

              // Stats
              Text('TRAINING DATA', style: theme.textTheme.titleSmall),
              const SizedBox(height: 8),

              if (!backend.isConnected)
                _infoBox('Engine not connected. Start the engine to see training stats.',
                    Icons.warning_amber_rounded, StudioTheme.error, theme)
              else if (_loading)
                const Center(child: Padding(
                  padding: EdgeInsets.all(16),
                  child: CircularProgressIndicator(strokeWidth: 2),
                ))
              else if (_stats != null) ...[
                _statRow('Training pairs (accepted tiles)', '${_stats!['training_pairs'] ?? 0}', theme),
                _statRow('Total feedback events', '${_stats!['total_feedback'] ?? 0}', theme),
                _statRow('Acceptance rate',
                    '${((_stats!['acceptance_rate'] as num? ?? 0) * 100).round()}%', theme),
                _statRow('Total accepts', '${_stats!['total_accepts'] ?? 0}', theme),
                _statRow('Total rejects', '${_stats!['total_rejects'] ?? 0}', theme),
              ],
              const SizedBox(height: 16),

              // Feedback insights
              if (_feedbackStats != null && (_feedbackStats!['total_accepts'] as int? ?? 0) > 0) ...[
                Text('FEEDBACK INSIGHTS', style: theme.textTheme.titleSmall),
                const SizedBox(height: 8),
                _statRow('Avg accepted style score',
                    '${((_feedbackStats!['avg_accepted_score'] as num? ?? 0) * 100).round()}%', theme),
                _statRow('Avg rejected style score',
                    '${((_feedbackStats!['avg_rejected_score'] as num? ?? 0) * 100).round()}%', theme),
                if (_feedbackStats!['top_reject_reasons'] is Map) ...[
                  const SizedBox(height: 4),
                  Text('Top rejection reasons:', style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 10, fontWeight: FontWeight.w600,
                  )),
                  const SizedBox(height: 2),
                  ...(_feedbackStats!['top_reject_reasons'] as Map).entries.take(5).map((e) {
                    return Padding(
                      padding: const EdgeInsets.only(left: 8, bottom: 1),
                      child: Row(
                        children: [
                          Container(
                            width: 6, height: 6,
                            margin: const EdgeInsets.only(right: 6),
                            decoration: BoxDecoration(
                              color: theme.colorScheme.primary.withValues(alpha: 0.5),
                              shape: BoxShape.circle,
                            ),
                          ),
                          Expanded(
                            child: Text(
                              '${e.key}',
                              style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                            ),
                          ),
                          Text(
                            '${e.value}x',
                            style: theme.textTheme.bodySmall!.copyWith(fontSize: 10, fontWeight: FontWeight.w600),
                          ),
                        ],
                      ),
                    );
                  }),
                ],
                const SizedBox(height: 16),
              ],

              // Adapter info
              if (_stats?['adapter'] != null) ...[
                Text('ADAPTER', style: theme.textTheme.titleSmall),
                const SizedBox(height: 8),
                _statRow('Model', '${_stats!['adapter']['model'] ?? 'none'}', theme),
                _statRow('Adapter', '${_stats!['adapter']['adapter_path'] ?? 'none'}', theme),
                const SizedBox(height: 16),
              ],

              // Export button
              Text('EXPORT', style: theme.textTheme.titleSmall),
              const SizedBox(height: 8),
              Text(
                'Export accepted tiles as JSONL training data. Use this to retrain '
                'the LoRA adapter with your curated tiles.',
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  ElevatedButton.icon(
                    onPressed: (_exporting || !backend.isConnected)
                        ? null
                        : _exportTraining,
                    icon: _exporting
                        ? const SizedBox(
                            width: 14, height: 14,
                            child: CircularProgressIndicator(strokeWidth: 1.5, color: Colors.white),
                          )
                        : const Icon(Icons.file_download, size: 14),
                    label: Text(_exporting ? 'Exporting...' : 'Export Training Data'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: theme.colorScheme.primary,
                      foregroundColor: Colors.white,
                      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                      textStyle: const TextStyle(fontSize: 12),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.refresh, size: 16),
                    tooltip: 'Refresh stats',
                    onPressed: _loadStats,
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
              if (_exportResult != null) ...[
                const SizedBox(height: 6),
                Text(
                  _exportResult!,
                  style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 10,
                    color: _exportResult!.contains('failed') ? StudioTheme.error : const Color(0xFF4caf50),
                  ),
                ),
              ],

              const SizedBox(height: 16),

              // Training instructions
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
                    Text('How to retrain', style: theme.textTheme.bodySmall!.copyWith(
                      fontWeight: FontWeight.w600, fontSize: 11,
                    )),
                    const SizedBox(height: 4),
                    Text(
                      '1. Export training data above\n'
                      '2. Run: cd training && ./train_matched.sh\n'
                      '3. Update adapter path in Settings > PIXL LoRA',
                      style: theme.textTheme.bodySmall!.copyWith(
                        fontSize: 10,
                        fontFamily: 'monospace',
                        height: 1.5,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _statRow(String label, String value, ThemeData theme) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        children: [
          Expanded(child: Text(label, style: theme.textTheme.bodySmall!.copyWith(fontSize: 11))),
          Text(value, style: theme.textTheme.bodySmall!.copyWith(
            fontSize: 11, fontWeight: FontWeight.w600,
          )),
        ],
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
          Expanded(child: Text(text, style: theme.textTheme.bodySmall!.copyWith(fontSize: 11))),
        ],
      ),
    );
  }
}
