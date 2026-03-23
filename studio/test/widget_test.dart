import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:pixl_studio/main.dart';

void main() {
  testWidgets('Studio shell renders', (WidgetTester tester) async {
    // Override HttpClient to prevent real network calls in tests
    HttpOverrides.global = _NoOpHttpOverrides();

    await tester.pumpWidget(const ProviderScope(child: PixlStudioApp()));
    // Don't pumpAndSettle — backend connection is async and will keep pumping
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text('PIXL'), findsWidgets);
    expect(find.text(' STUDIO'), findsOneWidget);

    HttpOverrides.global = null;
  });
}

class _NoOpHttpOverrides extends HttpOverrides {
  @override
  HttpClient createHttpClient(SecurityContext? context) {
    return super.createHttpClient(context);
  }
}
