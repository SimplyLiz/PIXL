import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Dithering mode for generation prompts.
enum Dithering { none, bayer, ordered, selective }

/// Outline style for generation prompts.
enum OutlineStyle { none, selfOutline, dropShadow, selective }

/// Mood modifier for generation prompts.
enum Mood { gritty, clean, vibrant, pastel, monochrome }

/// Style state — theme + modifier chips injected into generation prompts.
class StyleState {
  const StyleState({
    this.theme = 'dark_fantasy',
    this.dithering = Dithering.none,
    this.outline = OutlineStyle.selfOutline,
    this.mood = Mood.gritty,
  });

  final String theme;
  final Dithering dithering;
  final OutlineStyle outline;
  final Mood mood;

  /// Build style description for prompt injection.
  String toPromptFragment() {
    final parts = <String>[];
    parts.add('Theme: $theme');

    if (dithering != Dithering.none) {
      parts.add('Dithering: ${dithering.name}');
    }

    final outlineName = switch (outline) {
      OutlineStyle.none => 'none',
      OutlineStyle.selfOutline => 'self-outline',
      OutlineStyle.dropShadow => 'drop-shadow',
      OutlineStyle.selective => 'selective',
    };
    parts.add('Outline: $outlineName');
    parts.add('Mood: ${mood.name}');

    return parts.join('. ');
  }

  StyleState copyWith({
    String? theme,
    Dithering? dithering,
    OutlineStyle? outline,
    Mood? mood,
  }) {
    return StyleState(
      theme: theme ?? this.theme,
      dithering: dithering ?? this.dithering,
      outline: outline ?? this.outline,
      mood: mood ?? this.mood,
    );
  }
}

/// Available themes.
class AvailableThemes {
  static const themes = [
    ('dark_fantasy', 'Dark Fantasy'),
    ('light_fantasy', 'Light Fantasy'),
    ('sci_fi', 'Sci-Fi'),
    ('nature', 'Nature'),
    ('retro_8bit', 'Retro 8-bit'),
    ('gameboy', 'Game Boy'),
  ];
}

class StyleNotifier extends StateNotifier<StyleState> {
  StyleNotifier() : super(const StyleState());

  void setTheme(String theme) => state = state.copyWith(theme: theme);
  void setDithering(Dithering d) => state = state.copyWith(dithering: d);
  void setOutline(OutlineStyle o) => state = state.copyWith(outline: o);
  void setMood(Mood m) => state = state.copyWith(mood: m);

  /// Restore full style state (used by tab manager on tab switch).
  void restore(StyleState s) => state = s;
}

final styleProvider = StateNotifierProvider<StyleNotifier, StyleState>(
  (ref) => StyleNotifier(),
);
