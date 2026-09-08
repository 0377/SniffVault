const sniffChannelName = 'SniffChannel';

const sniffScript = r'''
(function() {
  if (window.__vsSniffHooked) return;
  window.__vsSniffHooked = true;
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
  const origFetch = window.fetch;
  window.fetch = function() { try { post(arguments[0], ''); } catch (e) {} return origFetch.apply(this, arguments); };
  const origOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function(method, url) { try { post(url, ''); } catch (e) {} return origOpen.apply(this, arguments); };
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
