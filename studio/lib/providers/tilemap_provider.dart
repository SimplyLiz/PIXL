import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/pixel_canvas.dart';

/// Tracks which editor mode is active (pixel vs tilemap).
final editorModeProvider = StateProvider<EditorMode>((ref) => EditorMode.pixel);

/// Tilemap notifier — manages the 2D tile grid, tools, and undo/redo.
class TilemapNotifier extends StateNotifier<TilemapState> {
  TilemapNotifier() : super(TilemapState());

  static const _maxUndo = 50;
  final _undoStack = <TilemapSnapshot>[];
  final _redoStack = <TilemapSnapshot>[];
  bool _inStroke = false;

  bool get canUndo => _undoStack.isNotEmpty;
  bool get canRedo => _redoStack.isNotEmpty;

  /// Expose undo/redo stacks for tab save/restore.
  List<TilemapSnapshot> get undoStack => List.unmodifiable(_undoStack);
  List<TilemapSnapshot> get redoStack => List.unmodifiable(_redoStack);

  /// Restore full document state (used by tab manager on tab switch).
  void restoreDocument(TilemapState newState, List<TilemapSnapshot> undo, List<TilemapSnapshot> redo) {
    _undoStack
      ..clear()
      ..addAll(undo);
    _redoStack
      ..clear()
      ..addAll(redo);
    state = newState;
  }

  // ── Undo / Redo ──────────────────────────────────

  void _pushSnapshot() {
    final snap = TilemapSnapshot(
      cells: state.cells.map((row) => List<String?>.from(row)).toList(),
    );
    _undoStack.add(snap);
    if (_undoStack.length > _maxUndo) _undoStack.removeAt(0);
    _redoStack.clear();
  }

  void undo() {
    if (_undoStack.isEmpty) return;
    // Save current state to redo
    _redoStack.add(
      TilemapSnapshot(
        cells: state.cells.map((row) => List<String?>.from(row)).toList(),
      ),
    );
    final snap = _undoStack.removeLast();
    state = state.copyWith(
      cells: snap.cells,
      gridWidth: snap.cells.isNotEmpty ? snap.cells[0].length : state.gridWidth,
      gridHeight: snap.cells.length,
    );
  }

  void redo() {
    if (_redoStack.isEmpty) return;
    _undoStack.add(
      TilemapSnapshot(
        cells: state.cells.map((row) => List<String?>.from(row)).toList(),
      ),
    );
    final snap = _redoStack.removeLast();
    state = state.copyWith(
      cells: snap.cells,
      gridWidth: snap.cells.isNotEmpty ? snap.cells[0].length : state.gridWidth,
      gridHeight: snap.cells.length,
    );
  }

  // ── Tools ────────────────────────────────────────

  void setTool(TilemapTool tool) {
    state = state.copyWith(activeTool: tool);
  }

  void setSelectedTile(String? name) {
    if (name == null) {
      state = state.copyWith(clearSelectedTile: true);
    } else {
      state = state.copyWith(selectedTile: name);
    }
  }

  // ── Stamp / Erase (stroke-based) ────────────────

  void beginStamp(int col, int row) {
    if (col < 0 || col >= state.gridWidth || row < 0 || row >= state.gridHeight)
      return;
    _pushSnapshot();
    _inStroke = true;
    _setCell(col, row);
    _notifyChange();
  }

  void continueStamp(int col, int row) {
    if (!_inStroke) return;
    if (col < 0 || col >= state.gridWidth || row < 0 || row >= state.gridHeight)
      return;
    _setCell(col, row);
    _notifyChange();
  }

  void endStroke() {
    _inStroke = false;
  }

  void _setCell(int col, int row) {
    switch (state.activeTool) {
      case TilemapTool.stamp:
        state.cells[row][col] = state.selectedTile;
        break;
      case TilemapTool.eraser:
        state.cells[row][col] = null;
        break;
      default:
        break;
    }
  }

  void _notifyChange() {
    state = state.copyWith(cells: List.from(state.cells));
  }

  // ── Bucket Fill ──────────────────────────────────

  void bucketFill(int col, int row) {
    if (col < 0 || col >= state.gridWidth || row < 0 || row >= state.gridHeight)
      return;
    _pushSnapshot();

    final fillWith = state.activeTool == TilemapTool.eraser
        ? null
        : state.selectedTile;
    final target = state.cells[row][col];
    if (target == fillWith) return;

    final stack = <(int, int)>[(col, row)];
    final visited = <int>{};
    final w = state.gridWidth;

    while (stack.isNotEmpty) {
      final (cx, cy) = stack.removeLast();
      final idx = cy * w + cx;
      if (visited.contains(idx)) continue;
      if (cx < 0 || cx >= state.gridWidth || cy < 0 || cy >= state.gridHeight)
        continue;
      if (state.cells[cy][cx] != target) continue;

      visited.add(idx);
      state.cells[cy][cx] = fillWith;

      stack.add((cx + 1, cy));
      stack.add((cx - 1, cy));
      stack.add((cx, cy + 1));
      stack.add((cx, cy - 1));
    }

    _notifyChange();
  }

  // ── Eyedropper ───────────────────────────────────

  void pickTile(int col, int row) {
    final tile = state.cellAt(col, row);
    if (tile != null) {
      state = state.copyWith(selectedTile: tile, activeTool: TilemapTool.stamp);
    }
  }

  // ── Grid Management ──────────────────────────────

  void resize(int width, int height) {
    _pushSnapshot();
    final newCells = List.generate(height, (row) {
      return List.generate(width, (col) {
        if (row < state.gridHeight && col < state.gridWidth) {
          return state.cells[row][col];
        }
        return null;
      });
    });
    state = state.copyWith(
      gridWidth: width,
      gridHeight: height,
      cells: newCells,
    );
  }

  void clear() {
    _pushSnapshot();
    state = state.copyWith(
      cells: List.generate(
        state.gridHeight,
        (_) => List.filled(state.gridWidth, null),
      ),
    );
  }

  /// Load a tile grid from WFC narrate result or PAX source.
  void loadFromGrid(List<List<String?>> grid) {
    _pushSnapshot();
    final h = grid.length;
    final w = h > 0 ? grid[0].length : 0;
    state = state.copyWith(gridWidth: w, gridHeight: h, cells: grid);
  }

  // ── Zoom / Grid ──────────────────────────────────

  static const _zoomLevels = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0];

  void zoomIn() {
    final idx = _zoomLevels.indexWhere((z) => z > state.zoomLevel);
    if (idx >= 0) state = state.copyWith(zoomLevel: _zoomLevels[idx]);
  }

  void zoomOut() {
    final idx = _zoomLevels.lastIndexWhere((z) => z < state.zoomLevel);
    if (idx >= 0) state = state.copyWith(zoomLevel: _zoomLevels[idx]);
  }

  void resetZoom() {
    state = state.copyWith(zoomLevel: _zoomLevels[1]); // default 2.0
  }

  void toggleGrid() {
    state = state.copyWith(showGrid: !state.showGrid);
  }

  // ── Play Mode (Zelda-style screen scroll) ───────

  void togglePlayMode() {
    final entering = !state.playMode;
    if (entering) {
      // Place player at center of first screen
      final startCol = (state.screenTilesX ~/ 2).clamp(0, state.gridWidth - 1);
      final startRow = (state.screenTilesY ~/ 2).clamp(0, state.gridHeight - 1);
      state = state.copyWith(
        playMode: true,
        playerCol: startCol,
        playerRow: startRow,
        screenX: 0,
        screenY: 0,
        transitioning: false,
        transitionProgress: 0.0,
      );
    } else {
      state = state.copyWith(
        playMode: false,
        transitioning: false,
      );
    }
  }

  /// Move player by (dx, dy) in tile units. Returns true if a screen
  /// transition was triggered.
  bool movePlayer(int dx, int dy) {
    if (!state.playMode || state.transitioning) return false;

    final newCol = state.playerCol + dx;
    final newRow = state.playerRow + dy;

    // Clamp to map bounds
    if (newCol < 0 || newCol >= state.gridWidth ||
        newRow < 0 || newRow >= state.gridHeight) {
      return false;
    }

    // Check which screen the new position falls on
    final newScreenX = newCol ~/ state.screenTilesX;
    final newScreenY = newRow ~/ state.screenTilesY;

    if (newScreenX != state.screenX || newScreenY != state.screenY) {
      // Screen transition — start animated scroll
      state = state.copyWith(
        playerCol: newCol,
        playerRow: newRow,
        prevScreenX: state.screenX,
        prevScreenY: state.screenY,
        screenX: newScreenX,
        screenY: newScreenY,
        transitioning: true,
        transitionProgress: 0.0,
      );
      return true;
    }

    // Same screen — just move
    state = state.copyWith(playerCol: newCol, playerRow: newRow);
    return false;
  }

  /// Advance the screen transition animation. Called by the viewport's ticker.
  void updateTransition(double progress) {
    if (!state.transitioning) return;
    if (progress >= 1.0) {
      state = state.copyWith(
        transitioning: false,
        transitionProgress: 1.0,
      );
    } else {
      state = state.copyWith(transitionProgress: progress);
    }
  }
}

final tilemapProvider = StateNotifierProvider<TilemapNotifier, TilemapState>(
  (ref) => TilemapNotifier(),
);
