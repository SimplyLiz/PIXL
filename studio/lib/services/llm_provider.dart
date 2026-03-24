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
  ollama('Ollama (Local)'),
  pixlLocal('PIXL LoRA (On-Device)');

  const LlmProviderType(this.displayName);
  final String displayName;
}

// ── Model definitions ──────────────────────────────────────

enum ModelCost { free, cheap, medium, high }

class LlmModel {
  const LlmModel(this.id, this.label, {
    this.contextLength = 0,
    this.vision = false,
    this.thinking = false,
    this.cost = ModelCost.medium,
    this.local = false,
    this.parameterSize = '',
  });
  final String id;
  final String label;
  final int contextLength; // max input tokens, 0 = unknown
  final bool vision;
  final bool thinking;
  final ModelCost cost;
  final bool local;
  final String parameterSize; // e.g. "7B", "70B" (Ollama)

  String get contextLabel {
    if (contextLength <= 0) return '';
    if (contextLength >= 1000000) return '${contextLength ~/ 1000000}M';
    return '${contextLength ~/ 1000}k';
  }

  String get costLabel => switch (cost) {
    ModelCost.free => 'Free',
    ModelCost.cheap => '\$',
    ModelCost.medium => '\$\$',
    ModelCost.high => '\$\$\$',
  };
}

const anthropicFallbackModels = [
  LlmModel('claude-sonnet-4-20250514', 'Sonnet 4', contextLength: 200000, vision: true, thinking: true, cost: ModelCost.medium),
  LlmModel('claude-haiku-4-5-20251001', 'Haiku 4.5', contextLength: 200000, vision: true, cost: ModelCost.cheap),
  LlmModel('claude-opus-4-6-20250527', 'Opus 4.6', contextLength: 200000, vision: true, thinking: true, cost: ModelCost.high),
];

const openaiFallbackModels = [
  LlmModel('gpt-4o', 'GPT-4o', contextLength: 128000, vision: true, cost: ModelCost.high),
  LlmModel('gpt-4o-mini', 'GPT-4o Mini', contextLength: 128000, vision: true, cost: ModelCost.cheap),
  LlmModel('o3-mini', 'o3-mini', contextLength: 200000, thinking: true, cost: ModelCost.medium),
];

const geminiFallbackModels = [
  LlmModel('gemini-2.5-flash', 'Gemini 2.5 Flash', contextLength: 1000000, vision: true, thinking: true, cost: ModelCost.cheap),
  LlmModel('gemini-2.5-pro', 'Gemini 2.5 Pro', contextLength: 1000000, vision: true, thinking: true, cost: ModelCost.medium),
  LlmModel('gemini-2.0-flash', 'Gemini 2.0 Flash', contextLength: 1000000, vision: true, cost: ModelCost.cheap),
];

/// Curated suggestions for Ollama models users might want to pull.
const ollamaSuggestions = [
  LlmModel('llama3.2', 'Llama 3.2 3B', cost: ModelCost.free, local: true, parameterSize: '3B'),
  LlmModel('llama3.1:8b', 'Llama 3.1 8B', cost: ModelCost.free, local: true, parameterSize: '8B'),
  LlmModel('qwen3:8b', 'Qwen 3 8B', thinking: true, cost: ModelCost.free, local: true, parameterSize: '8B'),
  LlmModel('gemma3:4b', 'Gemma 3 4B', vision: true, cost: ModelCost.free, local: true, parameterSize: '4B'),
  LlmModel('mistral', 'Mistral 7B', cost: ModelCost.free, local: true, parameterSize: '7B'),
  LlmModel('deepseek-coder:6.7b', 'DeepSeek Coder 6.7B', cost: ModelCost.free, local: true, parameterSize: '6.7B'),
  LlmModel('phi3', 'Phi-3 Mini', cost: ModelCost.free, local: true, parameterSize: '3.8B'),
  LlmModel('codellama:7b', 'Code Llama 7B', cost: ModelCost.free, local: true, parameterSize: '7B'),
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
    return LlmModel(name, label,
      cost: ModelCost.free,
      local: true,
      parameterSize: parameterSize,
    );
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
      Uri.parse('https://api.anthropic.com/v1/models?limit=100'),
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
      if (id.contains('batch') || id.contains('deprecated')) continue;
      final caps = item['capabilities'] as Map<String, dynamic>? ?? {};
      final thinkingCap = caps['thinking'] as Map<String, dynamic>?;
      final imageCap = caps['image_input'] as Map<String, dynamic>?;
      final maxInput = item['max_input_tokens'] as int? ?? 0;
      final idLower = id.toLowerCase();
      models.add(LlmModel(id, displayName,
        contextLength: maxInput,
        vision: imageCap?['supported'] == true,
        thinking: thinkingCap?['supported'] == true,
        cost: idLower.contains('opus') ? ModelCost.high
            : idLower.contains('haiku') ? ModelCost.cheap
            : ModelCost.medium,
      ));
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
    final chatPrefixes = ['gpt-4', 'gpt-3.5', 'o1', 'o3', 'o4', 'chatgpt'];
    final skipPrefixes = ['whisper', 'dall-e', 'tts', 'embedding', 'text-embedding', 'davinci', 'babbage'];
    final models = <LlmModel>[];
    for (final item in data) {
      final id = item['id'] as String? ?? '';
      if (id.isEmpty) continue;
      final lower = id.toLowerCase();
      if (skipPrefixes.any((s) => lower.contains(s))) continue;
      if (!chatPrefixes.any((p) => lower.startsWith(p))) continue;
      final isReasoning = lower.startsWith('o1') || lower.startsWith('o3') || lower.startsWith('o4');
      final isMini = lower.contains('mini');
      final hasVision = lower.contains('gpt-4o') || lower.contains('gpt-4-turbo') || lower.contains('chatgpt');
      final ctxLen = isReasoning ? 200000
          : lower.contains('gpt-3.5') ? 16385
          : 128000;
      models.add(LlmModel(id, id,
        contextLength: ctxLen,
        vision: hasVision,
        thinking: isReasoning,
        cost: isMini ? ModelCost.cheap
            : isReasoning ? ModelCost.high
            : lower.contains('gpt-4o') ? ModelCost.high
            : lower.contains('gpt-3.5') ? ModelCost.cheap
            : ModelCost.medium,
      ));
    }
    // Sort: reasoning first, then gpt-4o, then rest
    int priority(String id) {
      if (id.startsWith('o4')) return 0;
      if (id.startsWith('o3')) return 1;
      if (id.startsWith('o1')) return 2;
      if (id.startsWith('gpt-4o') && !id.contains('mini')) return 3;
      if (id.startsWith('chatgpt')) return 4;
      if (id.contains('mini')) return 5;
      if (id.startsWith('gpt-4')) return 6;
      return 7;
    }
    models.sort((a, b) {
      final pa = priority(a.id);
      final pb = priority(b.id);
      if (pa != pb) return pa.compareTo(pb);
      return b.id.compareTo(a.id);
    });
    return models;
  }

  Future<List<LlmModel>> _fetchGeminiModels() async {
    final key = _apiKeys[LlmProviderType.gemini];
    if (key == null || key.isEmpty) return [];
    final resp = await http.get(
      Uri.parse('https://generativelanguage.googleapis.com/v1beta/models?key=$key&pageSize=100'),
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
      final inputLimit = item['inputTokenLimit'] as int? ?? 0;
      final hasThinking = item['thinking'] == true;
      final nameLower = name.toLowerCase();
      models.add(LlmModel(name, displayName,
        contextLength: inputLimit,
        vision: true, // all Gemini generateContent models support vision
        thinking: hasThinking || nameLower.contains('2.5'),
        cost: nameLower.contains('pro') ? ModelCost.medium : ModelCost.cheap,
      ));
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
