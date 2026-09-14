import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_sniffing/engine/models/download_auth.dart';
import 'package:video_sniffing/engine/models/resolve_types.dart';

class BrowseResolveArgs {
  const BrowseResolveArgs({
    required this.outcome,
    this.auth,
    this.posterUrl,
  });

  final ResolveOutcome outcome;
  final DownloadAuth? auth;
  final String? posterUrl;
}

final browseResolveProvider = StateProvider<BrowseResolveArgs?>((ref) => null);
