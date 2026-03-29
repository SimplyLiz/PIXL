import 'dart:ui';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/animation_state.dart';
import '../models/pixel_canvas.dart';

/// Manages sprite animation frames for the pixel editor.
///
/// Coordinates with [CanvasNotifier] to save/load frame pixel data
/// when switching frames. The animation bar watches this provider.
class SpriteAnimationNotifier extends StateNotifier<SpriteAnimationState> {
  SpriteAnimationNotifier() : super(const SpriteAnimationState());

  /// Capture the current canvas pixels into a frame snapshot.
  static AnimationFrame _captureCanvas(CanvasState canvas) {
    return AnimationFrame(
      layerPixels: [
        for (final layer in canvas.layers) List<Color?>.from(layer.pixels),
      ],
    );
  }

  /// Initialize animation mode by capturing the current canvas as frame 1,
  /// then adding a blank frame 2.
  void startAnimation(CanvasState canvas) {
    final frame1 = _captureCanvas(canvas);
    final blankFrame = AnimationFrame(
      layerPixels: [
        for (final layer in canvas.layers)
          List<Color?>.filled(layer.pixels.length, null),
      ],
    );
    state = SpriteAnimationState(
      frames: [frame1, blankFrame],
      currentFrame: 0,
      fps: state.fps,
    );
  }

  /// Add a new frame. If [duplicate], copy current frame pixels.
  /// [canvas] is the current canvas state to capture before adding.
  void addFrame(CanvasState canvas, {bool duplicate = false}) {
    final frames = List<AnimationFrame>.from(state.frames);
    // Update current frame with latest canvas data.
    if (state.currentFrame < frames.length) {
      frames[state.currentFrame] = _captureCanvas(canvas);
    }
    final newFrame = duplicate
        ? _captureCanvas(canvas)
        : AnimationFrame(
            layerPixels: [
              for (final layer in canvas.layers)
                List<Color?>.filled(layer.pixels.length, null),
            ],
          );
    frames.add(newFrame);
    state = state.copyWith(
      frames: frames,
      currentFrame: frames.length - 1,
      clearGif: true,
    );
  }

  /// Remove a frame by index. Cannot remove if only 1 frame left —
  /// instead clears animation mode entirely.
  /// Returns the frame to load onto canvas (the new current frame),
  /// or null if animation mode was cleared.
  AnimationFrame? removeFrame(int index, CanvasState canvas) {
    if (state.frames.length <= 2) {
      // Dropping to 1 frame — exit animation mode, keep current frame.
      final keepIndex = index == 0 ? 1 : 0;
      final keep = keepIndex < state.frames.length
          ? state.frames[keepIndex]
          : _captureCanvas(canvas);
      state = const SpriteAnimationState();
      return keep;
    }
    final frames = List<AnimationFrame>.from(state.frames);
    // Save current canvas into its frame before removing.
    if (state.currentFrame < frames.length && state.currentFrame != index) {
      frames[state.currentFrame] = _captureCanvas(canvas);
    }
    frames.removeAt(index);
    final newCurrent = index >= frames.length ? frames.length - 1 : index;
    state = state.copyWith(
      frames: frames,
      currentFrame: newCurrent,
      clearGif: true,
    );
    return frames[newCurrent];
  }

  /// Switch to a different frame. Saves current canvas pixels into the
  /// old frame and returns the new frame's pixels to load onto canvas.
  AnimationFrame? switchFrame(int targetIndex, CanvasState canvas) {
    if (targetIndex == state.currentFrame) return null;
    if (targetIndex < 0 || targetIndex >= state.frames.length) return null;

    final frames = List<AnimationFrame>.from(state.frames);
    // Save current canvas into old frame.
    frames[state.currentFrame] = _captureCanvas(canvas);

    state = state.copyWith(
      frames: frames,
      currentFrame: targetIndex,
      clearGif: true,
    );
    return frames[targetIndex];
  }

  /// Advance to next frame during playback. Returns the frame to load.
  AnimationFrame? advanceFrame(CanvasState canvas) {
    if (state.frames.length <= 1) return null;
    final next = (state.currentFrame + 1) % state.frames.length;
    return switchFrame(next, canvas);
  }

  /// Seek to a specific frame (0-based).
  void setFrame(int frame) {
    if (state.frames.isEmpty) return;
    state = state.copyWith(
      currentFrame: frame.clamp(0, state.frames.length - 1),
    );
  }

  void setPlaying(bool playing) {
    state = state.copyWith(isPlaying: playing);
  }

  void setFps(int fps) {
    state = state.copyWith(fps: fps.clamp(1, 60));
  }

  /// Save current canvas into active frame (call before tab switch).
  void captureCurrentFrame(CanvasState canvas) {
    if (state.frames.isEmpty) return;
    final frames = List<AnimationFrame>.from(state.frames);
    frames[state.currentFrame] = _captureCanvas(canvas);
    state = state.copyWith(frames: frames);
  }

  void clear() {
    state = const SpriteAnimationState();
  }

  void restore(SpriteAnimationState s) => state = s;
}

final spriteAnimationProvider =
    StateNotifierProvider<SpriteAnimationNotifier, SpriteAnimationState>(
  (ref) => SpriteAnimationNotifier(),
);
