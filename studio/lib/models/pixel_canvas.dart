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

/// A single layer of pixel data.
class PixelLayer {
  PixelLayer({
    required this.name,
    required int width,
    required int height,
    this.visible = true,
  }) : pixels = List.filled(width * height, null);

  final String name;
  final List<Color?> pixels;
  bool visible;

  PixelLayer copyWith({String? name, bool? visible}) {
    final copy = PixelLayer(
      name: name ?? this.name,
      width: 0, // unused, we copy pixels directly
      height: 0,
      visible: visible ?? this.visible,
    );
    copy.pixels.clear();
    copy.pixels.addAll(pixels);
    return copy;
  }

  PixelLayer deepCopy() {
    final copy = PixelLayer(name: name, width: 0, height: 0, visible: visible);
    copy.pixels.clear();
    copy.pixels.addAll(pixels);
    return copy;
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
