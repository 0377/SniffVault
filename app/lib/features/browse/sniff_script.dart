const sniffChannelName = 'SniffChannel';

const sniffScript = r'''
(function() {
  const post = (url, mime) => {
    try {
      SniffChannel.postMessage(JSON.stringify({url: String(url), mime: mime || '', is_main_frame: false}));
    } catch (e) {}
  };
  const origFetch = window.fetch;
  window.fetch = function() { try { post(arguments[0], ''); } catch (e) {} return origFetch.apply(this, arguments); };
  const origOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function(method, url) { try { post(url, ''); } catch (e) {} return origOpen.apply(this, arguments); };
})();
''';
