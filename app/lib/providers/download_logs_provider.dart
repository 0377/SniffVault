import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/download_log_entry.dart';

const maxDownloadLogEntries = 200;

class DownloadLogsNotifier extends StateNotifier<List<DownloadLogEntry>> {
  DownloadLogsNotifier() : super(const []);

  void append(DownloadLogEntry entry) {
    final next = [...state, entry];
    if (next.length > maxDownloadLogEntries) {
      state = next.sublist(next.length - maxDownloadLogEntries);
    } else {
      state = next;
    }
  }

  void clear() {
    state = const [];
  }
}

final downloadLogsProvider =
    StateNotifierProvider<DownloadLogsNotifier, List<DownloadLogEntry>>(
  (ref) => DownloadLogsNotifier(),
);
