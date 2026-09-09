const sniffChannelName = 'SniffChannel';

/// Mirrors engine `scan_media_urls` / injected JS `scanMediaUrls` for unit tests.
List<String> scanMediaUrls(String text, String pageOrigin) {
  final absRe = RegExp(r'''https?://[^\s"'<>]+\.(?:m3u8|mp4)''');
  final relRe = RegExp(r'''["']([^"']+\.(?:m3u8|mp4))["']''');

  final seen = <String>{};
  final urls = <String>[];

  for (final match in absRe.allMatches(text)) {
    final url = match.group(0)!;
    if (seen.add(url)) {
      urls.add(url);
    }
  }

  final base = Uri.tryParse(pageOrigin);
  if (base == null) {
    return urls;
  }

  for (final match in relRe.allMatches(text)) {
    final path = match.group(1)!;
    if (path.startsWith('http://') || path.startsWith('https://')) {
      continue;
    }
    final url = base.resolve(path).toString();
    if (seen.add(url)) {
      urls.add(url);
    }
  }

  return urls;
}

const sniffScript = r'''
(function() {
  if (window.__vsSniffHooked) return;
  window.__vsSniffHooked = true;
  const pageOrigin = () => {
    try { return location.href; } catch (e) { return ''; }
  };
  const href = (u) => {
    try {
      if (typeof u === 'string') return u;
      if (u && typeof u.url === 'string') return u.url;
    } catch (e) {}
    return String(u);
  };
  const post = (url, mime) => {
    try {
      SniffChannel.postMessage(JSON.stringify({url: href(url), mime: mime || '', is_main_frame: false}));
    } catch (e) {}
  };
  const scanMediaUrls = (text, origin) => {
    const urls = [];
    const seen = new Set();
    const add = (url) => {
      if (url && !seen.has(url)) {
        seen.add(url);
        urls.push(url);
      }
    };
    try {
      const absRe = /https?:\/\/[^\s"'<>]+\.(?:m3u8|mp4)/g;
      let m;
      while ((m = absRe.exec(text)) !== null) {
        add(m[0]);
      }
      const relRe = /["']([^"']+\.(?:m3u8|mp4))["']/g;
      while ((m = relRe.exec(text)) !== null) {
        const path = m[1];
        if (path.startsWith('http://') || path.startsWith('https://')) continue;
        try {
          add(new URL(path, origin).href);
        } catch (e) {}
      }
    } catch (e) {}
    return urls;
  };
  const postScanned = (text, origin) => {
    for (const url of scanMediaUrls(text, origin)) {
      post(url, '');
    }
  };
  const origFetch = window.fetch;
  window.fetch = function() {
    try { post(arguments[0], ''); } catch (e) {}
    const p = origFetch.apply(this, arguments);
    try {
      if (p && typeof p.then === 'function') {
        return p.then(function(response) {
          try {
            response.clone().text().then(function(text) {
              postScanned(text, pageOrigin());
            }).catch(function() {});
          } catch (e) {}
          return response;
        });
      }
    } catch (e) {}
    return p;
  };
  const origOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function(method, url) {
    try { post(url, ''); } catch (e) {}
    return origOpen.apply(this, arguments);
  };
  const origSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.send = function() {
    try {
      this.addEventListener('load', function() {
        try {
          const text = (this.responseText || '').slice(0, 524288);
          postScanned(text, pageOrigin());
        } catch (e) {}
      });
    } catch (e) {}
    return origSend.apply(this, arguments);
  };
  const proto = window.HTMLMediaElement && HTMLMediaElement.prototype;
  if (proto) {
    const desc = Object.getOwnPropertyDescriptor(proto, 'src');
    if (desc && desc.set) {
      Object.defineProperty(proto, 'src', {
        configurable: true,
        enumerable: desc.enumerable,
        get: desc.get,
        set: function(v) { try { post(v, ''); } catch (e) {} return desc.set.call(this, v); }
      });
    }
  }
})();
''';
