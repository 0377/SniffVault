enum LibraryItemKind {
  single('single'),
  series('series');

  const LibraryItemKind(this.jsonValue);

  final String jsonValue;

  static LibraryItemKind fromJson(String value) {
    return LibraryItemKind.values.firstWhere(
      (kind) => kind.jsonValue == value,
      orElse: () => throw ArgumentError('unknown library item kind: $value'),
    );
  }

  String toJson() => jsonValue;
}
