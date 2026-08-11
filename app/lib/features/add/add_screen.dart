import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class AddScreen extends ConsumerWidget {
  const AddScreen({super.key, this.initialUrl});

  final String? initialUrl;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return const Scaffold(
      body: Center(child: Text('添加')),
    );
  }
}
