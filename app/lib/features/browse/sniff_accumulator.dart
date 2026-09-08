import 'package:video_sniffing/engine/models/sniff_types.dart';

class SniffAccumulator {
  static const int maxEvents = 500;

  final List<SniffEvent> _events = [];

  List<SniffEvent> get events => List<SniffEvent>.unmodifiable(_events);

  void add(SniffEvent event) {
    _events.add(event);
    if (_events.length > maxEvents) {
      _events.removeRange(0, _events.length - maxEvents);
    }
  }

  void onTopLevelNavigation() {
    _events.clear();
  }
}
