import 'package:flutter/material.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';
import 'package:video_sniffing/features/add/widgets/episode_multi_select.dart';
import 'package:video_sniffing/features/add/widgets/quality_picker.dart';

typedef ResolveQualitiesCallback = Future<List<Quality>> Function(String url);
typedef EnqueueSingleCallback = String Function({
  required String title,
  required String url,
  String? qualityLabel,
});
typedef EnqueueEpisodesCallback = EnqueueEpisodesResult Function({
  required String listTitle,
  int? season,
  required List<(int index, String title, String url)> episodes,
  String? qualityLabel,
});

class ResolveWizard extends StatefulWidget {
  const ResolveWizard({
    super.key,
    required this.outcome,
    required this.onEnqueue,
    this.resolveQualities,
    this.enqueueSingle,
    this.enqueueEpisodes,
    this.defaultQualityLabel,
  });

  final ResolveOutcome outcome;
  final Future<void> Function(BuildContext context) onEnqueue;
  final ResolveQualitiesCallback? resolveQualities;
  final EnqueueSingleCallback? enqueueSingle;
  final EnqueueEpisodesCallback? enqueueEpisodes;
  final String? defaultQualityLabel;

  @override
  State<ResolveWizard> createState() => _ResolveWizardState();
}

class _ResolveWizardState extends State<ResolveWizard> {
  late final TextEditingController _titleController;
  ResourceCandidate? _selectedCandidate;
  List<Quality> _qualities = [];
  Quality? _selectedQuality;
  var _loadingQualities = false;
  List<Episode> _selectedEpisodes = [];
  var _enqueueing = false;

  @override
  void initState() {
    super.initState();
    final outcome = widget.outcome;
    if (outcome is ResolveOutcomeSingle) {
      _titleController = TextEditingController(text: outcome.candidate.title ?? '');
    } else if (outcome is ResolveOutcomeCandidates &&
        outcome.candidates.length == 1) {
      _titleController = TextEditingController(
        text: outcome.candidates.first.title ?? '',
      );
      _selectCandidate(outcome.candidates.first);
    } else {
      _titleController = TextEditingController();
    }
  }

  @override
  void dispose() {
    _titleController.dispose();
    super.dispose();
  }

  Future<void> _selectCandidate(ResourceCandidate candidate) async {
    setState(() {
      _selectedCandidate = candidate;
      _qualities = [];
      _selectedQuality = candidate.quality;
      _loadingQualities = false;
    });

    if (candidate.quality == null &&
        candidate.kind == MediaKind.hls &&
        widget.resolveQualities != null) {
      setState(() => _loadingQualities = true);
      try {
        final qualities = await widget.resolveQualities!(candidate.url);
        if (!mounted) return;
        setState(() {
          _qualities = qualities;
          _selectedQuality = qualities.isNotEmpty ? qualities.first : null;
          _loadingQualities = false;
        });
      } catch (_) {
        if (mounted) {
          setState(() => _loadingQualities = false);
        }
      }
    }
  }

  Future<void> _enqueueSingle() async {
    if (_enqueueing) return;
    final candidate = switch (widget.outcome) {
      ResolveOutcomeSingle(:final candidate) => candidate,
      ResolveOutcomeCandidates() => _selectedCandidate,
      _ => null,
    };
    if (candidate == null || widget.enqueueSingle == null) return;

    setState(() => _enqueueing = true);
    try {
      widget.enqueueSingle!(
        title: _titleController.text.trim().isEmpty
            ? (candidate.title ?? '未命名')
            : _titleController.text.trim(),
        url: candidate.url,
        qualityLabel: _selectedQuality?.label ?? widget.defaultQualityLabel,
      );
      await widget.onEnqueue(context);
    } finally {
      if (mounted) {
        setState(() => _enqueueing = false);
      }
    }
  }

  Future<void> _enqueueEpisodes() async {
    if (_enqueueing || widget.enqueueEpisodes == null) return;
    final outcome = widget.outcome;
    if (outcome is! ResolveOutcomeEpisodeList) return;
    if (_selectedEpisodes.isEmpty) return;

    setState(() => _enqueueing = true);
    try {
      widget.enqueueEpisodes!(
        listTitle: outcome.episodeList.title,
        season: outcome.episodeList.season,
        episodes: _selectedEpisodes
            .map((e) => (e.index, e.title, e.url))
            .toList(),
        qualityLabel: widget.defaultQualityLabel,
      );
      await widget.onEnqueue(context);
    } finally {
      if (mounted) {
        setState(() => _enqueueing = false);
      }
    }
  }

  bool _canEnqueueSingle() {
    final candidate = switch (widget.outcome) {
      ResolveOutcomeSingle(:final candidate) => candidate,
      ResolveOutcomeCandidates() => _selectedCandidate,
      _ => null,
    };
    if (candidate == null) return false;
    if (candidate.kind == MediaKind.hls &&
        candidate.quality == null &&
        _selectedQuality == null &&
        widget.resolveQualities != null) {
      return false;
    }
    return true;
  }

  @override
  Widget build(BuildContext context) {
    return switch (widget.outcome) {
      ResolveOutcomeSingle(:final candidate) => _buildSingle(context, candidate),
      ResolveOutcomeCandidates(:final candidates) =>
        _buildCandidates(context, candidates),
      ResolveOutcomeEpisodeList(:final episodeList) =>
        _buildEpisodeList(context, episodeList),
      ResolveOutcomeNeedsBrowser(:final reason) =>
        _buildNeedsBrowser(context, reason),
    };
  }

  Widget _buildSingle(BuildContext context, ResourceCandidate candidate) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        TextField(
          key: const Key('resolve_title_field'),
          decoration: const InputDecoration(
            labelText: '标题',
            border: OutlineInputBorder(),
          ),
          controller: _titleController,
        ),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: _enqueueing ? null : _enqueueSingle,
          child: const Text('下载'),
        ),
      ],
    );
  }

  Widget _buildCandidates(
    BuildContext context,
    List<ResourceCandidate> candidates,
  ) {
    final selected = _selectedCandidate;
    final showQualityPicker = selected != null &&
        selected.quality == null &&
        selected.kind == MediaKind.hls;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('选择资源'),
        const SizedBox(height: 8),
        ...candidates.map(
          (candidate) => ListTile(
            title: Text(candidate.title ?? candidate.url),
            subtitle: Text(candidate.kind.jsonValue),
            selected: selected?.id == candidate.id,
            onTap: () => _selectCandidate(candidate),
          ),
        ),
        if (showQualityPicker) ...[
          const SizedBox(height: 16),
          if (_loadingQualities)
            const Center(child: CircularProgressIndicator())
          else
            QualityPicker(
              qualities: _qualities,
              selected: _selectedQuality,
              onSelected: (quality) =>
                  setState(() => _selectedQuality = quality),
            ),
        ],
        if (selected != null &&
            (selected.quality != null || selected.kind != MediaKind.hls)) ...[
          const SizedBox(height: 16),
          TextField(
            key: const Key('resolve_title_field'),
            decoration: const InputDecoration(
              labelText: '标题',
              border: OutlineInputBorder(),
            ),
            controller: _titleController,
          ),
        ],
        if (selected != null) ...[
          const SizedBox(height: 16),
          FilledButton(
            onPressed: _enqueueing || !_canEnqueueSingle()
                ? null
                : _enqueueSingle,
            child: const Text('下载'),
          ),
        ],
      ],
    );
  }

  Widget _buildEpisodeList(BuildContext context, EpisodeList episodeList) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        EpisodeMultiSelect(
          episodeList: episodeList,
          onSelectionChanged: (episodes) =>
              setState(() => _selectedEpisodes = episodes),
        ),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: _enqueueing || _selectedEpisodes.isEmpty
              ? null
              : _enqueueEpisodes,
          child: const Text('下载'),
        ),
      ],
    );
  }

  Widget _buildNeedsBrowser(BuildContext context, String reason) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text('此站点需登录浏览，内置浏览器将在后续版本支持'),
          if (reason.isNotEmpty) ...[
            const SizedBox(height: 8),
            Text(
              reason,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
          const Spacer(),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('返回'),
          ),
        ],
      ),
    );
  }
}
