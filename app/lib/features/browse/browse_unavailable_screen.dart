import 'package:flutter/material.dart';

class BrowseUnavailableScreen extends StatelessWidget {
  const BrowseUnavailableScreen({
    super.key,
    this.message = '请在手机或电脑使用内置浏览',
    this.detail,
    this.helpUrl,
  });

  final String message;
  final String? detail;
  final Uri? helpUrl;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(message, textAlign: TextAlign.center),
              if (detail != null) ...[
                const SizedBox(height: 12),
                Text(detail!, textAlign: TextAlign.center),
              ],
              if (helpUrl != null) ...[
                const SizedBox(height: 12),
                SelectableText(helpUrl!.toString()),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
