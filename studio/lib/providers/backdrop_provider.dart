import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Zone behavior types matching the 10 Rust ZoneBehavior variants.
const zoneBehaviors = [
  'cycle', 'wave', 'flicker', 'scroll_down',
  'hscroll_sine', 'vscroll_sine', 'color_gradient',
  'palette_ramp', 'mosaic', 'window',
];

/// State for a single animation zone.
class ZoneState {
  final String name;
  final int x, y, w, h;
  final String behavior;
  final Map<String, dynamic> params; // cycle, amplitude, period, speed, etc.

  const ZoneState({
    required this.name,
    required this.x,
    required this.y,
    required this.w,
    required this.h,
    required this.behavior,
    this.params = const {},
  });

  ZoneState copyWith({
    String? name, int? x, int? y, int? w, int? h,
    String? behavior, Map<String, dynamic>? params,
  }) => ZoneState(
    name: name ?? this.name,
    x: x ?? this.x, y: y ?? this.y,
    w: w ?? this.w, h: h ?? this.h,
    behavior: behavior ?? this.behavior,
    params: params ?? this.params,
  );
}

/// State for a single layer.
class LayerState {
  final String name;
  final double scrollFactor;
  final double opacity;
  final String blend;
  final bool visible;

  const LayerState({
    required this.name,
    this.scrollFactor = 1.0,
    this.opacity = 1.0,
    this.blend = 'normal',
    this.visible = true,
  });

  LayerState copyWith({
    String? name, double? scrollFactor, double? opacity,
    String? blend, bool? visible,
  }) => LayerState(
    name: name ?? this.name,
    scrollFactor: scrollFactor ?? this.scrollFactor,
    opacity: opacity ?? this.opacity,
    blend: blend ?? this.blend,
    visible: visible ?? this.visible,
  );
}

/// Full backdrop editor state.
class BackdropEditorState {
  final String? paxPath;
  final String? backdropName;
  final Uint8List? staticPreview;
  final Uint8List? animatedPreview;
  final List<ZoneState> zones;
  final List<LayerState> layers;
  final int? selectedZoneIndex;
  final bool isPlaying;
  final int currentTick;
  final int totalFrames;
  final bool loading;
  final String? error;

  const BackdropEditorState({
    this.paxPath,
    this.backdropName,
    this.staticPreview,
    this.animatedPreview,
    this.zones = const [],
    this.layers = const [],
    this.selectedZoneIndex,
    this.isPlaying = false,
    this.currentTick = 0,
    this.totalFrames = 8,
    this.loading = false,
    this.error,
  });

  BackdropEditorState copyWith({
    String? paxPath, String? backdropName,
    Uint8List? staticPreview, Uint8List? animatedPreview,
    List<ZoneState>? zones, List<LayerState>? layers,
    int? selectedZoneIndex, bool? isPlaying,
    int? currentTick, int? totalFrames,
    bool? loading, String? error,
  }) => BackdropEditorState(
    paxPath: paxPath ?? this.paxPath,
    backdropName: backdropName ?? this.backdropName,
    staticPreview: staticPreview ?? this.staticPreview,
    animatedPreview: animatedPreview ?? this.animatedPreview,
    zones: zones ?? this.zones,
    layers: layers ?? this.layers,
    selectedZoneIndex: selectedZoneIndex,
    isPlaying: isPlaying ?? this.isPlaying,
    currentTick: currentTick ?? this.currentTick,
    totalFrames: totalFrames ?? this.totalFrames,
    loading: loading ?? this.loading,
    error: error,
  );
}

class BackdropNotifier extends StateNotifier<BackdropEditorState> {
  BackdropNotifier() : super(const BackdropEditorState());

  void setPreview(Uint8List bytes) {
    state = state.copyWith(staticPreview: bytes);
  }

  void setAnimatedPreview(Uint8List bytes) {
    state = state.copyWith(animatedPreview: bytes);
  }

  void loadFromResponse(Map<String, dynamic> resp, String paxPath, String name) {
    final zones = <ZoneState>[];
    if (resp['zones'] is List) {
      for (final z in resp['zones'] as List) {
        zones.add(ZoneState(
          name: z['name'] ?? '',
          x: z['rect']?['x'] ?? 0,
          y: z['rect']?['y'] ?? 0,
          w: z['rect']?['w'] ?? 0,
          h: z['rect']?['h'] ?? 0,
          behavior: z['behavior'] ?? 'cycle',
          params: Map<String, dynamic>.from(z)
            ..remove('name')..remove('rect')..remove('behavior'),
        ));
      }
    }

    final layers = <LayerState>[];
    if (resp['layers'] is List) {
      for (final l in resp['layers'] as List) {
        layers.add(LayerState(
          name: l['name'] ?? '',
          scrollFactor: (l['scroll_factor'] as num?)?.toDouble() ?? 1.0,
          opacity: (l['opacity'] as num?)?.toDouble() ?? 1.0,
          blend: l['blend'] ?? 'normal',
        ));
      }
    }

    Uint8List? preview;
    if (resp['png_base64'] is String) {
      preview = base64Decode(resp['png_base64'] as String);
    }

    state = BackdropEditorState(
      paxPath: paxPath,
      backdropName: name,
      staticPreview: preview,
      zones: zones,
      layers: layers,
    );
  }

  void selectZone(int? index) {
    state = state.copyWith(selectedZoneIndex: index);
  }

  void updateZone(int index, ZoneState zone) {
    final zones = List<ZoneState>.from(state.zones);
    zones[index] = zone;
    state = state.copyWith(zones: zones);
  }

  void addZone(ZoneState zone) {
    state = state.copyWith(zones: [...state.zones, zone]);
  }

  void removeZone(int index) {
    final zones = List<ZoneState>.from(state.zones)..removeAt(index);
    state = state.copyWith(zones: zones, selectedZoneIndex: null);
  }

  void updateLayer(int index, LayerState layer) {
    final layers = List<LayerState>.from(state.layers);
    layers[index] = layer;
    state = state.copyWith(layers: layers);
  }

  void setPlaying(bool playing) {
    state = state.copyWith(isPlaying: playing);
  }

  void setTick(int tick) {
    state = state.copyWith(currentTick: tick);
  }

  void setLoading(bool loading) {
    state = state.copyWith(loading: loading);
  }

  void setError(String? error) {
    state = state.copyWith(error: error);
  }

  void clear() {
    state = const BackdropEditorState();
  }

  void restore(BackdropEditorState s) => state = s;
}

final backdropEditorProvider =
    StateNotifierProvider<BackdropNotifier, BackdropEditorState>(
  (ref) => BackdropNotifier(),
);
