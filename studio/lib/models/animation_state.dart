import 'dart:typed_data';
import 'dart:ui';

/// A single animation frame — stores a snapshot of all layer pixels.
class AnimationFrame {
  AnimationFrame({required this.layerPixels});

  /// Pixel data per layer, same format as [CanvasSnapshot.layerPixels].
  final List<List<Color?>> layerPixels;

  AnimationFrame deepCopy() {
    return AnimationFrame(
      layerPixels: [
        for (final layer in layerPixels) List<Color?>.from(layer),
      ],
    );
  }
}

/// Animation editing state for the pixel editor.
class SpriteAnimationState {
  const SpriteAnimationState({
    this.frames = const [],
    this.currentFrame = 0,
    this.fps = 8,
    this.isPlaying = false,
    this.gifBytes,
  });

  /// All animation frames. Empty = no animation (single-frame tile).
  final List<AnimationFrame> frames;

  /// Current frame index (0-based).
  final int currentFrame;

  /// Frames per second for playback.
  final int fps;

  /// Whether animation is currently playing.
  final bool isPlaying;

  /// Cached GIF bytes for export (from backend, if available).
  final Uint8List? gifBytes;

  /// Number of frames (0 means animation mode not active).
  int get totalFrames => frames.length;

  /// Whether animation mode is active (2+ frames).
  bool get hasMultipleFrames => frames.length > 1;

  SpriteAnimationState copyWith({
    List<AnimationFrame>? frames,
    int? currentFrame,
    int? fps,
    bool? isPlaying,
    Uint8List? gifBytes,
    bool clearGif = false,
  }) {
    return SpriteAnimationState(
      frames: frames ?? this.frames,
      currentFrame: currentFrame ?? this.currentFrame,
      fps: fps ?? this.fps,
      isPlaying: isPlaying ?? this.isPlaying,
      gifBytes: clearGif ? null : (gifBytes ?? this.gifBytes),
    );
  }
}
