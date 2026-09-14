import 'package:flutter_test/flutter_test.dart';
import 'package:video_sniffing/ui/app_theme.dart';

void main() {
  test('light theme uses brand primary from logo palette', () {
    expect(AppTheme.light.colorScheme.primary, AppColors.brandBright);
    expect(AppTheme.light.colorScheme.surface, AppColors.brandSurface);
  });

  test('dark theme uses brand accent on night surface', () {
    expect(AppTheme.dark.colorScheme.primary, AppColors.brandAccent);
    expect(AppTheme.dark.colorScheme.surface, AppColors.brandNight);
  });
}
