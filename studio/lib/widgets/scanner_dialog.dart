import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/scanner_provider.dart';
import '../theme/studio_theme.dart';

/// Multi-step wizard for the Style Scanner pipeline.
/// Drop reference images → preview scan → configure training → train → done.
class ScannerDialog extends ConsumerStatefulWidget {
  const ScannerDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      barrierDismissible: false,
      builder: (_) => const ScannerDialog(),
    );
  }

  @override
  ConsumerState<ScannerDialog> createState() => _ScannerDialogState();
}

class _ScannerDialogState extends ConsumerState<ScannerDialog> {
  final _styleController = TextEditingController(text: 'custom');
  int _stride = 8;
  int _epochs = 5;
  bool _colorAug = true;
  List<_DatasetEntry> _availableDatasets = [];
  final Set<String> _selectedDatasets = {};
  bool _showDetails = false;
  bool _isPaused = false;
  String _throttleLevel = 'normal';

  @override
  void dispose() {
    _styleController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scanner = ref.watch(scannerProvider);

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 560,
        constraints: const BoxConstraints(maxHeight: 600),
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _buildHeader(theme, scanner),
            const SizedBox(height: 20),
            Flexible(
              child: SingleChildScrollView(
                child: _buildContent(theme, scanner),
              ),
            ),
            const SizedBox(height: 16),
            _buildActions(theme, scanner),
          ],
        ),
      ),
    );
  }

  Widget _buildHeader(ThemeData theme, ScannerState scanner) {
    final (icon, title) = switch (scanner.phase) {
      ScannerPhase.idle => (Icons.document_scanner, 'Style Scanner'),
      ScannerPhase.scanning => (Icons.search, 'Scanning...'),
      ScannerPhase.scanned => (Icons.checklist, 'Scan Complete'),
      ScannerPhase.preparing => (Icons.auto_fix_high, 'Preparing...'),
      ScannerPhase.prepared => (Icons.dataset, 'Data Ready'),
      ScannerPhase.training => (Icons.model_training, 'Training...'),
      ScannerPhase.trained => (Icons.check_circle, 'Training Complete'),
      ScannerPhase.error => (Icons.error_outline, 'Error'),
    };

    return Row(
      children: [
        Container(
          padding: const EdgeInsets.all(8),
          decoration: BoxDecoration(
            color: theme.colorScheme.primary.withValues(alpha: 0.15),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Icon(icon, size: 20, color: theme.colorScheme.primary),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: theme.textTheme.bodyMedium!.copyWith(
                fontSize: 16, fontWeight: FontWeight.w700,
              )),
              Text(
                _phaseDescription(scanner.phase),
                style: theme.textTheme.bodySmall!.copyWith(
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.6),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  String _phaseDescription(ScannerPhase phase) => switch (phase) {
    ScannerPhase.idle => 'Import reference art to train a style model',
    ScannerPhase.scanning => 'Extracting tiles from your images...',
    ScannerPhase.scanned => 'Review the extracted patches',
    ScannerPhase.preparing => 'Building training dataset...',
    ScannerPhase.prepared => 'Configure and start training',
    ScannerPhase.training => 'Fine-tuning LoRA adapter on your art',
    ScannerPhase.trained => 'Your style adapter is ready',
    ScannerPhase.error => 'Something went wrong',
  };

  Widget _buildContent(ThemeData theme, ScannerState scanner) {
    return switch (scanner.phase) {
      ScannerPhase.idle => _buildDropZone(theme),
      ScannerPhase.scanning => _buildProgress(theme, 'Scanning images...'),
      ScannerPhase.scanned => _buildScanResult(theme, scanner),
      ScannerPhase.preparing => _buildProgress(theme, 'Preparing training data...'),
      ScannerPhase.prepared => _buildPrepareResult(theme, scanner),
      ScannerPhase.training => _buildTrainingProgress(theme, scanner),
      ScannerPhase.trained => _buildTrainedResult(theme, scanner),
      ScannerPhase.error => _buildError(theme, scanner),
    };
  }

  // ── Step 1: Drop zone ─────────────────────────────────

  Widget _buildDropZone(ThemeData theme) {
    return Column(
      children: [
        InkWell(
          onTap: _pickFolder,
          borderRadius: BorderRadius.circular(12),
          child: Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(vertical: 48),
            decoration: BoxDecoration(
              border: Border.all(
                color: theme.colorScheme.primary.withValues(alpha: 0.3),
                width: 2,
              ),
              borderRadius: BorderRadius.circular(12),
              color: theme.colorScheme.primary.withValues(alpha: 0.05),
            ),
            child: Column(
              children: [
                Icon(Icons.folder_open, size: 48,
                  color: theme.colorScheme.primary.withValues(alpha: 0.6)),
                const SizedBox(height: 12),
                Text('Select folder with reference images',
                  style: theme.textTheme.bodyMedium),
                const SizedBox(height: 4),
                Text('PNG, JPG, BMP, GIF, WebP — sprite sheets or individual tiles',
                  style: theme.textTheme.bodySmall!.copyWith(
                    color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
                  )),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        // Style name
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _styleController,
                decoration: InputDecoration(
                  labelText: 'Style name',
                  hintText: 'e.g. my-game, retro-rpg',
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
                  isDense: true,
                ),
                style: theme.textTheme.bodySmall,
              ),
            ),
            const SizedBox(width: 12),
            // Stride selector
            DropdownButton<int>(
              value: _stride,
              isDense: true,
              items: const [
                DropdownMenuItem(value: 16, child: Text('Stride 16 (fast)')),
                DropdownMenuItem(value: 8, child: Text('Stride 8 (more data)')),
                DropdownMenuItem(value: 4, child: Text('Stride 4 (max data)')),
              ],
              onChanged: (v) => setState(() => _stride = v!),
            ),
          ],
        ),
      ],
    );
  }

  Future<void> _pickFolder() async {
    final result = await FilePicker.platform.getDirectoryPath(
      dialogTitle: 'Select reference images folder',
    );
    if (result == null) return;

    ref.read(scannerProvider.notifier).setInputPath(result);
    ref.read(scannerProvider.notifier).setStyleName(_styleController.text);
    _runScan(result);
  }

  Future<void> _runScan(String inputPath) async {
    final notifier = ref.read(scannerProvider.notifier);
    notifier.setPhase(ScannerPhase.scanning);

    final backend = ref.read(backendProvider.notifier).backend;
    final resp = await backend.scanReference(
      inputPath: inputPath,
      stride: _stride,
    );

    if (resp.containsKey('error')) {
      notifier.setError(resp['error'] as String);
    } else {
      notifier.setScanResult(ScanSummary.fromJson(resp));
    }
  }

  // ── Step 2: Scan results ──────────────────────────────

  Widget _buildScanResult(ThemeData theme, ScannerState scanner) {
    final s = scanner.scanSummary!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _statRow(theme, 'Total patches', '${s.totalRaw}'),
        _statRow(theme, 'Quality passed', '${s.totalQuality}'),
        _statRow(theme, 'Filtered out', '${s.totalFiltered} (${s.totalRaw > 0 ? (s.totalFiltered * 100 ~/ s.totalRaw) : 0}%)'),
        const SizedBox(height: 12),
        Text('Categories', style: theme.textTheme.bodySmall!.copyWith(fontWeight: FontWeight.w600)),
        const SizedBox(height: 4),
        ...s.categories.entries.map((e) => Padding(
          padding: const EdgeInsets.only(left: 8, bottom: 2),
          child: Row(
            children: [
              Container(
                width: 8, height: 8,
                decoration: BoxDecoration(
                  color: _categoryColor(e.key),
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(width: 8),
              Text('${e.key}: ${e.value}', style: theme.textTheme.bodySmall),
            ],
          ),
        )),
        const SizedBox(height: 16),
        // Augmentation options
        Row(
          children: [
            Checkbox(
              value: _colorAug,
              onChanged: (v) => setState(() => _colorAug = v ?? true),
            ),
            Text('Color augmentation (warm/cool/dark shifts)', style: theme.textTheme.bodySmall),
          ],
        ),
      ],
    );
  }

  Color _categoryColor(String cat) => switch (cat) {
    'wall' => Colors.brown,
    'floor' => Colors.grey,
    'enemy' => Colors.red,
    'item' => Colors.amber,
    'door' => Colors.orange,
    'liquid' => Colors.blue,
    'vegetation' => Colors.green,
    'pillar' => Colors.blueGrey,
    _ => Colors.purple,
  };

  // ── Step 3: Prepare results ───────────────────────────

  Widget _buildPrepareResult(ThemeData theme, ScannerState scanner) {
    final p = scanner.prepareResult!;

    // Fetch available datasets on first render
    if (_availableDatasets.isEmpty) {
      _fetchDatasets();
    }

    final totalSelected = p.trainCount +
        _availableDatasets
            .where((d) => _selectedDatasets.contains(d.path))
            .fold(0, (sum, d) => sum + d.sampleCount);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _statRow(theme, 'New training samples', '${p.trainCount}'),
        _statRow(theme, 'After stratification', '${p.totalStratified}'),
        const SizedBox(height: 12),

        // Existing datasets to merge
        if (_availableDatasets.isNotEmpty) ...[
          Text('Merge with existing data:',
            style: theme.textTheme.bodySmall!.copyWith(fontWeight: FontWeight.w600)),
          const SizedBox(height: 4),
          ..._availableDatasets.map((d) => Row(
            children: [
              SizedBox(
                width: 24, height: 24,
                child: Checkbox(
                  value: _selectedDatasets.contains(d.path),
                  onChanged: (v) => setState(() {
                    if (v == true) {
                      _selectedDatasets.add(d.path);
                    } else {
                      _selectedDatasets.remove(d.path);
                    }
                  }),
                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
              ),
              const SizedBox(width: 4),
              Expanded(child: Text(d.name,
                style: theme.textTheme.bodySmall, overflow: TextOverflow.ellipsis)),
              Text('${d.sampleCount}',
                style: theme.textTheme.bodySmall!.copyWith(
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.5))),
            ],
          )),
          const SizedBox(height: 4),
          _statRow(theme, 'Total selected', '$totalSelected'),
        ],

        const SizedBox(height: 12),
        // Epochs selector
        Row(
          children: [
            Text('Epochs:', style: theme.textTheme.bodySmall),
            const SizedBox(width: 8),
            DropdownButton<int>(
              value: _epochs,
              isDense: true,
              items: const [
                DropdownMenuItem(value: 3, child: Text('3 (fast)')),
                DropdownMenuItem(value: 5, child: Text('5 (balanced)')),
                DropdownMenuItem(value: 10, child: Text('10 (best quality)')),
              ],
              onChanged: (v) => setState(() => _epochs = v!),
            ),
            const Spacer(),
            Text(
              '~${totalSelected * _epochs ~/ 2 ~/ 60} min',
              style: theme.textTheme.bodySmall!.copyWith(
                color: theme.colorScheme.primary,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Future<void> _fetchDatasets() async {
    final backend = ref.read(backendProvider.notifier).backend;
    final dirs = ref.read(scannerProvider).datasetDirs;
    final resp = await backend.listDatasets(dirs: dirs);

    if (resp.containsKey('datasets') && resp['datasets'] is List) {
      final found = <_DatasetEntry>[];
      for (final d in resp['datasets'] as List) {
        if (d is Map<String, dynamic>) {
          found.add(_DatasetEntry(
            name: d['name'] as String? ?? '',
            path: d['path'] as String? ?? '',
            sampleCount: (d['sample_count'] as num?)?.toInt() ?? 0,
          ));
        }
      }
      if (mounted) {
        setState(() => _availableDatasets = found);
      }
    }
  }

  // ── Step 4: Training progress ─────────────────────────

  Widget _buildTrainingProgress(ThemeData theme, ScannerState scanner) {
    final pct = (scanner.trainingProgress * 100).toStringAsFixed(0);
    final lossStr = scanner.trainingLoss?.toStringAsFixed(4) ?? '--';
    final epochStr = '${scanner.trainingEpoch}/${scanner.totalEpochs}';

    return Column(
      children: [
        // Progress bar
        const SizedBox(height: 8),
        LinearProgressIndicator(
          value: scanner.trainingProgress > 0 ? scanner.trainingProgress : null,
          minHeight: 8,
          borderRadius: BorderRadius.circular(4),
        ),
        const SizedBox(height: 8),

        // Compact summary line
        Text(
          '$pct% · Epoch $epochStr · $lossStr loss',
          style: theme.textTheme.bodySmall!.copyWith(fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 12),

        // Action buttons row
        Row(
          children: [
            // Pause / Resume
            OutlinedButton.icon(
              onPressed: _togglePause,
              icon: Icon(
                _isPaused ? Icons.play_arrow : Icons.pause,
                size: 16,
              ),
              label: Text(_isPaused ? 'Resume' : 'Pause'),
              style: OutlinedButton.styleFrom(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                textStyle: theme.textTheme.bodySmall,
              ),
            ),
            const SizedBox(width: 8),

            // Throttle dropdown
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              decoration: BoxDecoration(
                border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.4)),
                borderRadius: BorderRadius.circular(8),
              ),
              child: DropdownButtonHideUnderline(
                child: DropdownButton<String>(
                  value: _throttleLevel,
                  isDense: true,
                  style: theme.textTheme.bodySmall,
                  items: const [
                    DropdownMenuItem(value: 'full', child: Text('Full Speed')),
                    DropdownMenuItem(value: 'normal', child: Text('Normal')),
                    DropdownMenuItem(value: 'background', child: Text('Background')),
                    DropdownMenuItem(value: 'minimal', child: Text('Minimal')),
                  ],
                  onChanged: (v) {
                    if (v == null) return;
                    _setThrottle(v);
                  },
                ),
              ),
            ),
            const SizedBox(width: 8),

            // Details toggle
            IconButton(
              onPressed: () => setState(() => _showDetails = !_showDetails),
              icon: Icon(
                _showDetails ? Icons.expand_less : Icons.expand_more,
                size: 20,
              ),
              tooltip: _showDetails ? 'Hide details' : 'Show details',
              style: IconButton.styleFrom(
                padding: const EdgeInsets.all(4),
                minimumSize: const Size(32, 32),
              ),
            ),

            const Spacer(),

            // Cancel
            TextButton.icon(
              onPressed: () async {
                await ref.read(backendProvider.notifier).backend.stopTraining();
              },
              icon: const Icon(Icons.stop, size: 16),
              label: const Text('Cancel'),
              style: TextButton.styleFrom(
                foregroundColor: Colors.red,
                textStyle: theme.textTheme.bodySmall,
              ),
            ),
          ],
        ),

        // Expanded detail view
        if (_showDetails) ...[
          const SizedBox(height: 12),
          const Divider(height: 1),
          const SizedBox(height: 8),
          _statRow(theme, 'Iteration',
              '${(scanner.trainingProgress * (scanner.totalEpochs > 0 ? scanner.totalEpochs : 1) * (scanner.lossHistory.isNotEmpty ? scanner.lossHistory.length : 1) ).round()}'),
          _statRow(theme, 'Epoch', epochStr),
          _statRow(theme, 'Loss', lossStr),
          if (scanner.trainingBestLoss != null)
            _statRow(theme, 'Best loss', scanner.trainingBestLoss!.toStringAsFixed(4)),
          if (scanner.trainingSpeed != null)
            _statRow(theme, 'Speed', '${scanner.trainingSpeed!.toStringAsFixed(1)} it/sec'),
          _statRow(theme, 'ETA', _formatEta(scanner)),
          if (scanner.adapterPath != null)
            _statRow(theme, 'Adapter', scanner.adapterPath!),

          // Loss curve chart
          if (scanner.lossHistory.length >= 2) ...[
            const SizedBox(height: 12),
            Text('Loss curve', style: theme.textTheme.bodySmall!.copyWith(fontWeight: FontWeight.w600)),
            const SizedBox(height: 4),
            SizedBox(
              height: 100,
              child: CustomPaint(
                size: const Size(double.infinity, 100),
                painter: _LossCurvePainter(
                  values: scanner.lossHistory,
                  color: theme.colorScheme.primary,
                  gridColor: theme.colorScheme.onSurface.withValues(alpha: 0.1),
                ),
              ),
            ),
          ],
        ],

        const SizedBox(height: 8),
        Text(
          'Training runs on your machine. Your data never leaves your computer.',
          style: theme.textTheme.bodySmall!.copyWith(
            color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
            fontStyle: FontStyle.italic,
          ),
        ),
      ],
    );
  }

  String _formatEta(ScannerState scanner) {
    if (scanner.trainingSpeed == null || scanner.trainingSpeed! <= 0) return '--';
    final remaining = ((1.0 - scanner.trainingProgress) *
            scanner.totalEpochs *
            (scanner.lossHistory.isNotEmpty ? scanner.lossHistory.length : 1))
        .round();
    if (remaining <= 0) return '< 1 min';
    final mins = (remaining / scanner.trainingSpeed! / 60).round();
    if (mins < 1) return '< 1 min';
    if (mins < 60) return '$mins min';
    return '${mins ~/ 60}h ${mins % 60}m';
  }

  Future<void> _togglePause() async {
    final backend = ref.read(backendProvider.notifier).backend;
    final scanner = ref.read(scannerProvider);

    if (_isPaused) {
      // Resume: restart training from checkpoint
      setState(() => _isPaused = false);
      ref.read(scannerProvider.notifier).updateTrainingProgress(
        scanner.trainingProgress, scanner.trainingLoss, paused: false,
      );

      final resp = await backend.startTraining(
        dataDir: scanner.prepareResult?.dataDir ?? 'training/data_${scanner.styleName}',
        adapterPath: scanner.adapterPath ?? 'training/adapters/${scanner.styleName}',
        epochs: _epochs,
        resume: true,
      );

      if (!resp.containsKey('error')) {
        _pollTrainingStatus(scanner.adapterPath ?? 'training/adapters/${scanner.styleName}');
      }
    } else {
      // Pause: stop process gracefully (checkpoint auto-saved)
      await backend.pauseTraining();
      setState(() => _isPaused = true);
      ref.read(scannerProvider.notifier).updateTrainingProgress(
        scanner.trainingProgress, scanner.trainingLoss, paused: true,
      );
    }
  }

  Future<void> _setThrottle(String level) async {
    final backend = ref.read(backendProvider.notifier).backend;
    final resp = await backend.throttleTraining(level);
    if (resp['ok'] == true) {
      setState(() {
        _throttleLevel = level;
      });
    }
  }

  // ── Step 5: Done ──────────────────────────────────────

  Widget _buildTrainedResult(ThemeData theme, ScannerState scanner) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: Colors.green.withValues(alpha: 0.1),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: Colors.green.withValues(alpha: 0.3)),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Icon(Icons.check_circle, color: Colors.green, size: 20),
                  const SizedBox(width: 8),
                  Text('Adapter trained successfully',
                    style: theme.textTheme.bodyMedium!.copyWith(fontWeight: FontWeight.w600)),
                ],
              ),
              if (scanner.adapterPath != null) ...[
                const SizedBox(height: 8),
                Text(scanner.adapterPath!,
                  style: theme.textTheme.bodySmall!.copyWith(fontFamily: 'monospace')),
              ],
            ],
          ),
        ),
        const SizedBox(height: 16),
        Text(
          'The adapter is ready. Activate it to generate tiles in this style.',
          style: theme.textTheme.bodySmall,
        ),
      ],
    );
  }

  // ── Error ─────────────────────────────────────────────

  Widget _buildError(ThemeData theme, ScannerState scanner) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.red.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.red.withValues(alpha: 0.3)),
      ),
      child: Text(scanner.error ?? 'Unknown error',
        style: theme.textTheme.bodySmall!.copyWith(color: Colors.red)),
    );
  }

  Widget _buildProgress(ThemeData theme, String message) {
    return Column(
      children: [
        const SizedBox(height: 24),
        const CircularProgressIndicator(),
        const SizedBox(height: 16),
        Text(message, style: theme.textTheme.bodySmall),
        const SizedBox(height: 24),
      ],
    );
  }

  // ── Actions bar ───────────────────────────────────────

  Widget _buildActions(ThemeData theme, ScannerState scanner) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        TextButton(
          onPressed: () async {
            // If training is running, stop it first
            if (scanner.phase == ScannerPhase.training) {
              await ref.read(backendProvider.notifier).backend.stopTraining();
            }
            ref.read(scannerProvider.notifier).reset();
            if (mounted) Navigator.of(context).pop();
          },
          child: Text(scanner.phase == ScannerPhase.trained ? 'Close' : 'Cancel'),
        ),
        const SizedBox(width: 8),
        if (scanner.phase == ScannerPhase.scanned)
          FilledButton.icon(
            onPressed: _runPrepare,
            icon: const Icon(Icons.auto_fix_high, size: 16),
            label: const Text('Prepare Data'),
          ),
        if (scanner.phase == ScannerPhase.prepared)
          FilledButton.icon(
            onPressed: _runTraining,
            icon: const Icon(Icons.model_training, size: 16),
            label: const Text('Start Training'),
          ),
        if (scanner.phase == ScannerPhase.trained)
          FilledButton.icon(
            onPressed: _activateAdapter,
            icon: const Icon(Icons.check, size: 16),
            label: const Text('Activate Adapter'),
          ),
        if (scanner.phase == ScannerPhase.error)
          FilledButton.icon(
            onPressed: () => ref.read(scannerProvider.notifier).reset(),
            icon: const Icon(Icons.refresh, size: 16),
            label: const Text('Try Again'),
          ),
      ],
    );
  }

  Future<void> _runPrepare() async {
    final notifier = ref.read(scannerProvider.notifier);
    final scanner = ref.read(scannerProvider);
    notifier.setPhase(ScannerPhase.preparing);

    final backend = ref.read(backendProvider.notifier).backend;
    final resp = await backend.prepareTraining(
      scanDir: scanner.scanSummary?.scanDir ?? '',
      outputDir: 'training/data_${scanner.styleName}',
      style: scanner.styleName,
      colorAug: _colorAug,
    );

    if (resp.containsKey('error')) {
      notifier.setError(resp['error'] as String);
    } else {
      notifier.setPrepareResult(PrepareResult.fromJson(resp));
    }
  }

  Future<void> _runTraining() async {
    final notifier = ref.read(scannerProvider.notifier);
    final scanner = ref.read(scannerProvider);
    notifier.setTraining();

    final backend = ref.read(backendProvider.notifier).backend;
    final adapterPath = 'training/adapters/${scanner.styleName}';

    final resp = await backend.startTraining(
      dataDir: scanner.prepareResult?.dataDir ?? 'training/data_${scanner.styleName}',
      adapterPath: adapterPath,
      epochs: _epochs,
    );

    if (resp.containsKey('error')) {
      notifier.setError(resp['error'] as String);
      return;
    }

    // Poll training status
    _pollTrainingStatus(adapterPath);
  }

  Future<void> _pollTrainingStatus(String adapterPath) async {
    final notifier = ref.read(scannerProvider.notifier);
    final backend = ref.read(backendProvider.notifier).backend;

    while (mounted) {
      await Future.delayed(const Duration(seconds: 3));

      final status = await backend.trainingStatus();
      final statusStr = status['status'] as String? ?? 'idle';
      final progress = (status['progress'] as num?)?.toDouble() ?? 0.0;
      final loss = (status['loss'] as num?)?.toDouble();
      final speed = (status['speed'] as num?)?.toDouble();
      final bestLoss = (status['best_loss'] as num?)?.toDouble();
      final epoch = (status['epoch'] as num?)?.toInt();
      final totalEpochs = (status['total_epochs'] as num?)?.toInt();
      final paused = status['paused'] as bool?;

      // Sync local pause state with server
      if (paused != null && paused != _isPaused) {
        setState(() => _isPaused = paused);
      }

      notifier.updateTrainingProgress(
        progress,
        loss,
        speed: speed,
        bestLoss: bestLoss,
        epoch: epoch,
        totalEpochs: totalEpochs,
        paused: paused,
      );

      if (statusStr == 'done') {
        final error = status['error'] as String?;
        if (error != null) {
          notifier.setError(error);
        } else {
          notifier.setTrained(adapterPath);
        }
        return;
      }

      if (statusStr == 'idle') {
        // Process ended unexpectedly
        notifier.setError('Training process ended unexpectedly');
        return;
      }
    }
  }

  Future<void> _activateAdapter() async {
    final scanner = ref.read(scannerProvider);
    if (scanner.adapterPath == null) return;

    final backend = ref.read(backendProvider.notifier).backend;
    await backend.activateAdapter(scanner.adapterPath!);
    ref.read(scannerProvider.notifier).setActiveAdapter(scanner.adapterPath);

    if (mounted) Navigator.of(context).pop();
  }

  Widget _statRow(ThemeData theme, String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: theme.textTheme.bodySmall),
          Text(value, style: theme.textTheme.bodySmall!.copyWith(fontWeight: FontWeight.w600)),
        ],
      ),
    );
  }
}

/// Entry for the dataset checklist.
class _DatasetEntry {
  _DatasetEntry({required this.name, required this.path, required this.sampleCount});
  final String name;
  final String path;
  final int sampleCount;
}

/// Simple loss curve painter — draws a line chart of loss values over time.
class _LossCurvePainter extends CustomPainter {
  _LossCurvePainter({
    required this.values,
    required this.color,
    required this.gridColor,
  });

  final List<double> values;
  final Color color;
  final Color gridColor;

  @override
  void paint(Canvas canvas, Size size) {
    if (values.length < 2) return;

    final maxVal = values.reduce((a, b) => a > b ? a : b);
    final minVal = values.reduce((a, b) => a < b ? a : b);
    final range = (maxVal - minVal).clamp(0.001, double.infinity);

    // Grid lines
    final gridPaint = Paint()..color = gridColor..strokeWidth = 0.5;
    for (var i = 0; i <= 4; i++) {
      final y = size.height * i / 4;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), gridPaint);
    }

    // Loss line
    final paint = Paint()
      ..color = color
      ..strokeWidth = 1.5
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    final path = Path();
    for (var i = 0; i < values.length; i++) {
      final x = size.width * i / (values.length - 1);
      final y = size.height * (1.0 - (values[i] - minVal) / range);
      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    canvas.drawPath(path, paint);

    // Fill under curve
    final fillPath = Path.from(path)
      ..lineTo(size.width, size.height)
      ..lineTo(0, size.height)
      ..close();
    final fillPaint = Paint()
      ..color = color.withValues(alpha: 0.1)
      ..style = PaintingStyle.fill;
    canvas.drawPath(fillPath, fillPaint);
  }

  @override
  bool shouldRepaint(_LossCurvePainter old) =>
      old.values.length != values.length || old.values.lastOrNull != values.lastOrNull;
}
