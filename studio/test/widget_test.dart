import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:pixl_studio/main.dart';

void main() {
  testWidgets('Studio shell renders', (WidgetTester tester) async {
    await tester.pumpWidget(const ProviderScope(child: PixlStudioApp()));
    await tester.pumpAndSettle();

    expect(find.text('PIXL'), findsWidgets);
    expect(find.text(' STUDIO'), findsOneWidget);
  });
}
