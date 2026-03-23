import 'dart:async';
import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:shared_preferences/shared_preferences.dart';

// ── Response types ─────────────────────────────────────────

class LlmResponse {
  const LlmResponse({
    required this.content,
    this.inputTokens = 0,
    this.outputTokens = 0,
    this.model = '',
    this.errorMessage,
  });

  factory LlmResponse.error(String message) =>
      LlmResponse(content: '', errorMessage: message);

  final String content;
  final int inputTokens;
  final int outputTokens;
  final String model;
  final String? errorMessage;

  bool get isError => errorMessage != null;
  int get totalTokens => inputTokens + outputTokens;
}

class LlmMessage {
  const LlmMessage({required this.role, required this.content});
  factory LlmMessage.user(String content) => LlmMessage(role: 'user', content: content);
  factory LlmMessage.assistant(String content) => LlmMessage(role: 'assistant', content: content);

  final String role;
  final String content;
}

// ── Provider enum ──────────────────────────────────────────

enum LlmProviderType {
  anthropic('Anthropic (Claude)'),
  openai('OpenAI (GPT)'),
  gemini('Google (Gemini)'),
  ollama('Ollama (Local)');

  const LlmProviderType(this.displayName);
  final String displayName;
}

// ── Model definitions ──────────────────────────────────────

class LlmModel {
  const LlmModel(this.id, this.label);
  final String id;
  final String label;
}

const anthropicModels = [
  LlmModel('claude-sonnet-4-20250514', 'Sonnet 4 (recommended)'),
  LlmModel('claude-haiku-4-5-20251001', 'Haiku 4.5 (fast)'),
  LlmModel('claude-opus-4-6-20250527', 'Opus 4.6 (most capable)'),
];

const openaiModels = [
  LlmModel('gpt-4o', 'GPT-4o (recommended)'),
  LlmModel('gpt-4o-mini', 'GPT-4o Mini (fast)'),
  LlmModel('o3-mini', 'o3-mini (reasoning)'),
];

const geminiModels = [
  LlmModel('gemini-2.5-flash', 'Gemini 2.5 Flash (recommended)'),
  LlmModel('gemini-2.5-pro', 'Gemini 2.5 Pro'),
  LlmModel('gemini-2.0-flash', 'Gemini 2.0 Flash (fast)'),
];

const ollamaModels = [
  LlmModel('llama3.3', 'Llama 3.3 70B'),
  LlmModel('qwen2.5-coder:32b', 'Qwen 2.5 Coder 32B'),
  LlmModel('gemma2:27b', 'Gemma 2 27B'),
  LlmModel('mistral', 'Mistral 7B (fast)'),
];

List<LlmModel> modelsForProvider(LlmProviderType type) => switch (type) {
  LlmProviderType.anthropic => anthropicModels,
  LlmProviderType.openai => openaiModels,
  LlmProviderType.gemini => geminiModels,
  LlmProviderType.ollama => ollamaModels,
};

// ── Abstract provider ──────────────────────────────────────

abstract class LlmBackend {
  Future<LlmResponse> sendMessage({
    required String systemPrompt,
    required List<LlmMessage> messages,
    required String model,
    double temperature,
    int maxTokens,
  });
}

// ── Anthropic ──────────────────────────────────────────────

class AnthropicBackend implements LlmBackend {
  AnthropicBackend(this.apiKey);
  final String apiKey;

  @override
  Future<LlmResponse> sendMessage({
    required String systemPrompt,
    required List<LlmMessage> messages,
    required String model,
    double temperature = 0.3,
    int maxTokens = 4096,
  }) async {
    try {
      final resp = await http.post(
        Uri.parse('https://api.anthropic.com/v1/messages'),
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': apiKey,
          'anthropic-version': '2023-06-01',
        },
        body: jsonEncode({
          'model': model,
          'max_tokens': maxTokens,
          'temperature': temperature,
          'system': systemPrompt,
          'messages': messages.map((m) => {'role': m.role, 'content': m.content}).toList(),
        }),
      ).timeout(const Duration(seconds: 120));

      if (resp.statusCode != 200) {
        final err = jsonDecode(resp.body);
        return LlmResponse.error('Anthropic ${resp.statusCode}: ${(err['error'] as Map?)?['message'] ?? resp.body}');
      }

      final json = jsonDecode(resp.body) as Map<String, dynamic>;
      final content = (json['content'] as List)
          .where((b) => b['type'] == 'text')
          .map((b) => b['text'] as String)
          .join('\n');
      final usage = json['usage'] as Map<String, dynamic>?;

      return LlmResponse(
        content: content,
        inputTokens: usage?['input_tokens'] as int? ?? 0,
        outputTokens: usage?['output_tokens'] as int? ?? 0,
        model: json['model'] as String? ?? model,
      );
    } catch (e) {
      return LlmResponse.error('Anthropic request failed: $e');
    }
  }
}

// ── OpenAI ─────────────────────────────────────────────────

class OpenAiBackend implements LlmBackend {
  OpenAiBackend(this.apiKey);
  final String apiKey;

  @override
  Future<LlmResponse> sendMessage({
    required String systemPrompt,
    required List<LlmMessage> messages,
    required String model,
    double temperature = 0.3,
    int maxTokens = 4096,
  }) async {
    try {
      final allMessages = [
        {'role': 'system', 'content': systemPrompt},
        ...messages.map((m) => {'role': m.role, 'content': m.content}),
      ];

      final resp = await http.post(
        Uri.parse('https://api.openai.com/v1/chat/completions'),
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer $apiKey',
        },
        body: jsonEncode({
          'model': model,
          'max_completion_tokens': maxTokens,
          'temperature': temperature,
          'messages': allMessages,
        }),
      ).timeout(const Duration(seconds: 120));

      if (resp.statusCode != 200) {
        final err = jsonDecode(resp.body);
        return LlmResponse.error('OpenAI ${resp.statusCode}: ${(err['error'] as Map?)?['message'] ?? resp.body}');
      }

      final json = jsonDecode(resp.body) as Map<String, dynamic>;
      final choice = (json['choices'] as List).first as Map<String, dynamic>;
      final content = (choice['message'] as Map<String, dynamic>)['content'] as String? ?? '';
      final usage = json['usage'] as Map<String, dynamic>?;

      return LlmResponse(
        content: content,
        inputTokens: usage?['prompt_tokens'] as int? ?? 0,
        outputTokens: usage?['completion_tokens'] as int? ?? 0,
        model: json['model'] as String? ?? model,
      );
    } catch (e) {
      return LlmResponse.error('OpenAI request failed: $e');
    }
  }
}

// ── Gemini ─────────────────────────────────────────────────

class GeminiBackend implements LlmBackend {
  GeminiBackend(this.apiKey);
  final String apiKey;

  @override
  Future<LlmResponse> sendMessage({
    required String systemPrompt,
    required List<LlmMessage> messages,
    required String model,
    double temperature = 0.3,
    int maxTokens = 4096,
  }) async {
    try {
      final contents = messages.map((m) => {
        'role': m.role == 'assistant' ? 'model' : 'user',
        'parts': [{'text': m.content}],
      }).toList();

      final resp = await http.post(
        Uri.parse('https://generativelanguage.googleapis.com/v1beta/models/$model:generateContent?key=$apiKey'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({
          'system_instruction': {'parts': [{'text': systemPrompt}]},
          'contents': contents,
          'generationConfig': {
            'temperature': temperature,
            'maxOutputTokens': maxTokens,
          },
        }),
      ).timeout(const Duration(seconds: 120));

      if (resp.statusCode != 200) {
        return LlmResponse.error('Gemini ${resp.statusCode}: ${resp.body}');
      }

      final json = jsonDecode(resp.body) as Map<String, dynamic>;
      final candidates = json['candidates'] as List?;
      if (candidates == null || candidates.isEmpty) {
        return LlmResponse.error('Gemini returned no candidates');
      }

      final parts = ((candidates.first as Map)['content'] as Map)['parts'] as List;
      final content = parts.map((p) => (p as Map)['text'] as String).join('\n');

      final usage = json['usageMetadata'] as Map<String, dynamic>?;

      return LlmResponse(
        content: content,
        inputTokens: usage?['promptTokenCount'] as int? ?? 0,
        outputTokens: usage?['candidatesTokenCount'] as int? ?? 0,
        model: model,
      );
    } catch (e) {
      return LlmResponse.error('Gemini request failed: $e');
    }
  }
}

// ── Ollama (local) ─────────────────────────────────────────

class OllamaBackend implements LlmBackend {
  OllamaBackend({this.baseUrl = 'http://localhost:11434'});
  final String baseUrl;

  @override
  Future<LlmResponse> sendMessage({
    required String systemPrompt,
    required List<LlmMessage> messages,
    required String model,
    double temperature = 0.3,
    int maxTokens = 4096,
  }) async {
    try {
      final allMessages = [
        {'role': 'system', 'content': systemPrompt},
        ...messages.map((m) => {'role': m.role, 'content': m.content}),
      ];

      final resp = await http.post(
        Uri.parse('$baseUrl/api/chat'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({
          'model': model,
          'messages': allMessages,
          'stream': false,
          'options': {
            'temperature': temperature,
            'num_predict': maxTokens,
          },
        }),
      ).timeout(const Duration(seconds: 300)); // Ollama can be slow

      if (resp.statusCode != 200) {
        return LlmResponse.error('Ollama ${resp.statusCode}: ${resp.body}');
      }

      final json = jsonDecode(resp.body) as Map<String, dynamic>;
      final message = json['message'] as Map<String, dynamic>?;
      final content = message?['content'] as String? ?? '';

      return LlmResponse(
        content: content,
        inputTokens: json['prompt_eval_count'] as int? ?? 0,
        outputTokens: json['eval_count'] as int? ?? 0,
        model: model,
      );
    } catch (e) {
      return LlmResponse.error('Ollama request failed: $e');
    }
  }
}

// ── Unified LLM Service ────────────────────────────────────

class LlmService {
  LlmService();

  static const _secureStorage = FlutterSecureStorage();
  static const _prefProvider = 'llm_provider';
  static const _prefModel = 'llm_model';
  static const _prefOllamaUrl = 'ollama_url';

  LlmProviderType _provider = LlmProviderType.anthropic;
  String _model = 'claude-sonnet-4-20250514';
  String _ollamaUrl = 'http://localhost:11434';
  final Map<LlmProviderType, String> _apiKeys = {};

  LlmProviderType get provider => _provider;
  String get model => _model;
  String get ollamaUrl => _ollamaUrl;
  bool get hasApiKey =>
      _provider == LlmProviderType.ollama ||
      (_apiKeys[_provider]?.isNotEmpty ?? false);

  String _secureKey(LlmProviderType p) => 'llm_api_key_${p.name}';

  Future<void> init() async {
    final prefs = await SharedPreferences.getInstance();
    final providerName = prefs.getString(_prefProvider);
    if (providerName != null) {
      _provider = LlmProviderType.values.firstWhere(
        (p) => p.name == providerName,
        orElse: () => LlmProviderType.anthropic,
      );
    }
    _model = prefs.getString(_prefModel) ?? _model;
    _ollamaUrl = prefs.getString(_prefOllamaUrl) ?? _ollamaUrl;

    // Load all API keys
    for (final p in LlmProviderType.values) {
      final key = await _secureStorage.read(key: _secureKey(p));
      if (key != null && key.isNotEmpty) {
        _apiKeys[p] = key;
      }
    }
  }

  Future<void> setProvider(LlmProviderType provider) async {
    _provider = provider;
    // Auto-select first model for the new provider if current model doesn't match
    final models = modelsForProvider(provider);
    if (!models.any((m) => m.id == _model)) {
      _model = models.first.id;
    }
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_prefProvider, provider.name);
    await prefs.setString(_prefModel, _model);
  }

  Future<void> setModel(String model) async {
    _model = model;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_prefModel, model);
  }

  Future<void> setApiKey(LlmProviderType provider, String key) async {
    _apiKeys[provider] = key;
    await _secureStorage.write(key: _secureKey(provider), value: key);
  }

  Future<void> clearApiKey(LlmProviderType provider) async {
    _apiKeys.remove(provider);
    await _secureStorage.delete(key: _secureKey(provider));
  }

  String? getApiKey(LlmProviderType provider) => _apiKeys[provider];

  bool hasKeyFor(LlmProviderType provider) =>
      provider == LlmProviderType.ollama ||
      (_apiKeys[provider]?.isNotEmpty ?? false);

  Future<void> setOllamaUrl(String url) async {
    _ollamaUrl = url;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_prefOllamaUrl, url);
  }

  LlmBackend _createBackend() {
    return switch (_provider) {
      LlmProviderType.anthropic => AnthropicBackend(_apiKeys[_provider] ?? ''),
      LlmProviderType.openai => OpenAiBackend(_apiKeys[_provider] ?? ''),
      LlmProviderType.gemini => GeminiBackend(_apiKeys[_provider] ?? ''),
      LlmProviderType.ollama => OllamaBackend(baseUrl: _ollamaUrl),
    };
  }

  Future<LlmResponse> generate({
    required String systemPrompt,
    required String userPrompt,
    double temperature = 0.3,
    int maxTokens = 4096,
  }) async {
    if (!hasApiKey) {
      return LlmResponse.error('No API key for ${_provider.displayName}. Open Settings to configure.');
    }
    return _createBackend().sendMessage(
      systemPrompt: systemPrompt,
      messages: [LlmMessage.user(userPrompt)],
      model: _model,
      temperature: temperature,
      maxTokens: maxTokens,
    );
  }

  Future<LlmResponse> chat({
    required String systemPrompt,
    required List<LlmMessage> messages,
    double temperature = 0.7,
    int maxTokens = 2048,
  }) async {
    if (!hasApiKey) {
      return LlmResponse.error('No API key for ${_provider.displayName}. Open Settings to configure.');
    }
    return _createBackend().sendMessage(
      systemPrompt: systemPrompt,
      messages: messages,
      model: _model,
      temperature: temperature,
      maxTokens: maxTokens,
    );
  }
}
