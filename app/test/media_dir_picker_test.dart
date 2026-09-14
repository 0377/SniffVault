import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/features/settings/media_dir_picker.dart';

void main() {
  test('mediaDirNameFromPickerResult extracts basename', () {
    expect(
      mediaDirNameFromPickerResult('/foo/bar/SniffVault'),
      'SniffVault',
    );
    expect(
      mediaDirNameFromPickerResult(r'C:\Users\me\SniffVault'),
      'SniffVault',
    );
    expect(mediaDirNameFromPickerResult('/foo/bar/SniffVault/'), 'SniffVault');
  });

  test('mediaDirNameFromPickerResult null or invalid returns null', () {
    expect(mediaDirNameFromPickerResult(null), isNull);
    expect(mediaDirNameFromPickerResult(''), isNull);
    expect(mediaDirNameFromPickerResult('/'), isNull);
    expect(mediaDirNameFromPickerResult('.'), isNull);
  });
}
