import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Adapter metadata from the engine.
@immutable
class AdapterInfo {
  const AdapterInfo({
    required this.name,
    required this.path,
    this.model,
    this.trainSamples,
    this.epochs,
    this.created,
  });

  final String name;
  final String path;
  final String? model;
  final int? trainSamples;
  final int? epochs;
  final String? created;

  factory AdapterInfo.fromJson(Map<String, dynamic> json) => AdapterInfo(
        name: json['name'] as String? ?? '',
        path: json['path'] as String? ?? '',
        model: json['model'] as String?,
        trainSamples: json['train_samples'] as int?,
        epochs: json['epochs'] as int?,
        created: json['created'] as String?,
      );
}

/// Scan result summary.
@immutable
class ScanSummary {
  const ScanSummary({
    required this.totalRaw,
    required this.totalQuality,
    required this.totalFiltered,
    required this.categories,
    this.scanDir,
  });

  final int totalRaw;
  final int totalQuality;
  final int totalFiltered;
  final Map<String, int> categories;
  final String? scanDir;

  factory ScanSummary.fromJson(Map<String, dynamic> json) {
    final cats = <String, int>{};
    if (json['categories'] is Map) {
      for (final e in (json['categories'] as Map).entries) {
        cats[e.key.toString()] = (e.value as num).toInt();
      }
    }
    return ScanSummary(
      totalRaw: (json['total_patches_raw'] as num?)?.toInt() ?? 0,
      totalQuality: (json['total_patches_quality'] as num?)?.toInt() ?? 0,
      totalFiltered: (json['total_filtered'] as num?)?.toInt() ?? 0,
      categories: cats,
      scanDir: json['scan_dir'] as String?,
    );
  }
}

/// Training data prep result.
@immutable
class PrepareResult {
  const PrepareResult({
    required this.trainCount,
    required this.validCount,
    required this.testCount,
    required this.totalAugmented,
    required this.totalStratified,
    this.dataDir,
  });

  final int trainCount;
  final int validCount;
  final int testCount;
  final int totalAugmented;
  final int totalStratified;
  final String? dataDir;

  /// Estimated training time in minutes (M4 Pro, ~2 it/sec, 3 epochs).
  int get estMinutes3ep => (trainCount * 3) ~/ 2 ~/ 60;
  int get estMinutes5ep => (trainCount * 5) ~/ 2 ~/ 60;

  factory PrepareResult.fromJson(Map<String, dynamic> json) => PrepareResult(
        trainCount: (json['train_count'] as num?)?.toInt() ?? 0,
        validCount: (json['valid_count'] as num?)?.toInt() ?? 0,
        testCount: (json['test_count'] as num?)?.toInt() ?? 0,
        totalAugmented: (json['total_augmented'] as num?)?.toInt() ?? 0,
        totalStratified: (json['total_stratified'] as num?)?.toInt() ?? 0,
        dataDir: json['data_dir'] as String?,
      );
}

/// Scanner pipeline phase.
enum ScannerPhase { idle, scanning, scanned, preparing, prepared, training, trained, error }

/// Full scanner state.
@immutable
class ScannerState {
  const ScannerState({
    this.phase = ScannerPhase.idle,
    this.inputPath,
    this.styleName = 'custom',
    this.scanSummary,
    this.prepareResult,
    this.adapterPath,
    this.adapters = const [],
    this.activeAdapter,
    this.error,
    this.trainingProgress = 0.0,
    this.trainingLoss,
    this.lossHistory = const [],
    this.datasetDirs = const ['training'],
    this.trainingSpeed,
    this.trainingBestLoss,
    this.trainingEpoch = 0,
    this.totalEpochs = 0,
    this.trainingPaused = false,
  });

  final ScannerPhase phase;
  final String? inputPath;
  final String styleName;
  final ScanSummary? scanSummary;
  final PrepareResult? prepareResult;
  final String? adapterPath;
  final List<AdapterInfo> adapters;
  final String? activeAdapter;
  final String? error;
  final double trainingProgress;
  final double? trainingLoss;
  final List<double> lossHistory;
  /// Directories to scan for training datasets (persisted).
  final List<String> datasetDirs;
  final double? trainingSpeed;
  final double? trainingBestLoss;
  final int trainingEpoch;
  final int totalEpochs;
  final bool trainingPaused;

  ScannerState copyWith({
    ScannerPhase? phase,
    String? inputPath,
    String? styleName,
    ScanSummary? scanSummary,
    PrepareResult? prepareResult,
    String? adapterPath,
    List<AdapterInfo>? adapters,
    String? activeAdapter,
    bool clearActiveAdapter = false,
    String? error,
    bool clearError = false,
    double? trainingProgress,
    double? trainingLoss,
    List<double>? lossHistory,
    List<String>? datasetDirs,
    double? trainingSpeed,
    double? trainingBestLoss,
    int? trainingEpoch,
    int? totalEpochs,
    bool? trainingPaused,
  }) {
    return ScannerState(
      phase: phase ?? this.phase,
      inputPath: inputPath ?? this.inputPath,
      styleName: styleName ?? this.styleName,
      scanSummary: scanSummary ?? this.scanSummary,
      prepareResult: prepareResult ?? this.prepareResult,
      adapterPath: adapterPath ?? this.adapterPath,
      adapters: adapters ?? this.adapters,
      activeAdapter: clearActiveAdapter ? null : (activeAdapter ?? this.activeAdapter),
      error: clearError ? null : (error ?? this.error),
      trainingProgress: trainingProgress ?? this.trainingProgress,
      trainingLoss: trainingLoss ?? this.trainingLoss,
      lossHistory: lossHistory ?? this.lossHistory,
      datasetDirs: datasetDirs ?? this.datasetDirs,
      trainingSpeed: trainingSpeed ?? this.trainingSpeed,
      trainingBestLoss: trainingBestLoss ?? this.trainingBestLoss,
      trainingEpoch: trainingEpoch ?? this.trainingEpoch,
      totalEpochs: totalEpochs ?? this.totalEpochs,
      trainingPaused: trainingPaused ?? this.trainingPaused,
    );
  }
}

/// Scanner state notifier — drives the scan → prepare → train pipeline.
class ScannerNotifier extends StateNotifier<ScannerState> {
  ScannerNotifier() : super(const ScannerState()) {
    _loadDatasetDirs();
  }

  static const _prefsKey = 'scanner_dataset_dirs';

  Future<void> _loadDatasetDirs() async {
    final prefs = await SharedPreferences.getInstance();
    final dirs = prefs.getStringList(_prefsKey);
    if (dirs != null && dirs.isNotEmpty) {
      state = state.copyWith(datasetDirs: dirs);
    }
  }

  Future<void> _saveDatasetDirs() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setStringList(_prefsKey, state.datasetDirs);
  }

  void addDatasetDir(String dir) {
    if (state.datasetDirs.contains(dir)) return;
    state = state.copyWith(datasetDirs: [...state.datasetDirs, dir]);
    _saveDatasetDirs();
  }

  void removeDatasetDir(String dir) {
    state = state.copyWith(
      datasetDirs: state.datasetDirs.where((d) => d != dir).toList(),
    );
    _saveDatasetDirs();
  }

  void updateDatasetDir(String oldDir, String newDir) {
    state = state.copyWith(
      datasetDirs: state.datasetDirs.map((d) => d == oldDir ? newDir : d).toList(),
    );
    _saveDatasetDirs();
  }

  void setInputPath(String path) {
    state = state.copyWith(inputPath: path, phase: ScannerPhase.idle, clearError: true);
  }

  void setStyleName(String name) {
    state = state.copyWith(styleName: name);
  }

  void setScanResult(ScanSummary summary) {
    state = state.copyWith(phase: ScannerPhase.scanned, scanSummary: summary, clearError: true);
  }

  void setPrepareResult(PrepareResult result) {
    state = state.copyWith(phase: ScannerPhase.prepared, prepareResult: result, clearError: true);
  }

  void setTraining() {
    state = state.copyWith(phase: ScannerPhase.training, trainingProgress: 0.0);
  }

  void updateTrainingProgress(
    double progress,
    double? loss, {
    double? speed,
    double? bestLoss,
    int? epoch,
    int? totalEpochs,
    bool? paused,
  }) {
    final history = loss != null ? [...state.lossHistory, loss] : state.lossHistory;
    state = state.copyWith(
      trainingProgress: progress,
      trainingLoss: loss,
      lossHistory: history,
      trainingSpeed: speed,
      trainingBestLoss: bestLoss,
      trainingEpoch: epoch,
      totalEpochs: totalEpochs,
      trainingPaused: paused,
    );
  }

  void setTrained(String adapterPath) {
    state = state.copyWith(
      phase: ScannerPhase.trained,
      adapterPath: adapterPath,
      trainingProgress: 1.0,
    );
  }

  void setError(String error) {
    state = state.copyWith(phase: ScannerPhase.error, error: error);
  }

  void setPhase(ScannerPhase phase) {
    state = state.copyWith(phase: phase);
  }

  void setAdapters(List<AdapterInfo> adapters) {
    state = state.copyWith(adapters: adapters);
  }

  void setActiveAdapter(String? path) {
    if (path == null) {
      state = state.copyWith(clearActiveAdapter: true);
    } else {
      state = state.copyWith(activeAdapter: path);
    }
  }

  void reset() {
    state = const ScannerState();
  }
}

final scannerProvider = StateNotifierProvider<ScannerNotifier, ScannerState>(
  (ref) => ScannerNotifier(),
);
