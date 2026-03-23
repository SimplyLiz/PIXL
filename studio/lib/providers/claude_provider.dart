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
  });

  final LlmProviderType provider;
  final bool hasApiKey;
  final String model;
  final bool isGenerating;
  final int lastTokenUsage;

  LlmState copyWith({
    LlmProviderType? provider,
    bool? hasApiKey,
    String? model,
    bool? isGenerating,
    int? lastTokenUsage,
  }) {
    return LlmState(
      provider: provider ?? this.provider,
      hasApiKey: hasApiKey ?? this.hasApiKey,
      model: model ?? this.model,
      isGenerating: isGenerating ?? this.isGenerating,
      lastTokenUsage: lastTokenUsage ?? this.lastTokenUsage,
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
