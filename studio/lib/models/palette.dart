import 'dart:ui' show Color;

/// A named color palette for the editor.
class PixlPalette {
  const PixlPalette({
    required this.name,
    required this.colors,
    this.engineId,
    this.enginePalette,
  });

  final String name;
  final List<Color> colors;
  /// Engine-side theme identifier used for newFromTemplate (e.g. 'dark_fantasy').
  final String? engineId;
  /// Engine-side palette key used in tile creation (e.g. 'dungeon').
  /// This is the [palette.*] section name inside the theme's .pax file.
  final String? enginePalette;

  /// Build a palette from the engine's loadSource response.
  ///
  /// [resp] should contain `active_theme`, `active_palette`, and
  /// `palette_colors` (list of "#rrggbbaa" hex strings).
  static PixlPalette? fromEngineResponse(Map<String, dynamic> resp) {
    final theme = resp['active_theme'] as String?;
    final palette = resp['active_palette'] as String?;
    final colorList = resp['palette_colors'] as List<dynamic>?;
    if (theme == null || palette == null || colorList == null) return null;

    final colors = <Color>[];
    for (final hex in colorList) {
      final h = (hex as String).replaceFirst('#', '');
      if (h.length == 8) {
        // #rrggbbaa → Color(0xAARRGGBB)
        final r = int.parse(h.substring(0, 2), radix: 16);
        final g = int.parse(h.substring(2, 4), radix: 16);
        final b = int.parse(h.substring(4, 6), radix: 16);
        final a = int.parse(h.substring(6, 8), radix: 16);
        colors.add(Color.fromARGB(a, r, g, b));
      } else if (h.length == 6) {
        colors.add(Color(int.parse('ff$h', radix: 16)));
      }
    }

    // Use the display name from BuiltInPalettes if available, else humanize.
    final builtIn = BuiltInPalettes.all.where((p) => p.engineId == theme);
    final displayName = builtIn.isNotEmpty
        ? builtIn.first.name
        : theme.replaceAll('_', ' ').replaceAllMapped(
            RegExp(r'\b\w'),
            (m) => m[0]!.toUpperCase(),
          );

    return PixlPalette(
      name: displayName,
      colors: colors,
      engineId: theme,
      enginePalette: palette,
    );
  }

  int get length => colors.length;
  Color operator [](int index) => colors[index];

  PixlPalette withColor(int index, Color color) {
    final newColors = List<Color>.from(colors);
    newColors[index] = color;
    return PixlPalette(name: name, colors: newColors);
  }

  PixlPalette addColor(Color color) {
    return PixlPalette(name: name, colors: [...colors, color]);
  }

  PixlPalette removeColorAt(int index) {
    final newColors = List<Color>.from(colors)..removeAt(index);
    return PixlPalette(name: name, colors: newColors);
  }
}

/// Built-in theme palettes matching the PIXL themes.
class BuiltInPalettes {
  static const darkFantasy = PixlPalette(
    name: 'Dark Fantasy',
    engineId: 'dark_fantasy',
    enginePalette: 'dungeon',
    colors: [
      Color(0x00000000), // transparent
      Color(0xFF0f0b14), // void black
      Color(0xFF2a1f3d), // dark stone
      Color(0xFF4a3a6d), // lit stone
      Color(0xFF6b5a9e), // highlight
      Color(0xFF2d5a27), // moss green
      Color(0xFF4a8c3f), // moss light
      Color(0xFF8b6914), // gold dim
      Color(0xFFc9a030), // gold bright
      Color(0xFF5c2020), // blood dark
      Color(0xFF8b3030), // blood
      Color(0xFF3a5a8c), // water dark
      Color(0xFF5a8abd), // water light
      Color(0xFF1a1a2e), // shadow
      Color(0xFF7a6a9e), // stone light
      Color(0xFFddc870), // gold highlight
    ],
  );

  static const sciFi = PixlPalette(
    name: 'Sci-Fi',
    engineId: 'sci_fi',
    enginePalette: 'cyber',
    colors: [
      Color(0x00000000), // transparent
      Color(0xFF0a0a12), // deep black
      Color(0xFF1a1a2e), // dark panel
      Color(0xFF2a2a4e), // panel mid
      Color(0xFF00ff88), // neon green
      Color(0xFF00aaff), // neon blue
      Color(0xFFff0066), // neon pink
      Color(0xFF3a3a5e), // metal light
      Color(0xFFccccdd), // white metal
      Color(0xFF444466), // grate
      Color(0xFFff8800), // warning orange
      Color(0xFF660033), // danger dark
    ],
  );

  static const nature = PixlPalette(
    name: 'Nature',
    engineId: 'nature',
    enginePalette: 'forest',
    colors: [
      Color(0x00000000), // transparent
      Color(0xFF2d5a27), // dark green
      Color(0xFF4a8c3f), // green
      Color(0xFF6ab84f), // light green
      Color(0xFF8b6914), // earth dark
      Color(0xFFb8923a), // earth mid
      Color(0xFFddc870), // sand
      Color(0xFF3a5a8c), // water
      Color(0xFF5a8abd), // water light
      Color(0xFF8bcaff), // sky
      Color(0xFF5c3a1a), // bark dark
      Color(0xFF8c6a3a), // bark light
      Color(0xFFcc4444), // berry
      Color(0xFFeeeecc), // flower white
    ],
  );

  static const retro8bit = PixlPalette(
    name: 'Retro 8-bit',
    engineId: 'nes',
    enginePalette: 'nes_palette',
    colors: [
      Color(0x00000000), // transparent
      Color(0xFF000000), // black
      Color(0xFF555555), // dark gray
      Color(0xFFaaaaaa), // light gray
      Color(0xFFffffff), // white
      Color(0xFFff0000), // red
      Color(0xFF00ff00), // green
      Color(0xFF0000ff), // blue
      Color(0xFFffff00), // yellow
      Color(0xFFff8800), // orange
      Color(0xFF8800ff), // purple
      Color(0xFF00ffff), // cyan
      Color(0xFFff00ff), // magenta
      Color(0xFF884400), // brown
      Color(0xFF88ff88), // light green
      Color(0xFF8888ff), // light blue
    ],
  );

  static const gameboy = PixlPalette(
    name: 'Game Boy',
    engineId: 'gameboy',
    enginePalette: 'gameboy_palette',
    colors: [
      Color(0xFF0f380f), // darkest
      Color(0xFF306230), // dark
      Color(0xFF8bac0f), // light
      Color(0xFF9bbc0f), // lightest
    ],
  );

  static const List<PixlPalette> all = [
    darkFantasy,
    sciFi,
    nature,
    retro8bit,
    gameboy,
  ];
}
