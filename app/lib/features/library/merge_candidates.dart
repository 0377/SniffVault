import 'package:video_sniffing/engine/models/library_item.dart';
import 'package:video_sniffing/engine/models/library_item_kind.dart';

bool isMergeCandidate(LibraryItem source, LibraryItem other) =>
    other.id != source.id &&
    other.kind == LibraryItemKind.series &&
    source.kind == LibraryItemKind.series &&
    other.title == source.title &&
    other.season == source.season;

List<LibraryItem> mergeCandidatesFor(
  LibraryItem source,
  List<LibraryItem> allItems,
) =>
    allItems.where((other) => isMergeCandidate(source, other)).toList();
