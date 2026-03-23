import 'package:flutter/material.dart';

/// PIXL Studio theme — warm neutral grays inspired by Aseprite and
/// professional creative tools. Warm tones reduce eye strain during
/// long pixel art sessions and let the canvas colors be the focus.
class StudioTheme {
  // ── Core palette ──────────────────────────────────────────
  // Warm grays (slight brown/amber undertone, not blue/purple)
  static const _bg = Color(0xFF1e1e1e);          // app background
  static const _surface = Color(0xFF2b2b2b);      // panels, cards
  static const _surfaceLight = Color(0xFF353535);  // elevated surfaces, hover
  static const _border = Color(0xFF444444);        // dividers, borders
  static const _borderLight = Color(0xFF555555);   // lighter borders

  // Text
  static const _text = Color(0xFFe8e8e8);          // primary text
  static const _textDim = Color(0xFF999999);        // secondary, labels
  static const _textMuted = Color(0xFF666666);      // hints, disabled

  // Accent — warm orange-amber (Aseprite-inspired)
  static const _accent = Color(0xFFe8a045);         // primary accent
  static const _accentLight = Color(0xFFf0b860);    // hover/light accent
  // Canvas background (neutral dark, no color cast)
  static const canvasBg = Color(0xFF181818);

  // Semantic colors
  static const success = Color(0xFF6abf69);
  static const error = Color(0xFFe05555);
  static const warning = Color(0xFFe8a045);

  // Code/mono background
  static const codeBg = Color(0xFF252525);

  // ── ThemeData ─────────────────────────────────────────────

  static ThemeData get theme => ThemeData(
        brightness: Brightness.dark,
        scaffoldBackgroundColor: _bg,
        colorScheme: const ColorScheme.dark(
          primary: _accent,
          secondary: _accentLight,
          surface: _surface,
          onSurface: _text,
          outline: _border,
          error: error,
        ),
        cardColor: _surface,
        dividerColor: _border,
        disabledColor: _textMuted,
        textTheme: const TextTheme(
          bodyMedium: TextStyle(
            fontFamily: 'JetBrainsMono',
            fontSize: 13,
            color: _text,
          ),
          bodySmall: TextStyle(
            fontFamily: 'JetBrainsMono',
            fontSize: 11,
            color: _textDim,
          ),
          titleSmall: TextStyle(
            fontFamily: 'JetBrainsMono',
            fontSize: 12,
            fontWeight: FontWeight.w700,
            color: _textDim,
            letterSpacing: 1.2,
          ),
        ),
        iconTheme: const IconThemeData(color: _textDim, size: 18),
        tooltipTheme: TooltipThemeData(
          decoration: BoxDecoration(
            color: _surfaceLight,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: _border),
          ),
          textStyle: const TextStyle(
            fontFamily: 'JetBrainsMono',
            fontSize: 11,
            color: _text,
          ),
        ),
        scrollbarTheme: ScrollbarThemeData(
          thumbColor: WidgetStateProperty.all(_borderLight),
          radius: const Radius.circular(2),
        ),
        snackBarTheme: SnackBarThemeData(
          backgroundColor: _surfaceLight,
          contentTextStyle: const TextStyle(
            fontFamily: 'JetBrainsMono',
            fontSize: 12,
            color: _text,
          ),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(4),
          ),
          behavior: SnackBarBehavior.floating,
        ),
        elevatedButtonTheme: ElevatedButtonThemeData(
          style: ElevatedButton.styleFrom(
            backgroundColor: _accent,
            foregroundColor: _bg,
            textStyle: const TextStyle(
              fontFamily: 'JetBrainsMono',
              fontSize: 12,
              fontWeight: FontWeight.w700,
            ),
          ),
        ),
        textButtonTheme: TextButtonThemeData(
          style: TextButton.styleFrom(
            foregroundColor: _textDim,
            textStyle: const TextStyle(
              fontFamily: 'JetBrainsMono',
              fontSize: 12,
            ),
          ),
        ),
        popupMenuTheme: PopupMenuThemeData(
          color: _surfaceLight,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(4),
            side: const BorderSide(color: _border),
          ),
          textStyle: const TextStyle(
            fontFamily: 'JetBrainsMono',
            fontSize: 12,
            color: _text,
          ),
        ),
        dialogTheme: DialogThemeData(
          backgroundColor: _surface,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
            side: const BorderSide(color: _border),
          ),
        ),
      );

  // ── Panel styling ─────────────────────────────────────────

  static const panelPadding = EdgeInsets.all(8.0);
  static const sectionSpacing = 12.0;
  static const panelBorder = BorderSide(color: _border, width: 1);

  static BoxDecoration get panelDecoration => const BoxDecoration(
        color: _surface,
        border: Border(right: panelBorder),
      );

  static BoxDecoration get rightPanelDecoration => const BoxDecoration(
        color: _surface,
        border: Border(left: panelBorder),
      );

  static BoxDecoration get statusBarDecoration => const BoxDecoration(
        color: _surface,
        border: Border(top: panelBorder),
      );

  static BoxDecoration get topBarDecoration => const BoxDecoration(
        color: _surface,
        border: Border(bottom: panelBorder),
      );

  // ── Reusable inline colors ────────────────────────────────
  // Use these instead of hardcoding hex values in widgets.

  /// Recessed area (canvas bg, code blocks, preview boxes)
  static const recessedBg = Color(0xFF1a1a1a);

  /// Separator text color
  static const separatorColor = _border;

  /// Section highlight background (selected items, hover)
  static const highlightBg = Color(0xFF383838);

  /// Chip/tag background for active state
  static Color accentBg([double opacity = 0.2]) =>
      _accent.withValues(alpha: opacity);
}
