import 'package:flutter/material.dart';

class BrowseUnavailableScreen extends StatelessWidget {
  const BrowseUnavailableScreen({super.key, this.message = '请在手机或电脑使用内置浏览'});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Scaffold(body: Center(child: Text(message)));
  }
}
