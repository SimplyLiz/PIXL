import 'dart:async';
import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:shared_preferences/shared_preferences.dart';

/// Anthropic Claude API client for tile generation and expert chat.
class ClaudeApi {
  ClaudeApi();

  static const _apiUrl = 'https://api.anthropic.com/v1/messages';
  static const _apiVersion = '2023-06-01';
  static const _secureKeyApiKey = 'anthropic_api_key';
  static const _prefKeyModel = 'claude_model';
  static const _timeout = Duration(seconds: 120);

  final _secureStorage = const FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    lOptions: LinuxOptions(),
    wOptions: WindowsOptions(),
    mOptions: MacOsOptions(useDataProtectionKeyChain: false),
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock),
  );

  String? _apiKey;
  String _model = 'claude-sonnet-4-20250514';

  String get model => _model;
  bool get hasApiKey => _apiKey != null && _apiKey!.isNotEmpty;

  /// Load API key from secure storage and model from SharedPreferences.
  Future<void> init() async {
    _apiKey = await _secureStorage.read(key: _secureKeyApiKey);
    final prefs = await SharedPreferences.getInstance();
    _model = prefs.getString(_prefKeyModel) ?? _model;
  }

  /// Save API key to OS keychain via flutter_secure_storage.
  Future<void> setApiKey(String key) async {
    _apiKey = key;
    await _secureStorage.write(key: _secureKeyApiKey, value: key);
  }

  /// Save model preference.
  Future<void> setModel(String model) async {
    _model = model;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_prefKeyModel, model);
  }

  /// Clear stored API key.
  Future<void> clearApiKey() async {
    _apiKey = null;
    await _secureStorage.delete(key: _secureKeyApiKey);
  }

  /// Send a message to Claude and return the full response.
  /// [systemPrompt] is the enriched context from the PIXL backend.
  /// [messages] is the conversation history.
  Future<ClaudeResponse> sendMessage({
    required String systemPrompt,
    required List<ClaudeMessage> messages,
    double temperature = 0.3,
    int maxTokens = 4096,
  }) async {
    if (!hasApiKey) {
      return ClaudeResponse.error('No API key configured. Open Settings to add your Anthropic API key.');
    }

    try {
      final body = jsonEncode({
        'model': _model,
        'max_tokens': maxTokens,
        'temperature': temperature,
        'system': systemPrompt,
        'messages': messages.map((m) => m.toJson()).toList(),
      });

      final resp = await http.post(
        Uri.parse(_apiUrl),
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': _apiKey!,
          'anthropic-version': _apiVersion,
        },
        body: body,
      ).timeout(_timeout, onTimeout: () {
        throw TimeoutException('Claude API request timed out after ${_timeout.inSeconds}s');
      });

      if (resp.statusCode != 200) {
        final errBody = jsonDecode(resp.body) as Map<String, dynamic>;
        final errMsg = (errBody['error'] as Map<String, dynamic>?)?['message'] ?? resp.body;
        return ClaudeResponse.error('API error (${resp.statusCode}): $errMsg');
      }

      final json = jsonDecode(resp.body) as Map<String, dynamic>;
      final content = json['content'] as List<dynamic>;
      final textBlocks = content
          .where((b) => (b as Map<String, dynamic>)['type'] == 'text')
          .map((b) => (b as Map<String, dynamic>)['text'] as String)
          .join('\n');

      final usage = json['usage'] as Map<String, dynamic>?;

      return ClaudeResponse(
        content: textBlocks,
        inputTokens: usage?['input_tokens'] as int? ?? 0,
        outputTokens: usage?['output_tokens'] as int? ?? 0,
        model: json['model'] as String? ?? _model,
      );
    } catch (e) {
      return ClaudeResponse.error('Request failed: $e');
    }
  }

  /// Generate a tile: sends enriched prompt to Claude, expects PAX grid back.
  Future<ClaudeResponse> generateTile({
    required String systemPrompt,
    required String userPrompt,
  }) async {
    return sendMessage(
      systemPrompt: systemPrompt,
      messages: [ClaudeMessage.user(userPrompt)],
      temperature: 0.3,
      maxTokens: 4096,
    );
  }

  /// Chat with the AI expert (higher temperature for conversational tone).
  Future<ClaudeResponse> chat({
    required String systemPrompt,
    required List<ClaudeMessage> messages,
  }) async {
    return sendMessage(
      systemPrompt: systemPrompt,
      messages: messages,
      temperature: 0.7,
      maxTokens: 2048,
    );
  }
}

/// A message in the Claude conversation.
class ClaudeMessage {
  const ClaudeMessage({required this.role, required this.content});

  factory ClaudeMessage.user(String content) =>
      ClaudeMessage(role: 'user', content: content);

  factory ClaudeMessage.assistant(String content) =>
      ClaudeMessage(role: 'assistant', content: content);

  final String role;
  final String content;

  Map<String, dynamic> toJson() => {'role': role, 'content': content};
}

/// Response from the Claude API.
class ClaudeResponse {
  const ClaudeResponse({
    required this.content,
    this.inputTokens = 0,
    this.outputTokens = 0,
    this.model = '',
    this.errorMessage,
  });

  factory ClaudeResponse.error(String message) =>
      ClaudeResponse(content: '', errorMessage: message);

  final String content;
  final int inputTokens;
  final int outputTokens;
  final String model;
  final String? errorMessage;

  bool get isError => errorMessage != null;
  int get totalTokens => inputTokens + outputTokens;
}

/// Available Claude models for the Studio.
class ClaudeModels {
  static const models = [
    ('claude-sonnet-4-20250514', 'Sonnet 4 (fast, recommended)'),
    ('claude-haiku-4-5-20251001', 'Haiku 4.5 (fastest, cheapest)'),
    ('claude-opus-4-6-20250527', 'Opus 4.6 (most capable)'),
  ];
}
