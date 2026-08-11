class FfiError {
  const FfiError({
    required this.kind,
    required this.message,
  });

  final String kind;
  final String message;

  factory FfiError.fromJson(Map<String, dynamic> json) {
    return FfiError(
      kind: json['kind'] as String,
      message: json['message'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
        'kind': kind,
        'message': message,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FfiError && kind == other.kind && message == other.message;

  @override
  int get hashCode => Object.hash(kind, message);
}

class FfiResponse<T> {
  const FfiResponse({
    required this.ok,
    this.data,
    this.error,
  });

  final bool ok;
  final T? data;
  final FfiError? error;

  factory FfiResponse.fromJson(
    Map<String, dynamic> json,
    T Function(Object? json) fromJsonT,
  ) {
    return FfiResponse(
      ok: json['ok'] as bool,
      data: json['data'] == null ? null : fromJsonT(json['data']),
      error: json['error'] == null
          ? null
          : FfiError.fromJson(json['error'] as Map<String, dynamic>),
    );
  }

  Map<String, dynamic> toJson(Object? Function(T value) toJsonT) => {
        'ok': ok,
        if (data != null) 'data': toJsonT(data as T),
        if (error != null) 'error': error!.toJson(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FfiResponse<T> &&
          ok == other.ok &&
          data == other.data &&
          error == other.error;

  @override
  int get hashCode => Object.hash(ok, data, error);
}
