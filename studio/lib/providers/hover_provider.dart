import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Tracks the currently hovered pixel coordinate for the status bar.
class HoverState {
  const HoverState({this.x, this.y});
  final int? x;
  final int? y;

  bool get hasPosition => x != null && y != null;
  String get label => hasPosition ? '$x, $y' : '--';
}

class HoverNotifier extends StateNotifier<HoverState> {
  HoverNotifier() : super(const HoverState());

  void update(int x, int y) {
    if (state.x != x || state.y != y) {
      state = HoverState(x: x, y: y);
    }
  }

  void clear() {
    if (state.hasPosition) {
      state = const HoverState();
    }
  }
}

final hoverProvider = StateNotifierProvider<HoverNotifier, HoverState>(
  (ref) => HoverNotifier(),
);
