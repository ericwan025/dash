import 'package:flutter_test/flutter_test.dart';

import 'package:dash_app/main.dart';

void main() {
  testWidgets('dashboard renders its cards', (WidgetTester tester) async {
    await tester.pumpWidget(const DashApp());

    // The app title and the three service cards should be present.
    expect(find.text('dash'), findsOneWidget);
    expect(find.text('MEDIA'), findsOneWidget);
    expect(find.text('NAVIGATION'), findsOneWidget);
    expect(find.text('SETTINGS'), findsOneWidget);

    // Buttons are disabled until connected, so nothing should have thrown.
    expect(find.text('Nothing playing'), findsOneWidget);
  });
}
