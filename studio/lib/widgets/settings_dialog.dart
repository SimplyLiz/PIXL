import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/claude_provider.dart';
import '../services/llm_provider.dart';
import '../theme/studio_theme.dart';

/// Settings dialog — LLM provider selection, API keys, model config.
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
  final _ollamaUrlController = TextEditingController();
  bool _obscureKey = true;
  bool _saved = false;
  LlmProviderType? _editingProvider;

  @override
  void initState() {
    super.initState();
    final llm = ref.read(claudeProvider);
    _editingProvider = llm.provider;
    _loadKeyForProvider(llm.provider);
    _ollamaUrlController.text = ref.read(claudeProvider.notifier).service.ollamaUrl;
  }

  void _loadKeyForProvider(LlmProviderType provider) {
    final key = ref.read(claudeProvider.notifier).service.getApiKey(provider);
    _apiKeyController.text = key != null && key.isNotEmpty
        ? '${key.substring(0, key.length.clamp(0, 8))}••••••••'
        : '';
  }

  Future<void> _saveApiKey() async {
    final key = _apiKeyController.text.trim();
    if (key.isEmpty || key.contains('••')) return;
    final provider = _editingProvider ?? ref.read(claudeProvider).provider;

    await ref.read(claudeProvider.notifier).setApiKey(provider, key);
    setState(() => _saved = true);
    Future.delayed(const Duration(seconds: 2), () {
      if (mounted) setState(() => _saved = false);
    });
  }

  Future<void> _clearApiKey() async {
    final provider = _editingProvider ?? ref.read(claudeProvider).provider;
    await ref.read(claudeProvider.notifier).clearApiKey(provider);
    _apiKeyController.clear();
    setState(() {});
  }

  @override
  void dispose() {
    _apiKeyController.dispose();
    _ollamaUrlController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final llm = ref.watch(claudeProvider);
    final notifier = ref.read(claudeProvider.notifier);
    final theme = Theme.of(context);
    final models = modelsForProvider(llm.provider);

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

              // Provider Selection
              Text('LLM PROVIDER', style: theme.textTheme.titleSmall),
              const SizedBox(height: 8),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: LlmProviderType.values.map((p) {
                  final isActive = llm.provider == p;
                  final hasKey = notifier.service.hasKeyFor(p);
                  return InkWell(
                    onTap: () {
                      notifier.setProvider(p);
                      setState(() => _editingProvider = p);
                      _loadKeyForProvider(p);
                    },
                    borderRadius: BorderRadius.circular(6),
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
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
                          Icon(
                            isActive ? Icons.radio_button_checked : Icons.radio_button_unchecked,
                            size: 14,
                            color: isActive ? theme.colorScheme.primary : theme.dividerColor,
                          ),
                          const SizedBox(width: 6),
                          Text(p.displayName, style: theme.textTheme.bodySmall!.copyWith(
                            color: isActive ? theme.colorScheme.primary : null,
                            fontWeight: isActive ? FontWeight.w700 : null,
                          )),
                          if (hasKey && p != LlmProviderType.ollama) ...[
                            const SizedBox(width: 4),
                            const Icon(Icons.check_circle, size: 10, color: Color(0xFF4caf50)),
                          ],
                        ],
                      ),
                    ),
                  );
                }).toList(),
              ),
              const SizedBox(height: 20),

              // API Key (not for Ollama)
              if (llm.provider != LlmProviderType.ollama) ...[
                Text('API KEY', style: theme.textTheme.titleSmall),
                const SizedBox(height: 4),
                _apiKeyHint(llm.provider, theme),
                const SizedBox(height: 6),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _apiKeyController,
                        obscureText: _obscureKey,
                        style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                        decoration: InputDecoration(
                          hintText: _keyHintText(llm.provider),
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
                if (notifier.service.hasKeyFor(llm.provider)) ...[
                  const SizedBox(height: 4),
                  Row(
                    children: [
                      const Icon(Icons.check_circle, size: 12, color: Color(0xFF4caf50)),
                      const SizedBox(width: 4),
                      Text('Key configured', style: theme.textTheme.bodySmall!.copyWith(
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
              ],

              // Ollama URL
              if (llm.provider == LlmProviderType.ollama) ...[
                Text('OLLAMA ENDPOINT', style: theme.textTheme.titleSmall),
                const SizedBox(height: 4),
                Text('Make sure Ollama is running locally.',
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
                const SizedBox(height: 6),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _ollamaUrlController,
                        style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                        decoration: InputDecoration(
                          hintText: 'http://localhost:11434',
                          hintStyle: theme.textTheme.bodySmall,
                          isDense: true,
                          contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                          border: OutlineInputBorder(
                            borderRadius: BorderRadius.circular(4),
                            borderSide: StudioTheme.panelBorder,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    ElevatedButton(
                      onPressed: () {
                        notifier.setOllamaUrl(_ollamaUrlController.text.trim());
                      },
                      style: ElevatedButton.styleFrom(
                        backgroundColor: theme.colorScheme.primary,
                        foregroundColor: Colors.white,
                        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                        textStyle: const TextStyle(fontSize: 12),
                      ),
                      child: const Text('Save'),
                    ),
                  ],
                ),
                const SizedBox(height: 20),
              ],

              // Model Selection
              Text('MODEL', style: theme.textTheme.titleSmall),
              const SizedBox(height: 8),
              ...models.map((m) {
                final isSelected = llm.model == m.id;
                return InkWell(
                  onTap: () => notifier.setModel(m.id),
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
                        Text(m.label, style: theme.textTheme.bodySmall!.copyWith(
                          color: isSelected ? theme.colorScheme.primary : null,
                        )),
                      ],
                    ),
                  ),
                );
              }),

              if (llm.lastTokenUsage > 0) ...[
                const SizedBox(height: 12),
                Text(
                  'Last request: ${llm.lastTokenUsage} tokens',
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _apiKeyHint(LlmProviderType provider, ThemeData theme) {
    final (url, hint) = switch (provider) {
      LlmProviderType.anthropic => ('console.anthropic.com', 'Settings > API Keys'),
      LlmProviderType.openai => ('platform.openai.com', 'API Keys'),
      LlmProviderType.gemini => ('aistudio.google.com', 'Get API Key'),
      LlmProviderType.ollama => ('', ''),
    };
    return Text(
      'Get your key at $url > $hint',
      style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
    );
  }

  String _keyHintText(LlmProviderType provider) => switch (provider) {
    LlmProviderType.anthropic => 'sk-ant-...',
    LlmProviderType.openai => 'sk-...',
    LlmProviderType.gemini => 'AIza...',
    LlmProviderType.ollama => '',
  };
}
