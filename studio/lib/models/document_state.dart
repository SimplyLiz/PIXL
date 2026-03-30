import '../models/animation_state.dart';
import '../models/palette.dart';
import '../models/pixel_canvas.dart';
import '../providers/chat_provider.dart';
import '../providers/style_provider.dart';
import '../providers/backdrop_provider.dart';
import '../providers/composite_provider.dart';

/// Unique identifier for an open document tab.
typedef TabId = String;

/// Complete snapshot of a single document/tab's state.
///
/// Used by [TabManagerNotifier] to save/restore per-tab state
/// when switching between tabs.
class DocumentState {
  DocumentState({
    required this.id,
    required this.name,
    this.filePath,
    required this.canvasState,
    this.canvasUndoStack = const [],
    this.canvasRedoStack = const [],
    required this.palette,
    required this.chatMessages,
    required this.tilemapState,
    this.tilemapUndoStack = const [],
    this.tilemapRedoStack = const [],
    required this.styleState,
    required this.backdropState,
    required this.compositeState,
    this.spriteAnimationState = const SpriteAnimationState(),
    required this.editorMode,
    this.paxSource,
    this.isDirty = false,
    DateTime? createdAt,
  }) : createdAt = createdAt ?? DateTime.now();

  final TabId id;
  String name;
  String? filePath;
  bool isDirty;
  final DateTime createdAt;

  // Canvas
  CanvasState canvasState;
  List<CanvasSnapshot> canvasUndoStack;
  List<CanvasSnapshot> canvasRedoStack;

  // Palette
  PixlPalette palette;

  // Chat
  List<ChatMessage> chatMessages;

  // Tilemap
  TilemapState tilemapState;
  List<TilemapSnapshot> tilemapUndoStack;
  List<TilemapSnapshot> tilemapRedoStack;

  // Style
  StyleState styleState;

  // Backdrop & Composite
  BackdropEditorState backdropState;
  CompositeEditorState compositeState;

  // Sprite animation preview
  SpriteAnimationState spriteAnimationState;

  // Editor mode
  EditorMode editorMode;

  // Engine session — saved PAX source for tab-switch engine swaps
  String? paxSource;
}
