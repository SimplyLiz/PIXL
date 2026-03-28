import 'dart:ui' show Color;

import 'package:flutter_riverpod/flutter_riverpod.dart';

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
    // Guard against layer count mismatch (e.g. canvas resize between pushes).
    if (snapshot.layerPixels.length != state.layers.length) {
      return;
    }
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
    // Reuse the same list reference — the pixel data was mutated in-place.
    // Force state notification by creating a new CanvasState wrapper only.
    state = state.copyWith();
  }

  void endStroke() {
    _inStroke = false;
  }

  /// Flood fill from (x, y) with [fillColor]. Respects symmetry mode.
  void bucketFill(int x, int y, Color? fillColor) {
    if (x < 0 || x >= state.width || y < 0 || y >= state.height) return;
    final layer = state.activeLayer;
    if (!layer.visible) return;

    final w = state.width;
    final h = state.height;

    _pushSnapshot();

    // Fill at the clicked point and all symmetry mirrors.
    final origins = <(int, int)>[(x, y)];
    switch (state.symmetryMode) {
      case SymmetryMode.horizontal:
        origins.add((w - 1 - x, y));
        break;
      case SymmetryMode.vertical:
        origins.add((x, h - 1 - y));
        break;
      case SymmetryMode.both:
        origins.add((w - 1 - x, y));
        origins.add((x, h - 1 - y));
        origins.add((w - 1 - x, h - 1 - y));
        break;
      case SymmetryMode.none:
        break;
    }

    for (final (ox, oy) in origins) {
      if (ox < 0 || ox >= w || oy < 0 || oy >= h) continue;
      final targetColor = layer.pixels[oy * w + ox];
      if (targetColor == fillColor) continue;

      final stack = <(int, int)>[(ox, oy)];
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

  static const _zoomLevels = [2.0, 4.0, 8.0, 14.0, 20.0, 32.0];

  void setZoom(double zoom) {
    // Snap to nearest discrete zoom level
    final clamped = zoom.clamp(2.0, 32.0);
    var best = _zoomLevels.first;
    var bestDist = (clamped - best).abs();
    for (final level in _zoomLevels) {
      final dist = (clamped - level).abs();
      if (dist < bestDist) {
        best = level;
        bestDist = dist;
      }
    }
    state = state.copyWith(zoomLevel: best);
  }

  void zoomIn() {
    final idx = _zoomLevels.indexOf(state.zoomLevel);
    if (idx < 0) {
      setZoom(state.zoomLevel + 2);
    } else if (idx < _zoomLevels.length - 1) {
      state = state.copyWith(zoomLevel: _zoomLevels[idx + 1]);
    }
  }

  void zoomOut() {
    final idx = _zoomLevels.indexOf(state.zoomLevel);
    if (idx < 0) {
      setZoom(state.zoomLevel - 2);
    } else if (idx > 0) {
      state = state.copyWith(zoomLevel: _zoomLevels[idx - 1]);
    }
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

  /// Add a new empty layer above the current active layer.
  void addLayer(String name, {String? targetLayer}) {
    final layers = List<PixelLayer>.from(state.layers);
    final newLayer = PixelLayer(
      name: name,
      width: state.width,
      height: state.height,
      targetLayer: targetLayer,
    );
    layers.insert(state.activeLayerIndex + 1, newLayer);
    state = state.copyWith(
      layers: layers,
      activeLayerIndex: state.activeLayerIndex + 1,
    );
    _undoStack.clear();
    _redoStack.clear();
  }

  /// Remove a layer by index. Must keep at least 1 layer.
  void removeLayer(int index) {
    if (state.layers.length <= 1) return;
    if (index < 0 || index >= state.layers.length) return;
    final layers = List<PixelLayer>.from(state.layers)..removeAt(index);
    var activeIdx = state.activeLayerIndex;
    if (activeIdx >= layers.length) activeIdx = layers.length - 1;
    state = state.copyWith(layers: layers, activeLayerIndex: activeIdx);
    _undoStack.clear();
    _redoStack.clear();
  }

  /// Rename a layer.
  void renameLayer(int index, String name) {
    if (index < 0 || index >= state.layers.length) return;
    final layers = List<PixelLayer>.from(state.layers);
    layers[index] = layers[index].copyWith(name: name);
    state = state.copyWith(layers: layers);
  }

  /// Move a layer up (higher z-order).
  void moveLayerUp(int index) {
    if (index <= 0 || index >= state.layers.length) return;
    final layers = List<PixelLayer>.from(state.layers);
    final layer = layers.removeAt(index);
    layers.insert(index - 1, layer);
    state = state.copyWith(
      layers: layers,
      activeLayerIndex: state.activeLayerIndex == index
          ? index - 1
          : state.activeLayerIndex,
    );
  }

  /// Move a layer down (lower z-order).
  void moveLayerDown(int index) {
    if (index < 0 || index >= state.layers.length - 1) return;
    final layers = List<PixelLayer>.from(state.layers);
    final layer = layers.removeAt(index);
    layers.insert(index + 1, layer);
    state = state.copyWith(
      layers: layers,
      activeLayerIndex: state.activeLayerIndex == index
          ? index + 1
          : state.activeLayerIndex,
    );
  }

  /// Set layer opacity (0.0 – 1.0).
  void setLayerOpacity(int index, double opacity) {
    if (index < 0 || index >= state.layers.length) return;
    final layers = List<PixelLayer>.from(state.layers);
    layers[index] = layers[index].copyWith(opacity: opacity.clamp(0.0, 1.0));
    state = state.copyWith(layers: layers);
  }

  /// Set layer blend mode.
  void setLayerBlendMode(int index, BlendMode mode) {
    if (index < 0 || index >= state.layers.length) return;
    final layers = List<PixelLayer>.from(state.layers);
    layers[index] = layers[index].copyWith(blendMode: mode);
    state = state.copyWith(layers: layers);
  }

  /// Set layer target layer role.
  void setLayerTargetLayer(int index, String? targetLayer) {
    if (index < 0 || index >= state.layers.length) return;
    final layers = List<PixelLayer>.from(state.layers);
    layers[index] = layers[index].copyWith(targetLayer: targetLayer ?? '');
    state = state.copyWith(layers: layers);
  }

  void setForegroundColor(int index) {
    state = state.copyWith(foregroundColorIndex: index);
  }

  void setBackgroundColor(int index) {
    state = state.copyWith(backgroundColorIndex: index);
  }

  /// Clamp color indices to [maxIndex]. Call when palette changes.
  void clampColorIndices(int maxIndex) {
    final lastValid = maxIndex - 1;
    if (state.foregroundColorIndex > lastValid ||
        state.backgroundColorIndex > lastValid) {
      state = state.copyWith(
        foregroundColorIndex: state.foregroundColorIndex.clamp(0, lastValid),
        backgroundColorIndex: state.backgroundColorIndex.clamp(0, lastValid),
      );
    }
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

  // ── Line Tool (Bresenham) ────────────────────────

  /// Draw a line from (x0,y0) to (x1,y1) using Bresenham's algorithm.
  void drawLine(int x0, int y0, int x1, int y1, Color? color) {
    _pushSnapshot();
    _bresenham(x0, y0, x1, y1, color);
    state = state.copyWith(layers: List.from(state.layers));
  }

  void _bresenham(int x0, int y0, int x1, int y1, Color? color) {
    var dx = (x1 - x0).abs();
    var dy = -(y1 - y0).abs();
    final sx = x0 < x1 ? 1 : -1;
    final sy = y0 < y1 ? 1 : -1;
    var err = dx + dy;

    while (true) {
      _setPixelWithSymmetry(x0, y0, color);
      if (x0 == x1 && y0 == y1) break;
      final e2 = 2 * err;
      if (e2 >= dy) { err += dy; x0 += sx; }
      if (e2 <= dx) { err += dx; y0 += sy; }
    }
  }

  // ── Rectangle Tool ───────────────────────────────

  /// Draw a rectangle outline (or filled if [filled] is true).
  void drawRect(int x0, int y0, int x1, int y1, Color? color, {bool filled = false}) {
    _pushSnapshot();
    final minX = x0 < x1 ? x0 : x1;
    final maxX = x0 > x1 ? x0 : x1;
    final minY = y0 < y1 ? y0 : y1;
    final maxY = y0 > y1 ? y0 : y1;

    if (filled) {
      for (var y = minY; y <= maxY; y++) {
        for (var x = minX; x <= maxX; x++) {
          _setPixelWithSymmetry(x, y, color);
        }
      }
    } else {
      for (var x = minX; x <= maxX; x++) {
        _setPixelWithSymmetry(x, minY, color);
        _setPixelWithSymmetry(x, maxY, color);
      }
      for (var y = minY + 1; y < maxY; y++) {
        _setPixelWithSymmetry(minX, y, color);
        _setPixelWithSymmetry(maxX, y, color);
      }
    }
    state = state.copyWith(layers: List.from(state.layers));
  }

  // ── Selection / Copy / Paste ─────────────────────

  /// Copy pixels from the selection region on the active layer.
  List<Color?> copyRegion(int sx, int sy, int sw, int sh) {
    final w = state.width;
    final layer = state.activeLayer;
    final buffer = <Color?>[];
    for (var y = sy; y < sy + sh; y++) {
      for (var x = sx; x < sx + sw; x++) {
        if (x >= 0 && x < w && y >= 0 && y < state.height) {
          buffer.add(layer.pixels[y * w + x]);
        } else {
          buffer.add(null);
        }
      }
    }
    return buffer;
  }

  /// Paste pixels at position.
  void pasteRegion(int dx, int dy, List<Color?> data, int pw, int ph) {
    _pushSnapshot();
    final w = state.width;
    final h = state.height;
    final layer = state.activeLayer;
    for (var y = 0; y < ph; y++) {
      for (var x = 0; x < pw; x++) {
        final tx = dx + x;
        final ty = dy + y;
        if (tx < 0 || tx >= w || ty < 0 || ty >= h) continue;
        final color = data[y * pw + x];
        if (color != null) {
          layer.pixels[ty * w + tx] = color;
        }
      }
    }
    state = state.copyWith(layers: List.from(state.layers));
  }

  /// Clear pixels in a region.
  void clearRegion(int sx, int sy, int sw, int sh) {
    _pushSnapshot();
    final w = state.width;
    final h = state.height;
    final layer = state.activeLayer;
    for (var y = sy; y < sy + sh; y++) {
      for (var x = sx; x < sx + sw; x++) {
        if (x >= 0 && x < w && y >= 0 && y < h) {
          layer.pixels[y * w + x] = null;
        }
      }
    }
    state = state.copyWith(layers: List.from(state.layers));
  }
}

final canvasProvider = StateNotifierProvider<CanvasNotifier, CanvasState>(
  (ref) => CanvasNotifier(),
);

/// Blueprint landmarks overlay toggle + data.
final blueprintProvider = StateProvider<List<Map<String, dynamic>>?>((ref) => null);

/// Selection state for copy/paste.
final selectionProvider = StateProvider<SelectionState>((ref) => const SelectionState());

/// Reference image overlay (dart:ui.Image stored externally, path tracked here).
final referenceImagePathProvider = StateProvider<String?>((ref) => null);
