import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'theme/studio_theme.dart';
import 'widgets/studio_shell.dart';

void main() {
  runApp(const ProviderScope(child: PixlStudioApp()));
}

class PixlStudioApp extends StatelessWidget {
  const PixlStudioApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'PIXL Studio',
      debugShowCheckedModeBanner: false,
      theme: StudioTheme.theme,
      home: const StudioShell(),
    );
  }
}
