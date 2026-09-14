import 'package:file_picker/file_picker.dart';

abstract class MediaDirectoryPicker {
  Future<String?> pickDirectoryPath();
}

class FilePickerMediaDirectoryPicker implements MediaDirectoryPicker {
  @override
  Future<String?> pickDirectoryPath() => FilePicker.getDirectoryPath();
}
