require "json"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name         = "react-native-nitro-dgram"
  s.version      = package["version"]
  s.summary      = package["description"]
  s.homepage     = "https://github.com/iwater/react-native-nitro-dgram"
  s.license      = package["license"]
  s.authors      = package["author"]
  
  s.platform     = :ios, "13.0"
  s.source       = { :git => "https://github.com/iwater/react-native-nitro-dgram.git", :tag => "v#{s.version}" }
  
  s.module_name  = "RustCUdp"

  s.source_files = [
    "ios/**/*.{h,m,mm,swift}",
    "cpp/**/*.{hpp,cpp}"
  ]
  
  s.dependency "React-Core"
  
  # Add vendored xcframework (Rust binary)
  s.vendored_frameworks = "ios/Frameworks/RustCUdp.xcframework"
  
  s.pod_target_xcconfig = {
    "HEADER_SEARCH_PATHS" => [
      "\"$(PODS_ROOT)/react-native-nitro-modules/ios\"",
      "\"$(PODS_ROOT)/Headers/Public/react-native-nitro-modules\"",
      "\"$(PODS_TARGET_SRCROOT)/nitrogen/generated/shared/c++\"",
      "\"$(PODS_TARGET_SRCROOT)/nitrogen/generated/shared\"",
      "\"$(PODS_TARGET_SRCROOT)/nitrogen/generated/ios/c++\"",
      "\"$(PODS_TARGET_SRCROOT)/nitrogen/generated/ios\"",
      "\"$(PODS_TARGET_SRCROOT)/cpp\"",
      "\"$(PODS_TARGET_SRCROOT)/ios/Frameworks/RustCUdp.xcframework/ios-arm64/RustCUdp.framework/Headers\"",
      "\"$(PODS_TARGET_SRCROOT)/ios/Frameworks/RustCUdp.xcframework/ios-arm64_x86_64-simulator/RustCUdp.framework/Headers\""
    ],
    "OTHER_SWIFT_FLAGS" => "-cxx-interoperability-mode=default"
  }

  load 'nitrogen/generated/ios/RustCUdp+autolinking.rb'
  add_nitrogen_files(s)
end
