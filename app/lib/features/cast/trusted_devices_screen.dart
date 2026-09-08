import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/cast_types.dart';
import 'package:video_sniffing/providers/engine_host_provider.dart';

class TrustedDevicesScreen extends ConsumerStatefulWidget {
  const TrustedDevicesScreen({super.key});

  @override
  ConsumerState<TrustedDevicesScreen> createState() =>
      _TrustedDevicesScreenState();
}

class _TrustedDevicesScreenState extends ConsumerState<TrustedDevicesScreen> {
  void _refresh() => setState(() {});

  void _removePeer(String deviceId) {
    ref.read(engineRepositoryProvider).removeTrustedPeer(deviceId);
    _refresh();
  }

  @override
  Widget build(BuildContext context) {
    final peers = ref.read(engineRepositoryProvider).listTrustedPeers();

    return Scaffold(
      appBar: AppBar(title: const Text('已信任设备')),
      body: peers.isEmpty
          ? const Center(child: Text('暂无已信任设备'))
          : ListView.builder(
              key: const Key('trusted_devices_list'),
              itemCount: peers.length,
              itemBuilder: (context, index) {
                final peer = peers[index];
                return _TrustedDeviceTile(
                  peer: peer,
                  onRemove: () => _removePeer(peer.peerDeviceId),
                );
              },
            ),
    );
  }
}

class _TrustedDeviceTile extends StatelessWidget {
  const _TrustedDeviceTile({required this.peer, required this.onRemove});

  final TrustedPeer peer;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      key: Key('trusted_device_${peer.peerDeviceId}'),
      title: Text(peer.peerName),
      subtitle: Text('${peer.peerHost}:${peer.peerPort}'),
      trailing: IconButton(
        key: Key('trusted_device_remove_${peer.peerDeviceId}'),
        icon: const Icon(Icons.delete_outline),
        tooltip: '撤销信任',
        onPressed: onRemove,
      ),
    );
  }
}
