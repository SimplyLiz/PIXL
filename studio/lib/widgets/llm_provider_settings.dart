import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/claude_provider.dart';
import '../services/llm_provider.dart';
import '../theme/studio_theme.dart';
import 'training_dialog.dart';

/// LLM provider settings — provider selection, API keys, model config.
class LlmProviderSettings extends ConsumerStatefulWidget {
  const LlmProviderSettings({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const LlmProviderSettings(),
    );
  }

  @override
  ConsumerState<LlmProviderSettings> createState() => _LlmProviderSettingsState();
}

class _LlmProviderSettingsState extends ConsumerState<LlmProviderSettings> {
  final _apiKeyController = TextEditingController();
  final _ollamaUrlController = TextEditingController();
  final _pullModelController = TextEditingController();
  final _pixlModelController = TextEditingController();
  final _pixlAdapterController = TextEditingController();
  bool _obscureKey = true;
  bool _saved = false;
  LlmProviderType? _editingProvider;
  bool _isPulling = false;
  double _pullProgress = 0;
  String? _pullError;

  @override
  void initState() {
    super.initState();
    final llm = ref.read(claudeProvider);
    _editingProvider = llm.provider;
    _loadKeyForProvider(llm.provider);
    _ollamaUrlController.text = ref.read(claudeProvider.notifier).service.ollamaUrl;
    _pixlModelController.text = ref.read(claudeProvider.notifier).service.pixlModel;
    _pixlAdapterController.text = ref.read(claudeProvider.notifier).service.pixlAdapter;
    // Kick off model fetch for the current provider
    Future.microtask(() => ref.read(claudeProvider.notifier).fetchModels());
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

  Future<void> _startPull(String name) async {
    if (name.trim().isEmpty || _isPulling) return;
    setState(() {
      _isPulling = true;
      _pullProgress = 0;
      _pullError = null;
    });
    final notifier = ref.read(claudeProvider.notifier);
    await for (final progress in notifier.pullOllamaModel(name.trim())) {
      if (!mounted) return;
      if (progress < 0) {
        setState(() {
          _isPulling = false;
          _pullError = 'Failed to pull $name';
        });
        return;
      }
      setState(() => _pullProgress = progress);
    }
    if (mounted) {
      setState(() {
        _isPulling = false;
        _pullModelController.clear();
      });
    }
  }

  @override
  void dispose() {
    _apiKeyController.dispose();
    _ollamaUrlController.dispose();
    _pullModelController.dispose();
    _pixlModelController.dispose();
    _pixlAdapterController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final llm = ref.watch(claudeProvider);
    final notifier = ref.read(claudeProvider.notifier);
    final theme = Theme.of(context);
    // Use fetched models if available, otherwise fall back to hardcoded
    final models = llm.availableModels.isNotEmpty
        ? llm.availableModels
        : modelsForProvider(llm.provider);

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
                  Icon(Icons.smart_toy, size: 18, color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Text('LLM Settings', style: theme.textTheme.bodyMedium!.copyWith(
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

              // Scrollable content
              Flexible(
                child: SingleChildScrollView(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [

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
                    onTap: () async {
                      await notifier.setProvider(p);
                      setState(() => _editingProvider = p);
                      _loadKeyForProvider(p);
                      notifier.fetchModels();
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
                          if (hasKey && p != LlmProviderType.ollama && p != LlmProviderType.pixlLocal) ...[
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

              // API Key (not for Ollama or PIXL Local)
              if (llm.provider != LlmProviderType.ollama &&
                  llm.provider != LlmProviderType.pixlLocal) ...[
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
                        color: StudioTheme.success, fontSize: 10,
                      )),
                      const Spacer(),
                      InkWell(
                        onTap: _clearApiKey,
                        child: Text('Clear', style: theme.textTheme.bodySmall!.copyWith(
                          color: StudioTheme.error, fontSize: 10,
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

              // PIXL Local LoRA settings
              if (llm.provider == LlmProviderType.pixlLocal) ...[
                Text('ON-DEVICE INFERENCE', style: theme.textTheme.titleSmall),
                const SizedBox(height: 4),
                Text(
                  'Runs locally via mlx_lm.server with your trained LoRA adapter. '
                  'Requires pip install mlx-lm.',
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                ),
                const SizedBox(height: 10),
                Text('Base Model', style: theme.textTheme.bodySmall!.copyWith(
                  fontWeight: FontWeight.w600, fontSize: 11,
                )),
                const SizedBox(height: 4),
                TextField(
                  controller: _pixlModelController,
                  style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                  decoration: InputDecoration(
                    hintText: 'mlx-community/Qwen2.5-3B-Instruct-4bit',
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
                    prefixIcon: const Icon(Icons.memory, size: 16),
                  ),
                ),
                const SizedBox(height: 10),
                Text('LoRA Adapter Path', style: theme.textTheme.bodySmall!.copyWith(
                  fontWeight: FontWeight.w600, fontSize: 11,
                )),
                const SizedBox(height: 4),
                TextField(
                  controller: _pixlAdapterController,
                  style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                  decoration: InputDecoration(
                    hintText: 'training/adapters/pixl-lora-v2',
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
                    prefixIcon: const Icon(Icons.folder_open, size: 16),
                  ),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: ElevatedButton.icon(
                        onPressed: () async {
                          await notifier.setPixlModel(_pixlModelController.text.trim());
                          await notifier.setPixlAdapter(_pixlAdapterController.text.trim());
                          setState(() => _saved = true);
                          Future.delayed(const Duration(seconds: 2), () {
                            if (mounted) setState(() => _saved = false);
                          });
                        },
                        icon: Icon(_saved ? Icons.check : Icons.save, size: 14),
                        label: Text(_saved ? 'Saved' : 'Save Configuration'),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: theme.colorScheme.primary,
                          foregroundColor: Colors.white,
                          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                          textStyle: const TextStyle(fontSize: 12),
                        ),
                      ),
                    ),
                  ],
                ),
                if (notifier.service.hasPixlAdapter) ...[
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      const Icon(Icons.check_circle, size: 12, color: Color(0xFF4caf50)),
                      const SizedBox(width: 4),
                      Expanded(
                        child: Text(
                          'Adapter configured. Engine will auto-start mlx_lm.server on first generate.',
                          style: theme.textTheme.bodySmall!.copyWith(
                            color: const Color(0xFF4caf50), fontSize: 10,
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
                const SizedBox(height: 8),
                const _MlxSetupCheck(),
                const SizedBox(height: 20),
              ],

              // Model Selection
              Row(
                children: [
                  Text('MODEL', style: theme.textTheme.titleSmall),
                  if (llm.isFetchingModels) ...[
                    const SizedBox(width: 8),
                    const SizedBox(
                      width: 12, height: 12,
                      child: CircularProgressIndicator(strokeWidth: 1.5),
                    ),
                  ],
                ],
              ),
              const SizedBox(height: 8),
              if (llm.provider == LlmProviderType.ollama &&
                  models.isEmpty &&
                  !llm.isFetchingModels) ...[
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.errorContainer.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(color: theme.dividerColor),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.warning_amber_rounded, size: 14, color: StudioTheme.error),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'Ollama not running or no models installed. Pull a model below.',
                          style: theme.textTheme.bodySmall!.copyWith(fontSize: 11),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 8),
              ],
              ...models.map((m) {
                final isSelected = llm.model == m.id;
                final isOllama = llm.provider == LlmProviderType.ollama;
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
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(m.label, style: theme.textTheme.bodySmall!.copyWith(
                                color: isSelected ? theme.colorScheme.primary : null,
                              )),
                              const SizedBox(height: 2),
                              Row(
                                children: [
                                  _capBadge(m.costLabel, _costColor(m.cost), theme),
                                  if (m.contextLabel.isNotEmpty)
                                    _capBadge(m.contextLabel, theme.dividerColor, theme),
                                  if (m.vision)
                                    _capBadge('vision', const Color(0xFF6a1b9a), theme),
                                  if (m.thinking)
                                    _capBadge('thinking', const Color(0xFF0277bd), theme),
                                  if (m.local)
                                    _capBadge('local', const Color(0xFF2e7d32), theme),
                                ],
                              ),
                            ],
                          ),
                        ),
                        if (isOllama)
                          InkWell(
                            onTap: () async {
                              final confirm = await showDialog<bool>(
                                context: context,
                                builder: (ctx) => AlertDialog(
                                  title: const Text('Delete model?'),
                                  content: Text('Remove ${m.id} from Ollama?'),
                                  actions: [
                                    TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
                                    TextButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Delete')),
                                  ],
                                ),
                              );
                              if (confirm == true) {
                                await notifier.deleteOllamaModel(m.id);
                              }
                            },
                            child: Icon(Icons.delete_outline, size: 14, color: theme.dividerColor),
                          ),
                      ],
                    ),
                  ),
                );
              }),

              // Ollama pull section
              if (llm.provider == LlmProviderType.ollama) ...[
                const SizedBox(height: 16),
                Text('PULL MODEL', style: theme.textTheme.titleSmall),
                const SizedBox(height: 6),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _pullModelController,
                        enabled: !_isPulling,
                        style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                        decoration: InputDecoration(
                          hintText: 'e.g. llama3.2, qwen3:8b',
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
                      onPressed: _isPulling ? null : () => _startPull(_pullModelController.text),
                      style: ElevatedButton.styleFrom(
                        backgroundColor: theme.colorScheme.primary,
                        foregroundColor: Colors.white,
                        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                        textStyle: const TextStyle(fontSize: 12),
                      ),
                      child: const Text('Pull'),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                // Suggested models (only show ones not already installed)
                Builder(builder: (_) {
                  final installedIds = models.map((m) => m.id).toSet();
                  final suggestions = ollamaSuggestions
                      .where((s) => !installedIds.contains(s.id))
                      .toList();
                  if (suggestions.isEmpty) return const SizedBox.shrink();
                  return Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    children: suggestions.map((s) => ActionChip(
                      label: Text(s.label, style: const TextStyle(fontSize: 10)),
                      onPressed: _isPulling ? null : () => _startPull(s.id),
                      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      visualDensity: VisualDensity.compact,
                    )).toList(),
                  );
                }),
                if (_isPulling) ...[
                  const SizedBox(height: 8),
                  LinearProgressIndicator(value: _pullProgress > 0 ? _pullProgress : null),
                  const SizedBox(height: 4),
                  Text(
                    _pullProgress > 0
                        ? '${(_pullProgress * 100).toStringAsFixed(0)}%'
                        : 'Starting download...',
                    style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                  ),
                ],
                if (_pullError != null) ...[
                  const SizedBox(height: 4),
                  Text(_pullError!, style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 10, color: StudioTheme.error,
                  )),
                ],
              ],

              // Knowledge Base toggle
              Builder(builder: (_) {
                final backend = ref.watch(backendProvider);
                if (!backend.knowledgeAvailable) return const SizedBox.shrink();
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const SizedBox(height: 20),
                    Text('KNOWLEDGE BASE', style: theme.textTheme.titleSmall),
                    const SizedBox(height: 4),
                    Text(
                      'Inject relevant pixel art technique knowledge into generation prompts.',
                      style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                    ),
                    const SizedBox(height: 6),
                    InkWell(
                      onTap: () {
                        ref.read(backendProvider.notifier)
                            .setKnowledgeEnabled(!backend.knowledgeEnabled);
                      },
                      borderRadius: BorderRadius.circular(6),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                        decoration: BoxDecoration(
                          borderRadius: BorderRadius.circular(6),
                          border: Border.all(
                            color: backend.knowledgeEnabled
                                ? theme.colorScheme.primary
                                : theme.dividerColor,
                          ),
                          color: backend.knowledgeEnabled
                              ? theme.colorScheme.primary.withValues(alpha: 0.15)
                              : null,
                        ),
                        child: Row(
                          children: [
                            Icon(
                              backend.knowledgeEnabled
                                  ? Icons.auto_stories
                                  : Icons.auto_stories_outlined,
                              size: 16,
                              color: backend.knowledgeEnabled
                                  ? theme.colorScheme.primary
                                  : theme.dividerColor,
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                backend.knowledgeEnabled
                                    ? 'Knowledge injection enabled'
                                    : 'Knowledge injection disabled',
                                style: theme.textTheme.bodySmall!.copyWith(
                                  color: backend.knowledgeEnabled
                                      ? theme.colorScheme.primary
                                      : null,
                                  fontWeight: backend.knowledgeEnabled
                                      ? FontWeight.w600
                                      : null,
                                ),
                              ),
                            ),
                            Switch(
                              value: backend.knowledgeEnabled,
                              onChanged: (v) {
                                ref.read(backendProvider.notifier)
                                    .setKnowledgeEnabled(v);
                              },
                              materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                );
              }),

              // Auto-learn + Training link
              const SizedBox(height: 16),
              Row(
                children: [
                  Icon(
                    llm.autoLearn ? Icons.auto_awesome : Icons.auto_awesome_outlined,
                    size: 14,
                    color: llm.autoLearn ? theme.colorScheme.primary : theme.dividerColor,
                  ),
                  const SizedBox(width: 6),
                  Text('Auto-Learn', style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 11, fontWeight: FontWeight.w600,
                  )),
                  const SizedBox(width: 4),
                  Text(
                    '(accepted tiles become training data)',
                    style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                  ),
                  const Spacer(),
                  Switch(
                    value: llm.autoLearn,
                    onChanged: (v) => notifier.setAutoLearn(v),
                    materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                ],
              ),
              InkWell(
                onTap: () {
                  Navigator.of(context).pop();
                  TrainingDialog.show(context);
                },
                borderRadius: BorderRadius.circular(4),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    children: [
                      Icon(Icons.model_training, size: 14, color: theme.colorScheme.primary),
                      const SizedBox(width: 6),
                      Text('Open Training...', style: theme.textTheme.bodySmall!.copyWith(
                        fontSize: 11,
                        color: theme.colorScheme.primary,
                        decoration: TextDecoration.underline,
                      )),
                    ],
                  ),
                ),
              ),

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

              // Done button — always visible at bottom
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
      ),
    );
  }

  Widget _apiKeyHint(LlmProviderType provider, ThemeData theme) {
    final (url, hint) = switch (provider) {
      LlmProviderType.anthropic => ('console.anthropic.com', 'Settings > API Keys'),
      LlmProviderType.openai => ('platform.openai.com', 'API Keys'),
      LlmProviderType.gemini => ('aistudio.google.com', 'Get API Key'),
      LlmProviderType.ollama => ('', ''),
      LlmProviderType.pixlLocal => ('', ''),
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
    LlmProviderType.pixlLocal => '',
  };

  Color _costColor(ModelCost cost) => switch (cost) {
    ModelCost.free => const Color(0xFF2e7d32),
    ModelCost.cheap => const Color(0xFF558b2f),
    ModelCost.medium => const Color(0xFFf9a825),
    ModelCost.high => const Color(0xFFe65100),
  };

  Widget _capBadge(String text, Color color, ThemeData theme) {
    return Container(
      margin: const EdgeInsets.only(right: 4),
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(3),
        border: Border.all(color: color.withValues(alpha: 0.4), width: 0.5),
      ),
      child: Text(text, style: TextStyle(
        fontSize: 9,
        color: color,
        fontWeight: FontWeight.w600,
      )),
    );
  }
}

/// Checks if mlx-lm is available and shows setup instructions if not.
class _MlxSetupCheck extends StatefulWidget {
  const _MlxSetupCheck();

  @override
  State<_MlxSetupCheck> createState() => _MlxSetupCheckState();
}

class _MlxSetupCheckState extends State<_MlxSetupCheck> {
  bool? _available;
  String _pythonPath = '';
  bool _installing = false;

  @override
  void initState() {
    super.initState();
    _check();
  }

  Future<void> _check() async {
    // Try to find python with mlx_lm — same search as the Rust backend
    final candidates = [
      '../training/.venv/bin/python',
      'training/.venv/bin/python',
      '.venv/bin/python',
      'python3',
      'python',
    ];

    for (final py in candidates) {
      try {
        final result = await Process.run(py, ['-c', 'import mlx_lm; print("ok")']);
        if (result.exitCode == 0 && result.stdout.toString().contains('ok')) {
          setState(() {
            _available = true;
            _pythonPath = py;
          });
          return;
        }
      } catch (_) {}
    }
    setState(() => _available = false);
  }

  Future<void> _install() async {
    setState(() => _installing = true);
    try {
      final venvPath = '../training/.venv';
      // Create venv — try python3 first, fall back to python
      for (final py in ['python3', 'python']) {
        try {
          final r = await Process.run(py, ['-m', 'venv', venvPath]);
          if (r.exitCode == 0) break;
        } catch (_) {}
      }
      // Install mlx-lm using the venv's own python -m pip (always works)
      final result = await Process.run(
        '$venvPath/bin/python',
        ['-m', 'pip', 'install', 'mlx-lm'],
      );
      if (result.exitCode == 0) {
        await _check();
      }
    } catch (_) {}
    if (mounted) setState(() => _installing = false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (_available == null) {
      return Row(
        children: [
          const SizedBox(width: 10, height: 10, child: CircularProgressIndicator(strokeWidth: 1)),
          const SizedBox(width: 6),
          Text('Checking mlx-lm...', style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
        ],
      );
    }

    if (_available!) {
      return Row(
        children: [
          const Icon(Icons.check_circle, size: 12, color: Color(0xFF4caf50)),
          const SizedBox(width: 4),
          Expanded(
            child: Text(
              'mlx-lm ready ($_pythonPath)',
              style: theme.textTheme.bodySmall!.copyWith(fontSize: 10, color: const Color(0xFF4caf50)),
            ),
          ),
        ],
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.warning_amber_rounded, size: 12, color: Color(0xFFe8a045)),
            const SizedBox(width: 4),
            Expanded(
              child: Text(
                'mlx-lm not found. Required for on-device inference.',
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 10, color: const Color(0xFFe8a045)),
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Row(
          children: [
            ElevatedButton(
              onPressed: _installing ? null : _install,
              style: ElevatedButton.styleFrom(
                backgroundColor: theme.colorScheme.primary,
                foregroundColor: Colors.white,
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                textStyle: const TextStyle(fontSize: 10),
              ),
              child: _installing
                  ? const SizedBox(width: 10, height: 10, child: CircularProgressIndicator(strokeWidth: 1, color: Colors.white))
                  : const Text('Install mlx-lm'),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                'or run: pip install mlx-lm',
                style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
