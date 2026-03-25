import 'dart:ui' show Color;

import 'package:flutter/foundation.dart';

/// Canvas sizes available in the editor.
enum CanvasSize {
  s8x8(8, 8),
  s16x16(16, 16),
  s32x32(32, 32),
  s48x48(48, 48),
  s64x64(64, 64);

  const CanvasSize(this.width, this.height);
  final int width;
  final int height;

  String get label => '$width×$height';
}

/// Drawing tools available in the editor.
enum DrawingTool {
  pencil,
  eraser,
  bucket,
  eyedropper,
  rectSelect,
  move,
}

/// Symmetry modes for mirrored drawing.
enum SymmetryMode {
  none,
  horizontal,
  vertical,
  both,
}

/// Blend modes for layer compositing.
enum BlendMode {
  normal,
  multiply,
  screen,
  add,
}

/// A single layer of pixel data with properties.
class PixelLayer {
  PixelLayer({
    required this.name,
    required int width,
    required int height,
    this.visible = true,
    this.opacity = 1.0,
    this.blendMode = BlendMode.normal,
    this.targetLayer,
  }) : pixels = List.filled(width * height, null);

  PixelLayer._fromPixels({
    required this.name,
    required List<Color?> source,
    this.visible = true,
    this.opacity = 1.0,
    this.blendMode = BlendMode.normal,
    this.targetLayer,
  }) : pixels = List<Color?>.from(source);

  final String name;
  final List<Color?> pixels;
  bool visible;
  double opacity; // 0.0 – 1.0
  BlendMode blendMode;
  /// PAX target layer role for tilemap placement.
  String? targetLayer;

  PixelLayer copyWith({
    String? name,
    bool? visible,
    double? opacity,
    BlendMode? blendMode,
    String? targetLayer,
  }) {
    return PixelLayer._fromPixels(
      name: name ?? this.name,
      source: pixels,
      visible: visible ?? this.visible,
      opacity: opacity ?? this.opacity,
      blendMode: blendMode ?? this.blendMode,
      targetLayer: targetLayer ?? this.targetLayer,
    );
  }

  PixelLayer deepCopy() {
    return PixelLayer._fromPixels(
      name: name,
      source: pixels,
      visible: visible,
      opacity: opacity,
      blendMode: blendMode,
      targetLayer: targetLayer,
    );
  }
}

/// Complete canvas state.
class CanvasState {
  CanvasState({
    this.canvasSize = CanvasSize.s16x16,
    List<PixelLayer>? layers,
    this.activeLayerIndex = 0,
    this.activeTool = DrawingTool.pencil,
    this.symmetryMode = SymmetryMode.none,
    this.zoomLevel = 14.0,
    this.showGrid = true,
    this.foregroundColorIndex = 1,
    this.backgroundColorIndex = 0,
  }) : layers = layers ??
            [
              PixelLayer(
                name: 'Base',
                width: canvasSize.width,
                height: canvasSize.height,
              ),
              PixelLayer(
                name: 'Detail',
                width: canvasSize.width,
                height: canvasSize.height,
              ),
            ];

  final CanvasSize canvasSize;
  final List<PixelLayer> layers;
  final int activeLayerIndex;
  final DrawingTool activeTool;
  final SymmetryMode symmetryMode;
  final double zoomLevel;
  final bool showGrid;
  final int foregroundColorIndex;
  final int backgroundColorIndex;

  PixelLayer get activeLayer => layers[activeLayerIndex];

  int get width => canvasSize.width;
  int get height => canvasSize.height;

  CanvasState copyWith({
    CanvasSize? canvasSize,
    List<PixelLayer>? layers,
    int? activeLayerIndex,
    DrawingTool? activeTool,
    SymmetryMode? symmetryMode,
    double? zoomLevel,
    bool? showGrid,
    int? foregroundColorIndex,
    int? backgroundColorIndex,
  }) {
    return CanvasState(
      canvasSize: canvasSize ?? this.canvasSize,
      layers: layers ?? this.layers,
      activeLayerIndex: activeLayerIndex ?? this.activeLayerIndex,
      activeTool: activeTool ?? this.activeTool,
      symmetryMode: symmetryMode ?? this.symmetryMode,
      zoomLevel: zoomLevel ?? this.zoomLevel,
      showGrid: showGrid ?? this.showGrid,
      foregroundColorIndex: foregroundColorIndex ?? this.foregroundColorIndex,
      backgroundColorIndex: backgroundColorIndex ?? this.backgroundColorIndex,
    );
  }
}

/// Snapshot for undo/redo.
@immutable
class CanvasSnapshot {
  const CanvasSnapshot({required this.layerPixels});

  final List<List<Color?>> layerPixels;
}

// ── Tilemap Mode ──────────────────────────────────────────

/// Editor modes — pixel editing or tilemap painting.
enum EditorMode { pixel, tilemap }

/// Tools available in tilemap mode.
enum TilemapTool { stamp, eraser, bucket, eyedropper }

/// A tilemap — 2D grid of tile name references.
class TilemapState {
  TilemapState({
    this.gridWidth = 12,
    this.gridHeight = 8,
    List<List<String?>>? cells,
    this.activeTool = TilemapTool.stamp,
    this.selectedTile,
    this.zoomLevel = 4.0,
    this.showGrid = true,
    this.tilePixelSize = 16,
  }) : cells = cells ??
           List.generate(gridHeight, (_) => List.filled(gridWidth, null));

  final int gridWidth;
  final int gridHeight;
  final List<List<String?>> cells; // cells[row][col] = tile name or null
  final TilemapTool activeTool;
  final String? selectedTile; // tile name used as brush
  final double zoomLevel;
  final bool showGrid;
  final int tilePixelSize; // native tile size in pixels (e.g. 16)

  String? cellAt(int col, int row) {
    if (col < 0 || col >= gridWidth || row < 0 || row >= gridHeight) return null;
    return cells[row][col];
  }

  TilemapState copyWith({
    int? gridWidth,
    int? gridHeight,
    List<List<String?>>? cells,
    TilemapTool? activeTool,
    String? selectedTile,
    bool clearSelectedTile = false,
    double? zoomLevel,
    bool? showGrid,
    int? tilePixelSize,
  }) {
    return TilemapState(
      gridWidth: gridWidth ?? this.gridWidth,
      gridHeight: gridHeight ?? this.gridHeight,
      cells: cells ?? this.cells,
      activeTool: activeTool ?? this.activeTool,
      selectedTile: clearSelectedTile ? null : (selectedTile ?? this.selectedTile),
      zoomLevel: zoomLevel ?? this.zoomLevel,
      showGrid: showGrid ?? this.showGrid,
      tilePixelSize: tilePixelSize ?? this.tilePixelSize,
    );
  }
}

/// Snapshot for tilemap undo/redo.
@immutable
class TilemapSnapshot {
  const TilemapSnapshot({required this.cells});
  final List<List<String?>> cells;
}
