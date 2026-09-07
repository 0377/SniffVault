import 'package:video_sniffing/engine/models/sniff_types.dart';

class HookRequest {
  const HookRequest({
    required this.url,
    this.pageUrl,
    required this.isMainFrame,
    this.mime,
  });

  final String url;
  final String? pageUrl;
  final bool isMainFrame;
  final String? mime;
}

SniffEvent hookToSniffEvent(HookRequest request) {
  return SniffEvent(
    url: request.url,
    pageUrl: request.pageUrl,
    initiator: _initiatorFor(request),
  );
}

SniffInitiator _initiatorFor(HookRequest request) {
  if (request.isMainFrame) {
    return SniffInitiator.navigation;
  }
  if (_isMediaMime(request.mime) || _isMediaPath(request.url)) {
    return SniffInitiator.media;
  }
  return SniffInitiator.subResource;
}

bool _isMediaMime(String? mime) {
  if (mime == null) {
    return false;
  }
  final lower = mime.toLowerCase();
  return lower.contains('video') || lower.contains('audio');
}

bool _isMediaPath(String url) {
  final uri = Uri.tryParse(url);
  final path = (uri?.path.isNotEmpty == true ? uri!.path : url.split('?').first)
      .toLowerCase();
  return path.endsWith('.m3u8') || path.endsWith('.mp4');
}
