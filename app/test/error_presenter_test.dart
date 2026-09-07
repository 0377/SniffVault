import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/engine/engine_host.dart';
import 'package:video_sniffing/engine/models/ffi_response.dart';
import 'package:video_sniffing/ui/error_presenter.dart';

void main() {
  test('maps http kind to network message', () {
    final message = presentEngineError(
      EngineException(const FfiError(kind: 'http', message: 'timeout')),
    );
    expect(message, contains('网络'));
  });

  test('maps invalid_arg to raw message', () {
    final message = presentEngineError(
      EngineException(
        const FfiError(kind: 'invalid_arg', message: 'media_dir invalid'),
      ),
    );
    expect(message, 'media_dir invalid');
  });

  test('suppresses downloads already running', () {
    expect(
      presentEngineError(
        EngineException(
          const FfiError(
            kind: 'message',
            message: 'invalid argument: downloads already running',
          ),
        ),
      ),
      isNull,
    );
  });
}
