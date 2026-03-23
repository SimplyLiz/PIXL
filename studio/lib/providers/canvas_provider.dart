import 'dart:ui' show Color;

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/palette.dart';
import '../models/pixel_canvas.dart';

/// Manages the canvas state: pixels, layers, tool, zoom, etc.
class CanvasNotifier extends StateNotifier<CanvasState> {
  CanvasNotifier() : super(CanvasState());

  static const int maxUndoSteps = 50;
  final List<CanvasSnapshot> _undoStack = [];
  final List<CanvasSnapshot> _redoStack = [];

  // -- Snapshot / Undo / Redo --

  void _pushSnapshot() {
    _undoStack.add(CanvasSnapshot(
      layerPixels: state.layers.map((l) => List<Color?>.from(l.pixels)).toList(),
    ));
    if (_undoStack.length > maxUndoSteps) {
      _undoStack.removeAt(0);
    }
    _redoStack.clear();
  }

  void undo() {
    if (_undoStack.isEmpty) return;
    // Save current for redo
    _redoStack.add(CanvasSnapshot(
      layerPixels: state.layers.map((l) => List<Color?>.from(l.pixels)).toList(),
    ));
    final snapshot = _undoStack.removeLast();
    _restoreSnapshot(snapshot);
  }

  void redo() {
    if (_redoStack.isEmpty) return;
    _undoStack.add(CanvasSnapshot(
      layerPixels: state.layers.map((l) => List<Color?>.from(l.pixels)).toList(),
    ));
    final snapshot = _redoStack.removeLast();
    _restoreSnapshot(snapshot);
  }

  void _restoreSnapshot(CanvasSnapshot snapshot) {
    final newLayers = <PixelLayer>[];
    for (var i = 0; i < state.layers.length; i++) {
      final layer = state.layers[i].deepCopy();
      layer.pixels.clear();
      layer.pixels.addAll(snapshot.layerPixels[i]);
      newLayers.add(layer);
    }
    state = state.copyWith(layers: newLayers);
  }

  bool get canUndo => _undoStack.isNotEmpty;
  bool get canRedo => _redoStack.isNotEmpty;

  // -- Drawing --

  void drawPixel(int x, int y, Color? color) {
    if (x < 0 || x >= state.width || y < 0 || y >= state.height) return;
    final layer = state.activeLayer;
    if (!layer.visible) return;

    _pushSnapshot();
    _setPixelWithSymmetry(x, y, color);
    state = state.copyWith(layers: List.from(state.layers));
  }

  void _setPixelWithSymmetry(int x, int y, Color? color) {
    final layer = state.activeLayer;
    final w = state.width;
    final h = state.height;

    layer.pixels[y * w + x] = color;

    switch (state.symmetryMode) {
      case SymmetryMode.horizontal:
        layer.pixels[y * w + (w - 1 - x)] = color;
        break;
      case SymmetryMode.vertical:
        layer.pixels[(h - 1 - y) * w + x] = color;
        break;
      case SymmetryMode.both:
        layer.pixels[y * w + (w - 1 - x)] = color;
        layer.pixels[(h - 1 - y) * w + x] = color;
        layer.pixels[(h - 1 - y) * w + (w - 1 - x)] = color;
        break;
      case SymmetryMode.none:
        break;
    }
  }

  /// Continuously draw without pushing undo for each pixel.
  /// Call [beginStroke] before a drag, [continueStroke] during, [endStroke] after.
  bool _inStroke = false;

  void beginStroke(int x, int y, Color? color) {
    if (x < 0 || x >= state.width || y < 0 || y >= state.height) return;
    if (!state.activeLayer.visible) return;
    _pushSnapshot();
    _inStroke = true;
    _setPixelWithSymmetry(x, y, color);
    state = state.copyWith(layers: List.from(state.layers));
  }

  void continueStroke(int x, int y, Color? color) {
    if (!_inStroke) return;
    if (x < 0 || x >= state.width || y < 0 || y >= state.height) return;
    _setPixelWithSymmetry(x, y, color);
    state = state.copyWith(layers: List.from(state.layers));
  }

  void endStroke() {
    _inStroke = false;
  }

  /// Flood fill from (x, y) with [fillColor].
  void bucketFill(int x, int y, Color? fillColor) {
    if (x < 0 || x >= state.width || y < 0 || y >= state.height) return;
    final layer = state.activeLayer;
    if (!layer.visible) return;

    final w = state.width;
    final h = state.height;
    final targetColor = layer.pixels[y * w + x];

    if (targetColor == fillColor) return;

    _pushSnapshot();

    final stack = <(int, int)>[(x, y)];
    final visited = <int>{};

    while (stack.isNotEmpty) {
      final (cx, cy) = stack.removeLast();
      final idx = cy * w + cx;
      if (visited.contains(idx)) continue;
      if (cx < 0 || cx >= w || cy < 0 || cy >= h) continue;
      if (layer.pixels[idx] != targetColor) continue;

      visited.add(idx);
      layer.pixels[idx] = fillColor;

      stack.add((cx + 1, cy));
      stack.add((cx - 1, cy));
      stack.add((cx, cy + 1));
      stack.add((cx, cy - 1));
    }

    state = state.copyWith(layers: List.from(state.layers));
  }

  /// Pick color at (x, y) from the composite view.
  Color? pickColor(int x, int y) {
    if (x < 0 || x >= state.width || y < 0 || y >= state.height) return null;
    final w = state.width;
    // Check from top layer down
    for (var i = state.layers.length - 1; i >= 0; i--) {
      final layer = state.layers[i];
      if (!layer.visible) continue;
      final color = layer.pixels[y * w + x];
      if (color != null) return color;
    }
    return null;
  }

  // -- Tool / Settings --

  void setTool(DrawingTool tool) {
    state = state.copyWith(activeTool: tool);
  }

  void setSymmetry(SymmetryMode mode) {
    state = state.copyWith(symmetryMode: mode);
  }

  void setZoom(double zoom) {
    state = state.copyWith(zoomLevel: zoom.clamp(2.0, 32.0));
  }

  void toggleGrid() {
    state = state.copyWith(showGrid: !state.showGrid);
  }

  void setActiveLayer(int index) {
    if (index >= 0 && index < state.layers.length) {
      state = state.copyWith(activeLayerIndex: index);
    }
  }

  void toggleLayerVisibility(int index) {
    if (index >= 0 && index < state.layers.length) {
      final layers = List<PixelLayer>.from(state.layers);
      layers[index] = layers[index].copyWith(visible: !layers[index].visible);
      state = state.copyWith(layers: layers);
    }
  }

  void setForegroundColor(int index) {
    state = state.copyWith(foregroundColorIndex: index);
  }

  void setBackgroundColor(int index) {
    state = state.copyWith(backgroundColorIndex: index);
  }

  void setCanvasSize(CanvasSize size) {
    if (size == state.canvasSize) return;
    state = CanvasState(
      canvasSize: size,
      activeTool: state.activeTool,
      symmetryMode: state.symmetryMode,
      zoomLevel: state.zoomLevel,
      showGrid: state.showGrid,
      foregroundColorIndex: state.foregroundColorIndex,
      backgroundColorIndex: state.backgroundColorIndex,
    );
    _undoStack.clear();
    _redoStack.clear();
  }

  /// Clear all pixels on all layers.
  void clearCanvas() {
    _pushSnapshot();
    for (final layer in state.layers) {
      layer.pixels.fillRange(0, layer.pixels.length, null);
    }
    state = state.copyWith(layers: List.from(state.layers));
  }
}

final canvasProvider = StateNotifierProvider<CanvasNotifier, CanvasState>(
  (ref) => CanvasNotifier(),
);
