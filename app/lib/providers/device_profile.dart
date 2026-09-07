import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/platform/television.dart';

final isTelevisionProvider = FutureProvider<bool>(
  (ref) => detectIsTelevision(),
);
