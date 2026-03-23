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
}

final paletteProvider = StateNotifierProvider<PaletteNotifier, PixlPalette>(
  (ref) => PaletteNotifier(),
);
