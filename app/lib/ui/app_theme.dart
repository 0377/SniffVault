import 'package:flutter/material.dart';

/// Brand palette from `assets/branding/app_icon.svg`.
abstract final class AppColors {
  static const brandDeep = Color(0xFF2B1F5E);
  static const brandMid = Color(0xFF4C35A8);
  static const brandBright = Color(0xFF7B5CF6);
  static const brandAccent = Color(0xFFA78BFA);
  static const brandMuted = Color(0xFFC4B5FD);
  static const brandLavender = Color(0xFFD9D2FF);
  static const brandLavenderLight = Color(0xFFDDD6FE);
  static const brandSurface = Color(0xFFF8F7FF);
  static const brandInk = Color(0xFF1A1240);
  static const brandNight = Color(0xFF231A4A);
}

abstract final class AppTheme {
  static const ColorScheme lightColorScheme = ColorScheme(
    brightness: Brightness.light,
    primary: AppColors.brandBright,
    onPrimary: AppColors.brandSurface,
    primaryContainer: AppColors.brandLavenderLight,
    onPrimaryContainer: AppColors.brandDeep,
    secondary: AppColors.brandMid,
    onSecondary: AppColors.brandSurface,
    secondaryContainer: AppColors.brandLavender,
    onSecondaryContainer: AppColors.brandDeep,
    tertiary: AppColors.brandAccent,
    onTertiary: AppColors.brandInk,
    tertiaryContainer: AppColors.brandMuted,
    onTertiaryContainer: AppColors.brandDeep,
    error: Color(0xFFBA1A1A),
    onError: Colors.white,
    surface: AppColors.brandSurface,
    onSurface: AppColors.brandInk,
    onSurfaceVariant: AppColors.brandMid,
    outline: AppColors.brandMuted,
    outlineVariant: AppColors.brandLavenderLight,
    shadow: AppColors.brandInk,
    scrim: AppColors.brandInk,
    inverseSurface: AppColors.brandDeep,
    onInverseSurface: AppColors.brandSurface,
    inversePrimary: AppColors.brandMuted,
    surfaceTint: AppColors.brandBright,
  );

  static const ColorScheme darkColorScheme = ColorScheme(
    brightness: Brightness.dark,
    primary: AppColors.brandAccent,
    onPrimary: AppColors.brandInk,
    primaryContainer: AppColors.brandMid,
    onPrimaryContainer: AppColors.brandLavender,
    secondary: AppColors.brandMuted,
    onSecondary: AppColors.brandInk,
    secondaryContainer: AppColors.brandDeep,
    onSecondaryContainer: AppColors.brandLavender,
    tertiary: AppColors.brandBright,
    onTertiary: AppColors.brandSurface,
    tertiaryContainer: AppColors.brandMid,
    onTertiaryContainer: AppColors.brandLavenderLight,
    error: Color(0xFFFFB4AB),
    onError: Color(0xFF690005),
    surface: AppColors.brandNight,
    onSurface: AppColors.brandSurface,
    onSurfaceVariant: AppColors.brandMuted,
    outline: AppColors.brandMid,
    outlineVariant: AppColors.brandDeep,
    shadow: Colors.black,
    scrim: Colors.black,
    inverseSurface: AppColors.brandSurface,
    onInverseSurface: AppColors.brandInk,
    inversePrimary: AppColors.brandMid,
    surfaceTint: AppColors.brandAccent,
  );

  static ThemeData light = _buildTheme(lightColorScheme);

  static ThemeData dark = _buildTheme(darkColorScheme);

  static ThemeData _buildTheme(ColorScheme colorScheme) {
    return ThemeData(
      colorScheme: colorScheme,
      useMaterial3: true,
      scaffoldBackgroundColor: colorScheme.surface,
      appBarTheme: AppBarTheme(
        backgroundColor: colorScheme.surface,
        foregroundColor: colorScheme.onSurface,
        elevation: 0,
        scrolledUnderElevation: 1,
        surfaceTintColor: colorScheme.primary,
      ),
      navigationBarTheme: NavigationBarThemeData(
        backgroundColor: colorScheme.surface,
        indicatorColor: colorScheme.primaryContainer,
        labelTextStyle: WidgetStateProperty.resolveWith((states) {
          final selected = states.contains(WidgetState.selected);
          return TextStyle(
            fontSize: 12,
            fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
            color: selected
                ? colorScheme.onSurface
                : colorScheme.onSurfaceVariant,
          );
        }),
        iconTheme: WidgetStateProperty.resolveWith((states) {
          final selected = states.contains(WidgetState.selected);
          return IconThemeData(
            color: selected ? colorScheme.primary : colorScheme.onSurfaceVariant,
          );
        }),
      ),
      navigationRailTheme: NavigationRailThemeData(
        backgroundColor: colorScheme.surface,
        indicatorColor: colorScheme.primaryContainer,
        selectedIconTheme: IconThemeData(color: colorScheme.primary),
        unselectedIconTheme: IconThemeData(color: colorScheme.onSurfaceVariant),
        selectedLabelTextStyle: TextStyle(
          color: colorScheme.onSurface,
          fontWeight: FontWeight.w600,
        ),
        unselectedLabelTextStyle: TextStyle(color: colorScheme.onSurfaceVariant),
      ),
      dividerTheme: DividerThemeData(color: colorScheme.outlineVariant),
      progressIndicatorTheme: ProgressIndicatorThemeData(
        color: colorScheme.primary,
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: colorScheme.primary,
          foregroundColor: colorScheme.onPrimary,
        ),
      ),
    );
  }
}
