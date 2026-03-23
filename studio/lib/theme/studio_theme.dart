import 'package:flutter/material.dart';

/// PIXL Studio dark theme — clean, minimal, focused on the canvas.
class StudioTheme {
  static const _bg = Color(0xFF1a1a2e);
  static const _surface = Color(0xFF222240);
  static const _surfaceLight = Color(0xFF2a2a4e);
  static const _border = Color(0xFF3a3a5e);
  static const _text = Color(0xFFe0e0ee);
  static const _textDim = Color(0xFF8888aa);
  static const _accent = Color(0xFF7c6ff0);
  static const _accentLight = Color(0xFF9d93f5);

  static ThemeData get theme => ThemeData(
        brightness: Brightness.dark,
        scaffoldBackgroundColor: _bg,
        colorScheme: const ColorScheme.dark(
          primary: _accent,
          secondary: _accentLight,
          surface: _surface,
          onSurface: _text,
          outline: _border,
        ),
        cardColor: _surface,
        dividerColor: _border,
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
          thumbColor: WidgetStateProperty.all(_border),
          radius: const Radius.circular(2),
        ),
      );

  // Panel styling constants
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
}
