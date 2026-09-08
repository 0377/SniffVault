import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/cast_receiver/cast_providers.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

Future<void> showCastDestinationSheet(
  BuildContext context, {
  required String episodeId,
}) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    builder: (context) => CastDestinationSheet(episodeId: episodeId),
  );
}

class CastDestinationSheet extends ConsumerStatefulWidget {
  const CastDestinationSheet({super.key, required this.episodeId});

  final String episodeId;

  @override
  ConsumerState<CastDestinationSheet> createState() => _CastDestinationSheetState();
}

class _CastDestinationSheetState extends ConsumerState<CastDestinationSheet> {
  var _casting = false;
  String? _errorMessage;

  Future<void> _castToPeer(LanPeer peer) async {
    setState(() {
      _casting = true;
      _errorMessage = null;
    });

    final repo = ref.read(engineRepositoryProvider);
    try {
      if (!peer.isTrusted) {
        final pin = await _promptForPin(peer);
        if (pin == null) {
          setState(() => _casting = false);
          return;
        }
        repo.pairPeer(host: peer.host, port: peer.port, pin: pin);
      }

      repo.castEpisode(
        episodeId: widget.episodeId,
        peerDeviceId: peer.deviceId,
      );

      if (!mounted) {
        return;
      }
      Navigator.of(context).pop();
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('正在电视播放')),
      );
    } on EngineException catch (e) {
      if (!mounted) {
        return;
      }
      setState(() {
        _casting = false;
        _errorMessage = presentCastError(e);
      });
    }
  }

  Future<String?> _promptForPin(LanPeer peer) {
    final controller = TextEditingController();
    return showDialog<String>(
      context: context,
      builder: (dialogContext) {
        return AlertDialog(
          key: const Key('cast_pin_dialog'),
          title: Text('配对 ${peer.deviceName}'),
          content: TextField(
            key: const Key('cast_pin_input'),
            controller: controller,
            keyboardType: TextInputType.number,
            maxLength: 6,
            decoration: const InputDecoration(
              labelText: '电视上的 6 位配对码',
              border: OutlineInputBorder(),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('取消'),
            ),
            FilledButton(
              key: const Key('cast_pin_confirm'),
              onPressed: () => Navigator.of(dialogContext).pop(controller.text),
              child: const Text('配对并投送'),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final peersAsync = ref.watch(discoveredPeersProvider);

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              '投送到 TV',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 16),
            if (_errorMessage != null) ...[
              Text(
                _errorMessage!,
                key: const Key('cast_error'),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
              const SizedBox(height: 12),
            ],
            Flexible(
              child: peersAsync.when(
                loading: () => const Center(
                  key: Key('cast_discover_loading'),
                  child: Padding(
                    padding: EdgeInsets.all(24),
                    child: CircularProgressIndicator(),
                  ),
                ),
                error: (error, _) {
                  final message = error is EngineException
                      ? presentCastError(error)
                      : error.toString();
                  return Center(
                    child: Text(message ?? '发现电视失败'),
                  );
                },
                data: (peers) {
                  if (peers.isEmpty) {
                    return const Center(
                      key: Key('cast_empty_peers'),
                      child: Padding(
                        padding: EdgeInsets.all(24),
                        child: Text(emptyPeersMessage),
                      ),
                    );
                  }

                  return ListView.separated(
                    shrinkWrap: true,
                    itemCount: peers.length,
                    separatorBuilder: (_, __) => const Divider(height: 1),
                    itemBuilder: (context, index) {
                      final peer = peers[index];
                      return ListTile(
                        key: Key('cast_peer_${peer.deviceId}'),
                        enabled: !_casting,
                        leading: const Icon(Icons.tv),
                        title: Text(peer.deviceName),
                        subtitle: Text('${peer.host}:${peer.port}'),
                        trailing: peer.isTrusted
                            ? const Chip(label: Text('已配对'))
                            : null,
                        onTap: () => _castToPeer(peer),
                      );
                    },
                  );
                },
              ),
            ),
            if (_casting) ...[
              const SizedBox(height: 16),
              const LinearProgressIndicator(key: Key('cast_progress')),
            ],
          ],
        ),
      ),
    );
  }
}
