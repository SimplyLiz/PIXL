import 'dart:ui' show Color;

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/palette.dart';

class PaletteNotifier extends StateNotifier<PixlPalette> {
  PaletteNotifier() : super(BuiltInPalettes.darkFantasy);

  void setPalette(PixlPalette palette) {
    state = palette;
  }

  void selectBuiltIn(String name) {
    final match = BuiltInPalettes.all.where((p) => p.name == name);
    if (match.isNotEmpty) {
      state = match.first;
    }
  }

  void editColor(int index, Color color) {
    state = state.withColor(index, color);
  }

  void addColor(Color color) {
    if (state.length >= 64) return;
    state = state.addColor(color);
  }

  void removeColor(int index) {
    if (state.length <= 2) return;
    state = state.removeColorAt(index);
  }
}

final paletteProvider = StateNotifierProvider<PaletteNotifier, PixlPalette>(
  (ref) => PaletteNotifier(),
);
