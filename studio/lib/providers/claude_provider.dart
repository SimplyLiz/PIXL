import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/llm_provider.dart';

/// State for the LLM connection (multi-provider).
class LlmState {
  const LlmState({
    this.provider = LlmProviderType.anthropic,
    this.hasApiKey = false,
    this.model = 'claude-sonnet-4-20250514',
    this.isGenerating = false,
    this.lastTokenUsage = 0,
    this.ollamaModels = const [],
    this.availableModels = const [],
    this.isFetchingModels = false,
  });

  final LlmProviderType provider;
  final bool hasApiKey;
  final String model;
  final bool isGenerating;
  final int lastTokenUsage;
  final List<LlmModel> ollamaModels;
  final List<LlmModel> availableModels;
  final bool isFetchingModels;

  LlmState copyWith({
    LlmProviderType? provider,
    bool? hasApiKey,
    String? model,
    bool? isGenerating,
    int? lastTokenUsage,
    List<LlmModel>? ollamaModels,
    List<LlmModel>? availableModels,
    bool? isFetchingModels,
  }) {
    return LlmState(
      provider: provider ?? this.provider,
      hasApiKey: hasApiKey ?? this.hasApiKey,
      model: model ?? this.model,
      isGenerating: isGenerating ?? this.isGenerating,
      lastTokenUsage: lastTokenUsage ?? this.lastTokenUsage,
      ollamaModels: ollamaModels ?? this.ollamaModels,
      availableModels: availableModels ?? this.availableModels,
      isFetchingModels: isFetchingModels ?? this.isFetchingModels,
    );
  }
}

class LlmNotifier extends StateNotifier<LlmState> {
  LlmNotifier() : super(const LlmState());

  final LlmService _service = LlmService();
  LlmService get service => _service;

  Future<void> init() async {
    await _service.init();
    state = state.copyWith(
      provider: _service.provider,
      hasApiKey: _service.hasApiKey,
      model: _service.model,
    );
  }

  Future<void> setProvider(LlmProviderType provider) async {
    await _service.setProvider(provider);
    state = state.copyWith(
      provider: provider,
      hasApiKey: _service.hasApiKey,
      model: _service.model,
    );
  }

  Future<void> setApiKey(LlmProviderType provider, String key) async {
    await _service.setApiKey(provider, key);
    state = state.copyWith(hasApiKey: _service.hasApiKey);
  }

  Future<void> clearApiKey(LlmProviderType provider) async {
    await _service.clearApiKey(provider);
    state = state.copyWith(hasApiKey: _service.hasApiKey);
  }

  Future<void> setModel(String model) async {
    await _service.setModel(model);
    state = state.copyWith(model: model);
  }

  Future<void> setOllamaUrl(String url) async {
    await _service.setOllamaUrl(url);
  }

  /// Fetch models from the active provider's API and update state.
  Future<void> fetchModels() async {
    state = state.copyWith(isFetchingModels: true);
    try {
      final models = await _service.fetchModelsForProvider(_service.provider);
      if (_service.provider == LlmProviderType.ollama) {
        state = state.copyWith(
          ollamaModels: models,
          availableModels: models,
          isFetchingModels: false,
        );
      } else {
        state = state.copyWith(
          availableModels: models,
          isFetchingModels: false,
        );
      }
    } catch (_) {
      state = state.copyWith(
        availableModels: [],
        isFetchingModels: false,
      );
    }
  }

  /// Pull an Ollama model by name. Returns a progress stream (0.0-1.0, -1 = error).
  /// Refreshes the model list on completion.
  Stream<double> pullOllamaModel(String name) async* {
    final client = _service.ollamaClient;
    await for (final progress in client.pullModel(name)) {
      yield progress;
    }
    // Refresh model list after pull completes
    await fetchModels();
  }

  /// Delete an Ollama model and refresh the list.
  Future<bool> deleteOllamaModel(String name) async {
    final client = _service.ollamaClient;
    final ok = await client.deleteModel(name);
    if (ok) await fetchModels();
    return ok;
  }

  /// Generate a tile.
  Future<LlmResponse> generateTile({
    required String systemPrompt,
    required String userPrompt,
  }) async {
    state = state.copyWith(isGenerating: true);
    try {
      final resp = await _service.generate(
        systemPrompt: systemPrompt,
        userPrompt: userPrompt,
      );
      state = state.copyWith(
        isGenerating: false,
        lastTokenUsage: resp.totalTokens,
      );
      return resp;
    } catch (e) {
      state = state.copyWith(isGenerating: false);
      return LlmResponse.error('$e');
    }
  }

  /// Chat with the AI expert.
  Future<LlmResponse> chat({
    required String systemPrompt,
    required List<LlmMessage> messages,
  }) async {
    state = state.copyWith(isGenerating: true);
    try {
      final resp = await _service.chat(
        systemPrompt: systemPrompt,
        messages: messages,
      );
      state = state.copyWith(
        isGenerating: false,
        lastTokenUsage: resp.totalTokens,
      );
      return resp;
    } catch (e) {
      state = state.copyWith(isGenerating: false);
      return LlmResponse.error('$e');
    }
  }
}

/// Backwards-compatible provider name — used everywhere in the app.
final claudeProvider = StateNotifierProvider<LlmNotifier, LlmState>(
  (ref) => LlmNotifier(),
);
