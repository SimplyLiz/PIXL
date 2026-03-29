import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../models/document_state.dart';
import '../models/palette.dart';
import '../models/pixel_canvas.dart';
import 'animation_provider.dart';
import 'backdrop_provider.dart';
import 'canvas_provider.dart';
import 'chat_provider.dart';
import 'composite_provider.dart';
import 'palette_provider.dart';
import 'style_provider.dart';
import 'tilemap_provider.dart';

const _uuid = Uuid();

/// Tab manager state — ordered list of open documents.
class TabManagerState {
  const TabManagerState({
    this.tabOrder = const [],
    this.activeTabId,
    this.documents = const {},
  });

  final List<TabId> tabOrder;
  final TabId? activeTabId;
  final Map<TabId, DocumentState> documents;

  DocumentState? get activeDocument =>
      activeTabId != null ? documents[activeTabId] : null;

  TabManagerState copyWith({
    List<TabId>? tabOrder,
    TabId? activeTabId,
    Map<TabId, DocumentState>? documents,
    bool clearActiveTab = false,
  }) {
    return TabManagerState(
      tabOrder: tabOrder ?? this.tabOrder,
      activeTabId: clearActiveTab ? null : (activeTabId ?? this.activeTabId),
      documents: documents ?? this.documents,
    );
  }
}

/// Manages open document tabs and orchestrates save/restore of per-tab state.
class TabManagerNotifier extends StateNotifier<TabManagerState> {
  TabManagerNotifier(this._ref) : super(const TabManagerState());

  final Ref _ref;
  bool _isRestoring = false;

  /// Create a new tab with the given settings.
  TabId newTab({
    String name = 'untitled',
    CanvasSize canvasSize = CanvasSize.s16x16,
    PixlPalette? palette,
  }) {
    // Save current tab before switching.
    _saveCurrentTab();

    final id = _uuid.v4();
    final doc = DocumentState(
      id: id,
      name: name,
      canvasState: CanvasState(canvasSize: canvasSize),
      palette: palette ?? BuiltInPalettes.darkFantasy,
      chatMessages: const [],
      tilemapState: TilemapState(),
      styleState: const StyleState(),
      backdropState: const BackdropEditorState(),
      compositeState: const CompositeEditorState(),
      editorMode: EditorMode.pixel,
    );

    final newDocs = Map<TabId, DocumentState>.from(state.documents);
    newDocs[id] = doc;

    state = state.copyWith(
      tabOrder: [...state.tabOrder, id],
      activeTabId: id,
      documents: newDocs,
    );

    // Restore the new tab's state into providers.
    _restoreTab(doc);
    return id;
  }

  /// Open a file in a new tab.
  TabId openTab({
    required String name,
    String? filePath,
    PixlPalette? palette,
    CanvasSize canvasSize = CanvasSize.s16x16,
  }) {
    final id = newTab(name: name, canvasSize: canvasSize, palette: palette);
    if (filePath != null) {
      final docs = Map<TabId, DocumentState>.from(state.documents);
      docs[id]!.filePath = filePath;
      state = state.copyWith(documents: docs);
    }
    return id;
  }

  /// Switch to an existing tab.
  void switchTab(TabId id) {
    if (id == state.activeTabId) return;
    if (!state.documents.containsKey(id)) return;

    _saveCurrentTab();

    state = state.copyWith(activeTabId: id);
    _restoreTab(state.documents[id]!);
  }

  /// Close a tab. Returns false if it was the last tab.
  bool closeTab(TabId id) {
    if (!state.documents.containsKey(id)) return true;

    final newOrder = state.tabOrder.where((t) => t != id).toList();
    final newDocs = Map<TabId, DocumentState>.from(state.documents)..remove(id);

    if (newOrder.isEmpty) {
      // Last tab closed — create a fresh default tab.
      state = state.copyWith(
        tabOrder: newOrder,
        documents: newDocs,
        clearActiveTab: true,
      );
      newTab();
      return true;
    }

    // If closing the active tab, switch to the nearest neighbor.
    TabId? newActive = state.activeTabId;
    if (id == state.activeTabId) {
      final oldIndex = state.tabOrder.indexOf(id);
      final newIndex = oldIndex >= newOrder.length ? newOrder.length - 1 : oldIndex;
      newActive = newOrder[newIndex];
    }

    state = state.copyWith(
      tabOrder: newOrder,
      activeTabId: newActive,
      documents: newDocs,
    );

    if (newActive != null && newActive != state.activeTabId) {
      _restoreTab(state.documents[newActive]!);
    } else if (id == state.activeTabId) {
      _restoreTab(state.documents[newActive!]!);
    }

    return true;
  }

  /// Reorder tabs by drag.
  void reorderTab(int oldIndex, int newIndex) {
    final order = List<TabId>.from(state.tabOrder);
    if (newIndex > oldIndex) newIndex--;
    final tab = order.removeAt(oldIndex);
    order.insert(newIndex, tab);
    state = state.copyWith(tabOrder: order);
  }

  /// Mark the active tab as dirty (has unsaved changes).
  void markDirty() {
    final id = state.activeTabId;
    if (id == null) return;
    final doc = state.documents[id];
    if (doc == null || doc.isDirty) return;
    doc.isDirty = true;
    state = state.copyWith(documents: Map.from(state.documents));
  }

  /// Mark a tab as clean (just saved).
  void markClean(TabId id) {
    final doc = state.documents[id];
    if (doc == null || !doc.isDirty) return;
    doc.isDirty = false;
    state = state.copyWith(documents: Map.from(state.documents));
  }

  /// Rename a tab.
  void renameTab(TabId id, String name) {
    final doc = state.documents[id];
    if (doc == null) return;
    doc.name = name;
    state = state.copyWith(documents: Map.from(state.documents));
  }

  // ── Save / Restore ────────────────────────────────────

  void _saveCurrentTab() {
    final id = state.activeTabId;
    if (id == null) return;
    final doc = state.documents[id];
    if (doc == null) return;

    final canvas = _ref.read(canvasProvider.notifier);
    doc.canvasState = _ref.read(canvasProvider);
    doc.canvasUndoStack = List.from(canvas.undoStack);
    doc.canvasRedoStack = List.from(canvas.redoStack);

    doc.palette = _ref.read(paletteProvider);
    doc.chatMessages = List.from(_ref.read(chatProvider));

    final tilemap = _ref.read(tilemapProvider.notifier);
    doc.tilemapState = _ref.read(tilemapProvider);
    doc.tilemapUndoStack = List.from(tilemap.undoStack);
    doc.tilemapRedoStack = List.from(tilemap.redoStack);

    doc.styleState = _ref.read(styleProvider);
    doc.backdropState = _ref.read(backdropEditorProvider);
    doc.compositeState = _ref.read(compositeEditorProvider);
    // Capture current canvas pixels into the active animation frame.
    _ref.read(spriteAnimationProvider.notifier)
        .captureCurrentFrame(_ref.read(canvasProvider));
    doc.spriteAnimationState = _ref.read(spriteAnimationProvider);
    doc.editorMode = _ref.read(editorModeProvider);
  }

  void _restoreTab(DocumentState doc) {
    _isRestoring = true;

    _ref.read(canvasProvider.notifier).restoreDocument(
      doc.canvasState,
      List.from(doc.canvasUndoStack),
      List.from(doc.canvasRedoStack),
    );

    _ref.read(paletteProvider.notifier).setPalette(doc.palette);
    _ref.read(chatProvider.notifier).restoreMessages(doc.chatMessages);

    _ref.read(tilemapProvider.notifier).restoreDocument(
      doc.tilemapState,
      List.from(doc.tilemapUndoStack),
      List.from(doc.tilemapRedoStack),
    );

    _ref.read(styleProvider.notifier).restore(doc.styleState);
    _ref.read(backdropEditorProvider.notifier).restore(doc.backdropState);
    _ref.read(compositeEditorProvider.notifier).restore(doc.compositeState);
    _ref.read(spriteAnimationProvider.notifier).restore(doc.spriteAnimationState);
    _ref.read(editorModeProvider.notifier).state = doc.editorMode;

    _isRestoring = false;
  }

  /// Whether the tab manager is currently restoring state (suppress dirty tracking).
  bool get isRestoring => _isRestoring;
}

final tabManagerProvider =
    StateNotifierProvider<TabManagerNotifier, TabManagerState>(
  (ref) => TabManagerNotifier(ref),
);
