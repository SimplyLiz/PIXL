import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Info about a single composite sprite.
class CompositeInfo {
  final String name;
  final String size;
  final String tileSize;
  final List<String> variants;
  final List<String> animations;

  const CompositeInfo({
    required this.name,
    required this.size,
    required this.tileSize,
    this.variants = const [],
    this.animations = const [],
  });
}

/// State for the composite editor.
class CompositeEditorState {
  final List<CompositeInfo> composites;
  final String? selectedName;
  final String? selectedVariant;
  final String? selectedAnim;
  final int? selectedFrame;
  final Uint8List? preview;
  final List<Map<String, dynamic>> seamWarnings;
  final bool loading;
  final String? error;

  const CompositeEditorState({
    this.composites = const [],
    this.selectedName,
    this.selectedVariant,
    this.selectedAnim,
    this.selectedFrame,
    this.preview,
    this.seamWarnings = const [],
    this.loading = false,
    this.error,
  });

  CompositeEditorState copyWith({
    List<CompositeInfo>? composites,
    String? selectedName,
    String? selectedVariant,
    String? selectedAnim,
    int? selectedFrame,
    Uint8List? preview,
    List<Map<String, dynamic>>? seamWarnings,
    bool? loading,
    String? error,
  }) =>
      CompositeEditorState(
        composites: composites ?? this.composites,
        selectedName: selectedName ?? this.selectedName,
        selectedVariant: selectedVariant,
        selectedAnim: selectedAnim,
        selectedFrame: selectedFrame,
        preview: preview ?? this.preview,
        seamWarnings: seamWarnings ?? this.seamWarnings,
        loading: loading ?? this.loading,
        error: error,
      );

  CompositeInfo? get selected =>
      selectedName == null
          ? null
          : composites.cast<CompositeInfo?>().firstWhere(
              (c) => c?.name == selectedName,
              orElse: () => null,
            );
}

class CompositeNotifier extends StateNotifier<CompositeEditorState> {
  CompositeNotifier() : super(const CompositeEditorState());

  void setComposites(List<CompositeInfo> composites) {
    state = state.copyWith(composites: composites);
  }

  void select(String name) {
    state = CompositeEditorState(
      composites: state.composites,
      selectedName: name,
      seamWarnings: state.seamWarnings,
    );
  }

  void selectVariant(String? variant) {
    state = state.copyWith(selectedVariant: variant);
  }

  void selectAnim(String? anim) {
    state = state.copyWith(selectedAnim: anim, selectedFrame: anim != null ? 1 : null);
  }

  void selectFrame(int? frame) {
    state = state.copyWith(selectedFrame: frame);
  }

  void setPreview(Uint8List bytes) {
    state = state.copyWith(preview: bytes);
  }

  void setSeamWarnings(List<Map<String, dynamic>> warnings) {
    state = state.copyWith(seamWarnings: warnings);
  }

  void setLoading(bool loading) {
    state = state.copyWith(loading: loading);
  }

  void setError(String? error) {
    state = state.copyWith(error: error);
  }

  void clear() {
    state = const CompositeEditorState();
  }
}

final compositeEditorProvider =
    StateNotifierProvider<CompositeNotifier, CompositeEditorState>(
  (ref) => CompositeNotifier(),
);
