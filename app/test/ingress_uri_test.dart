import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/deep_link/ingress_uri.dart';

void main() {
  test('parses sniffvault add url query', () {
    final result = parseSniffVaultIngress(
      Uri.parse('sniffvault://add?url=https%3A%2F%2Fexample.com'),
    );
    expect(result, isA<IngressNavigateSuccess>());
    expect((result! as IngressNavigateSuccess).url, 'https://example.com');
  });

  test('W11 javascript payload is invalidScheme', () {
    final result = parseSniffVaultIngress(
      Uri.parse(
        'sniffvault://add?url=${Uri.encodeComponent('javascript:alert(1)')}',
      ),
    );
    expect(result, isA<IngressNavigateFailure>());
    expect(
      (result! as IngressNavigateFailure).reason,
      IngressFailure.invalidScheme,
    );
    expect(
      snackBarMessageFor(IngressFailure.invalidScheme),
      '仅支持 http/https 链接',
    );
  });

  test('plain text without url is noHttpUrl', () {
    final result = parseSniffVaultIngress(
      Uri.parse('sniffvault://add?url=${Uri.encodeComponent('只是文字')}'),
    );
    expect((result! as IngressNavigateFailure).reason, IngressFailure.noHttpUrl);
  });

  test('ignores non sniffvault scheme', () {
    expect(parseSniffVaultIngress(Uri.parse('https://example.com')), isNull);
  });
}
