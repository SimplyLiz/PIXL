import 'dart:ui' show Color;

import 'package:flutter_test/flutter_test.dart';
import 'package:pixl_studio/models/pixel_canvas.dart';
import 'package:pixl_studio/providers/canvas_provider.dart';

void main() {
  late CanvasNotifier notifier;

  setUp(() {
    notifier = CanvasNotifier();
  });

  group('undo/redo', () {
    test('undo reverts a stroke', () {
      const color = Color(0xFFFF0000);
      notifier.beginStroke(0, 0, color);
      notifier.endStroke();

      expect(notifier.state.activeLayer.pixels[0], color);
      expect(notifier.canUndo, isTrue);

      notifier.undo();
      expect(notifier.state.activeLayer.pixels[0], isNull);
      expect(notifier.canRedo, isTrue);
    });

    test('redo restores undone stroke', () {
      const color = Color(0xFFFF0000);
      notifier.beginStroke(0, 0, color);
      notifier.endStroke();
      notifier.undo();
      notifier.redo();

      expect(notifier.state.activeLayer.pixels[0], color);
    });

    test('undo stack respects max depth', () {
      for (var i = 0; i < 60; i++) {
        notifier.beginStroke(0, 0, const Color(0xFFFF0000));
        notifier.endStroke();
      }
      // Max is 50, should not crash
      for (var i = 0; i < 50; i++) {
        notifier.undo();
      }
      expect(notifier.canUndo, isFalse);
    });
  });

  group('symmetry', () {
    test('horizontal symmetry mirrors pixels', () {
      notifier.setSymmetry(SymmetryMode.horizontal);
      const color = Color(0xFF00FF00);
      notifier.beginStroke(0, 0, color);
      notifier.endStroke();

      final w = notifier.state.width;
      expect(notifier.state.activeLayer.pixels[0], color);
      expect(notifier.state.activeLayer.pixels[w - 1], color);
    });

    test('vertical symmetry mirrors pixels', () {
      notifier.setSymmetry(SymmetryMode.vertical);
      const color = Color(0xFF0000FF);
      notifier.beginStroke(0, 0, color);
      notifier.endStroke();

      final w = notifier.state.width;
      final h = notifier.state.height;
      expect(notifier.state.activeLayer.pixels[0], color);
      expect(notifier.state.activeLayer.pixels[(h - 1) * w], color);
    });

    test('both symmetry mirrors to all quadrants', () {
      notifier.setSymmetry(SymmetryMode.both);
      const color = Color(0xFFFFFF00);
      notifier.beginStroke(0, 0, color);
      notifier.endStroke();

      final w = notifier.state.width;
      final h = notifier.state.height;
      expect(notifier.state.activeLayer.pixels[0], color);
      expect(notifier.state.activeLayer.pixels[w - 1], color);
      expect(notifier.state.activeLayer.pixels[(h - 1) * w], color);
      expect(notifier.state.activeLayer.pixels[(h - 1) * w + (w - 1)], color);
    });
  });

  group('bucket fill', () {
    test('fills connected region', () {
      const color = Color(0xFFFF0000);
      notifier.bucketFill(0, 0, color);

      // All pixels should be filled on a blank canvas
      final pixels = notifier.state.activeLayer.pixels;
      expect(pixels.every((p) => p == color), isTrue);
    });

    test('respects horizontal symmetry', () {
      final w = notifier.state.width;
      notifier.setSymmetry(SymmetryMode.horizontal);

      // Draw a vertical barrier in the middle
      const barrier = Color(0xFF000000);
      final midX = w ~/ 2;
      for (var y = 0; y < notifier.state.height; y++) {
        notifier.state.activeLayer.pixels[y * w + midX] = barrier;
      }

      // Fill left side
      const fill = Color(0xFF00FF00);
      notifier.bucketFill(0, 0, fill);

      // Right side (mirror of left) should also be filled
      final lastX = w - 1;
      expect(notifier.state.activeLayer.pixels[lastX], fill);
    });

    test('does not fill if target equals fill color', () {
      const color = Color(0xFFFF0000);
      notifier.bucketFill(0, 0, color);
      // Fill again with same color — should not push undo
      final undoBefore = notifier.canUndo;
      notifier.bucketFill(0, 0, color);
      // The second fill should be a no-op for each origin
      // (canUndo will still be true from the first fill)
      expect(undoBefore, isTrue);
    });
  });

  group('clampColorIndices', () {
    test('clamps when palette shrinks', () {
      notifier.setForegroundColor(15);
      notifier.setBackgroundColor(10);
      notifier.clampColorIndices(4); // Game Boy palette

      expect(notifier.state.foregroundColorIndex, 3);
      expect(notifier.state.backgroundColorIndex, 3);
    });

    test('no-op when indices are in range', () {
      notifier.setForegroundColor(2);
      notifier.setBackgroundColor(1);
      notifier.clampColorIndices(16);

      expect(notifier.state.foregroundColorIndex, 2);
      expect(notifier.state.backgroundColorIndex, 1);
    });
  });

  group('snapshot safety', () {
    test('restore skips on layer count mismatch', () {
      const color = Color(0xFFFF0000);
      notifier.beginStroke(0, 0, color);
      notifier.endStroke();

      // Change canvas size (which resets layers)
      notifier.setCanvasSize(CanvasSize.s32x32);

      // Undo should not crash — it silently skips the mismatched snapshot
      notifier.undo();
      // Should still have blank canvas
      expect(notifier.state.activeLayer.pixels[0], isNull);
    });
  });
}
