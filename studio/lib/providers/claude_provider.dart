import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/claude_api.dart';

/// State for the Claude API connection.
class ClaudeState {
  const ClaudeState({
    this.hasApiKey = false,
    this.model = 'claude-sonnet-4-20250514',
    this.isGenerating = false,
    this.lastTokenUsage = 0,
  });

  final bool hasApiKey;
  final String model;
  final bool isGenerating;
  final int lastTokenUsage;

  ClaudeState copyWith({
    bool? hasApiKey,
    String? model,
    bool? isGenerating,
    int? lastTokenUsage,
  }) {
    return ClaudeState(
      hasApiKey: hasApiKey ?? this.hasApiKey,
      model: model ?? this.model,
      isGenerating: isGenerating ?? this.isGenerating,
      lastTokenUsage: lastTokenUsage ?? this.lastTokenUsage,
    );
  }
}

class ClaudeNotifier extends StateNotifier<ClaudeState> {
  ClaudeNotifier() : super(const ClaudeState());

  final ClaudeApi _api = ClaudeApi();
  ClaudeApi get api => _api;

  Future<void> init() async {
    await _api.init();
    state = state.copyWith(
      hasApiKey: _api.hasApiKey,
      model: _api.model,
    );
  }

  Future<void> setApiKey(String key) async {
    await _api.setApiKey(key);
    state = state.copyWith(hasApiKey: _api.hasApiKey);
  }

  Future<void> clearApiKey() async {
    await _api.clearApiKey();
    state = state.copyWith(hasApiKey: false);
  }

  Future<void> setModel(String model) async {
    await _api.setModel(model);
    state = state.copyWith(model: model);
  }

  /// Generate a tile via Claude.
  Future<ClaudeResponse> generateTile({
    required String systemPrompt,
    required String userPrompt,
  }) async {
    state = state.copyWith(isGenerating: true);
    try {
      final resp = await _api.generateTile(
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
      return ClaudeResponse.error('$e');
    }
  }

  /// Chat with the AI expert.
  Future<ClaudeResponse> chat({
    required String systemPrompt,
    required List<ClaudeMessage> messages,
  }) async {
    state = state.copyWith(isGenerating: true);
    try {
      final resp = await _api.chat(
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
      return ClaudeResponse.error('$e');
    }
  }
}

final claudeProvider = StateNotifierProvider<ClaudeNotifier, ClaudeState>(
  (ref) => ClaudeNotifier(),
);
