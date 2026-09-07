Pod::Spec.new do |s|
  s.name             = 'webview_sniff'
  s.version          = '0.0.1'
  s.summary          = 'Cookie store and Android isTelevision detection.'
  s.description      = <<-DESC
Cookie store for the Video Sniffing WebView and Android isTelevision detection.
                       DESC
  s.homepage         = 'https://github.com/bingyun/video_sniffing'
  s.license          = { :type => 'Apache-2.0' }
  s.author           = { 'Video Sniffing' => 'noreply@example.com' }
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'Flutter'
  s.platform = :ios, '13.0'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES', 'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386' }
  s.swift_version = '5.0'
end
