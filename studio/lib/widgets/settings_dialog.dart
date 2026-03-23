import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/claude_provider.dart';
import '../services/claude_api.dart';
import '../theme/studio_theme.dart';

/// Settings dialog for API key, model selection, and preferences.
class SettingsDialog extends ConsumerStatefulWidget {
  const SettingsDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const SettingsDialog(),
    );
  }

  @override
  ConsumerState<SettingsDialog> createState() => _SettingsDialogState();
}

class _SettingsDialogState extends ConsumerState<SettingsDialog> {
  final _apiKeyController = TextEditingController();
  bool _obscureKey = true;
  bool _saved = false;

  @override
  void initState() {
    super.initState();
    // Pre-fill with masked key if one exists
    final hasKey = ref.read(claudeProvider).hasApiKey;
    if (hasKey) {
      _apiKeyController.text = 'sk-ant-••••••••••••';
    }
  }

  @override
  void dispose() {
    _apiKeyController.dispose();
    super.dispose();
  }

  Future<void> _saveApiKey() async {
    final key = _apiKeyController.text.trim();
    if (key.isEmpty || key.startsWith('sk-ant-••')) return;

    await ref.read(claudeProvider.notifier).setApiKey(key);
    setState(() => _saved = true);
    Future.delayed(const Duration(seconds: 2), () {
      if (mounted) setState(() => _saved = false);
    });
  }

  Future<void> _clearApiKey() async {
    await ref.read(claudeProvider.notifier).clearApiKey();
    _apiKeyController.clear();
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final claude = ref.watch(claudeProvider);
    final theme = Theme.of(context);

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 420,
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

            // API Key
            Text('ANTHROPIC API KEY', style: theme.textTheme.titleSmall),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _apiKeyController,
                    obscureText: _obscureKey,
                    style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                    decoration: InputDecoration(
                      hintText: 'sk-ant-...',
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
                      suffixIcon: IconButton(
                        icon: Icon(
                          _obscureKey ? Icons.visibility_off : Icons.visibility,
                          size: 16,
                        ),
                        onPressed: () => setState(() => _obscureKey = !_obscureKey),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                ElevatedButton(
                  onPressed: _saveApiKey,
                  style: ElevatedButton.styleFrom(
                    backgroundColor: theme.colorScheme.primary,
                    foregroundColor: Colors.white,
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                    textStyle: const TextStyle(fontSize: 12),
                  ),
                  child: Text(_saved ? 'Saved' : 'Save'),
                ),
              ],
            ),
            if (claude.hasApiKey) ...[
              const SizedBox(height: 4),
              Row(
                children: [
                  const Icon(Icons.check_circle, size: 12, color: Color(0xFF4caf50)),
                  const SizedBox(width: 4),
                  Text('API key configured', style: theme.textTheme.bodySmall!.copyWith(
                    color: const Color(0xFF4caf50), fontSize: 10,
                  )),
                  const Spacer(),
                  InkWell(
                    onTap: _clearApiKey,
                    child: Text('Clear', style: theme.textTheme.bodySmall!.copyWith(
                      color: const Color(0xFFf44336), fontSize: 10,
                      decoration: TextDecoration.underline,
                    )),
                  ),
                ],
              ),
            ],
            const SizedBox(height: 20),

            // Model Selection
            Text('MODEL', style: theme.textTheme.titleSmall),
            const SizedBox(height: 8),
            ...ClaudeModels.models.map((entry) {
              final (modelId, label) = entry;
              final isSelected = claude.model == modelId;
              return InkWell(
                onTap: () => ref.read(claudeProvider.notifier).setModel(modelId),
                borderRadius: BorderRadius.circular(4),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  margin: const EdgeInsets.only(bottom: 4),
                  decoration: BoxDecoration(
                    color: isSelected ? theme.colorScheme.primary.withValues(alpha: 0.15) : null,
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: isSelected ? theme.colorScheme.primary : theme.dividerColor,
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
                      Text(label, style: theme.textTheme.bodySmall!.copyWith(
                        color: isSelected ? theme.colorScheme.primary : null,
                      )),
                    ],
                  ),
                ),
              );
            }),

            if (claude.lastTokenUsage > 0) ...[
              const SizedBox(height: 12),
              Text(
                'Last request: ${claude.lastTokenUsage} tokens',
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
