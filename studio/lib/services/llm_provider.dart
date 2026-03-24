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

const anthropicFallbackModels = [
  LlmModel('claude-sonnet-4-20250514', 'Sonnet 4 (recommended)'),
  LlmModel('claude-haiku-4-5-20251001', 'Haiku 4.5 (fast)'),
  LlmModel('claude-opus-4-6-20250527', 'Opus 4.6 (most capable)'),
];

const openaiFallbackModels = [
  LlmModel('gpt-4o', 'GPT-4o (recommended)'),
  LlmModel('gpt-4o-mini', 'GPT-4o Mini (fast)'),
  LlmModel('o3-mini', 'o3-mini (reasoning)'),
];

const geminiFallbackModels = [
  LlmModel('gemini-2.5-flash', 'Gemini 2.5 Flash (recommended)'),
  LlmModel('gemini-2.5-pro', 'Gemini 2.5 Pro'),
  LlmModel('gemini-2.0-flash', 'Gemini 2.0 Flash (fast)'),
];

/// Curated suggestions for Ollama models users might want to pull.
const ollamaSuggestions = [
  LlmModel('llama3.2', 'Llama 3.2 3B'),
  LlmModel('llama3.1:8b', 'Llama 3.1 8B'),
  LlmModel('qwen3:8b', 'Qwen 3 8B'),
  LlmModel('gemma3:4b', 'Gemma 3 4B'),
  LlmModel('mistral', 'Mistral 7B'),
  LlmModel('deepseek-coder:6.7b', 'DeepSeek Coder 6.7B'),
  LlmModel('phi3', 'Phi-3 Mini'),
  LlmModel('codellama:7b', 'Code Llama 7B'),
];

/// Fallback if Ollama is unreachable — empty list means "connect to see models".
const ollamaFallbackModels = <LlmModel>[];

List<LlmModel> modelsForProvider(LlmProviderType type) => switch (type) {
  LlmProviderType.anthropic => anthropicFallbackModels,
  LlmProviderType.openai => openaiFallbackModels,
  LlmProviderType.gemini => geminiFallbackModels,
  LlmProviderType.ollama => ollamaFallbackModels,
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

// ── Ollama model management ────────────────────────────────

class OllamaModelInfo {
  const OllamaModelInfo({required this.name, required this.size, this.parameterSize = ''});
  final String name;
  final int size; // bytes
  final String parameterSize;

  String get sizeLabel {
    if (size > 1e9) return '${(size / 1e9).toStringAsFixed(1)} GB';
    if (size > 1e6) return '${(size / 1e6).toStringAsFixed(0)} MB';
    return '$size B';
  }

  LlmModel toLlmModel() {
    final label = parameterSize.isNotEmpty ? '$name ($parameterSize)' : name;
    return LlmModel(name, label);
  }
}

class OllamaClient {
  OllamaClient({this.baseUrl = 'http://localhost:11434'});
  final String baseUrl;

  /// Fetch locally installed models from Ollama's /api/tags endpoint.
  Future<List<OllamaModelInfo>> fetchModels() async {
    try {
      final resp = await http.get(
        Uri.parse('$baseUrl/api/tags'),
        headers: {'Content-Type': 'application/json'},
      ).timeout(const Duration(seconds: 5));

      if (resp.statusCode != 200) return [];

      final json = jsonDecode(resp.body) as Map<String, dynamic>;
      final models = json['models'] as List? ?? [];
      return models.map((m) {
        final details = m['details'] as Map<String, dynamic>? ?? {};
        return OllamaModelInfo(
          name: m['name'] as String? ?? '',
          size: m['size'] as int? ?? 0,
          parameterSize: details['parameter_size'] as String? ?? '',
        );
      }).where((m) => m.name.isNotEmpty).toList();
    } catch (_) {
      return [];
    }
  }

  /// Pull a model from the Ollama library with progress reporting.
  /// Returns a stream of progress values (0.0 to 1.0).
  Stream<double> pullModel(String name) async* {
    final client = http.Client();
    try {
      final request = http.Request('POST', Uri.parse('$baseUrl/api/pull'));
      request.headers['Content-Type'] = 'application/json';
      request.body = jsonEncode({'name': name, 'stream': true});

      final response = await client.send(request)
          .timeout(const Duration(minutes: 30));

      if (response.statusCode != 200) {
        yield -1; // signal error
        return;
      }

      final stream = response.stream.transform(utf8.decoder);
      String buffer = '';

      await for (final chunk in stream) {
        buffer += chunk;
        // Ollama sends newline-delimited JSON
        while (buffer.contains('\n')) {
          final idx = buffer.indexOf('\n');
          final line = buffer.substring(0, idx).trim();
          buffer = buffer.substring(idx + 1);

          if (line.isEmpty) continue;
          try {
            final json = jsonDecode(line) as Map<String, dynamic>;
            final total = json['total'] as int? ?? 0;
            final completed = json['completed'] as int? ?? 0;
            if (total > 0) {
              yield completed / total;
            }
          } catch (_) {}
        }
      }
      yield 1.0; // done
    } catch (_) {
      yield -1; // error
    } finally {
      client.close();
    }
  }

  /// Delete a model.
  Future<bool> deleteModel(String name) async {
    try {
      final resp = await http.delete(
        Uri.parse('$baseUrl/api/delete'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'name': name}),
      ).timeout(const Duration(seconds: 10));
      return resp.statusCode == 200;
    } catch (_) {
      return false;
    }
  }
}

// ── Unified LLM Service ────────────────────────────────────

class LlmService {
  LlmService();

  static const _secureStorage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    lOptions: LinuxOptions(),
    wOptions: WindowsOptions(),
    mOptions: MacOsOptions(),
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock),
  );
  static const _prefProvider = 'llm_provider';
  static const _prefModel = 'llm_model';
  static const _prefOllamaUrl = 'ollama_url';

  LlmProviderType _provider = LlmProviderType.anthropic;
  String _model = 'claude-sonnet-4-20250514';
  String _ollamaUrl = 'http://localhost:11434';
  final Map<LlmProviderType, String> _apiKeys = {};
  final Map<LlmProviderType, List<LlmModel>> _fetchedModels = {};

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

  /// Dynamically discovered Ollama models (populated by fetchOllamaModels).
  List<LlmModel> _ollamaInstalledModels = [];
  List<LlmModel> get ollamaInstalledModels => _ollamaInstalledModels;

  OllamaClient get ollamaClient => OllamaClient(baseUrl: _ollamaUrl);

  /// Fetch installed Ollama models from the running server.
  Future<List<LlmModel>> fetchOllamaModels() async {
    final client = OllamaClient(baseUrl: _ollamaUrl);
    final models = await client.fetchModels();
    _ollamaInstalledModels = models.map((m) => m.toLlmModel()).toList();
    return _ollamaInstalledModels;
  }

  /// Cached fetched models for a provider (empty if not yet fetched).
  List<LlmModel> fetchedModelsFor(LlmProviderType provider) =>
      _fetchedModels[provider] ?? [];

  /// Fetch available models from a cloud provider's API.
  /// Returns the fetched list, or empty on failure (caller falls back to hardcoded).
  Future<List<LlmModel>> fetchModelsForProvider(LlmProviderType provider) async {
    try {
      final models = await switch (provider) {
        LlmProviderType.anthropic => _fetchAnthropicModels(),
        LlmProviderType.openai => _fetchOpenAiModels(),
        LlmProviderType.gemini => _fetchGeminiModels(),
        LlmProviderType.ollama => fetchOllamaModels(),
      };
      if (models.isNotEmpty) {
        _fetchedModels[provider] = models;
      }
      return models;
    } catch (_) {
      return [];
    }
  }

  Future<List<LlmModel>> _fetchAnthropicModels() async {
    final key = _apiKeys[LlmProviderType.anthropic];
    if (key == null || key.isEmpty) return [];
    final resp = await http.get(
      Uri.parse('https://api.anthropic.com/v1/models'),
      headers: {
        'x-api-key': key,
        'anthropic-version': '2023-06-01',
      },
    ).timeout(const Duration(seconds: 5));
    if (resp.statusCode != 200) return [];
    final json = jsonDecode(resp.body) as Map<String, dynamic>;
    final data = json['data'] as List? ?? [];
    final models = <LlmModel>[];
    for (final item in data) {
      final id = item['id'] as String? ?? '';
      final displayName = item['display_name'] as String? ?? id;
      if (id.isEmpty) continue;
      // Skip batch-only, deprecated, or non-chat models
      if (id.contains('batch') || id.contains('deprecated')) continue;
      models.add(LlmModel(id, displayName));
    }
    return models;
  }

  Future<List<LlmModel>> _fetchOpenAiModels() async {
    final key = _apiKeys[LlmProviderType.openai];
    if (key == null || key.isEmpty) return [];
    final resp = await http.get(
      Uri.parse('https://api.openai.com/v1/models'),
      headers: {'Authorization': 'Bearer $key'},
    ).timeout(const Duration(seconds: 5));
    if (resp.statusCode != 200) return [];
    final json = jsonDecode(resp.body) as Map<String, dynamic>;
    final data = json['data'] as List? ?? [];
    final chatPrefixes = ['gpt-4', 'gpt-3.5', 'o1', 'o3', 'o4'];
    final skipPrefixes = ['whisper', 'dall-e', 'tts', 'embedding', 'text-embedding'];
    final models = <LlmModel>[];
    for (final item in data) {
      final id = item['id'] as String? ?? '';
      if (id.isEmpty) continue;
      final lower = id.toLowerCase();
      if (skipPrefixes.any((s) => lower.contains(s))) continue;
      if (!chatPrefixes.any((p) => lower.startsWith(p))) continue;
      models.add(LlmModel(id, id));
    }
    // Sort: newest/best first — rough heuristic by name
    models.sort((a, b) => b.id.compareTo(a.id));
    return models;
  }

  Future<List<LlmModel>> _fetchGeminiModels() async {
    final key = _apiKeys[LlmProviderType.gemini];
    if (key == null || key.isEmpty) return [];
    final resp = await http.get(
      Uri.parse('https://generativelanguage.googleapis.com/v1beta/models?key=$key'),
    ).timeout(const Duration(seconds: 5));
    if (resp.statusCode != 200) return [];
    final json = jsonDecode(resp.body) as Map<String, dynamic>;
    final data = json['models'] as List? ?? [];
    final models = <LlmModel>[];
    for (final item in data) {
      final name = (item['name'] as String? ?? '').replaceFirst('models/', '');
      final displayName = item['displayName'] as String? ?? name;
      final methods = (item['supportedGenerationMethods'] as List?)
              ?.cast<String>() ??
          [];
      if (name.isEmpty) continue;
      if (!methods.contains('generateContent')) continue;
      models.add(LlmModel(name, displayName));
    }
    return models;
  }

  Future<void> setProvider(LlmProviderType provider) async {
    _provider = provider;
    // Auto-select first model for the new provider if current model doesn't match
    final fetched = _fetchedModels[provider] ?? [];
    final models = fetched.isNotEmpty
        ? fetched
        : provider == LlmProviderType.ollama && _ollamaInstalledModels.isNotEmpty
            ? _ollamaInstalledModels
            : modelsForProvider(provider);
    if (models.isNotEmpty && !models.any((m) => m.id == _model)) {
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
