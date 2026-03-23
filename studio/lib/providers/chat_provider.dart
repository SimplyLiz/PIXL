import 'package:flutter_riverpod/flutter_riverpod.dart';

/// A chat message in the AI expert panel.
class ChatMessage {
  const ChatMessage({
    required this.role,
    required this.content,
    this.timestamp,
    this.isStatus = false,
  });

  final String role; // 'user' or 'assistant'
  final String content;
  final DateTime? timestamp;
  /// Status messages (e.g. "Generating...") are shown in UI but excluded
  /// from the context window sent to Claude.
  final bool isStatus;
}

class ChatNotifier extends StateNotifier<List<ChatMessage>> {
  ChatNotifier()
      : super([
          const ChatMessage(
            role: 'assistant',
            content:
                'Welcome to PIXL Studio! I\'m your pixel art expert. '
                'Ask me about techniques, request tile generation, '
                'or get help with your palette and style.\n\n'
                'Try: *"Generate a 16×16 dungeon wall tile"*',
          ),
        ]);

  void addUserMessage(String content) {
    state = [
      ...state,
      ChatMessage(
        role: 'user',
        content: content,
        timestamp: DateTime.now(),
      ),
    ];
  }

  void addAssistantMessage(String content, {bool isStatus = false}) {
    state = [
      ...state,
      ChatMessage(
        role: 'assistant',
        content: content,
        timestamp: DateTime.now(),
        isStatus: isStatus,
      ),
    ];
  }

  void clear() {
    state = [];
  }
}

final chatProvider = StateNotifierProvider<ChatNotifier, List<ChatMessage>>(
  (ref) => ChatNotifier(),
);
