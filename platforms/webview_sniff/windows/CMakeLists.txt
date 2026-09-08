# The Flutter tooling requires that developers have a version of Visual Studio
# installed that includes CMake 3.14 or later. You should not increase this
# version, as doing so will cause the plugin to fail to compile for some
# customers of the plugin.
cmake_minimum_required(VERSION 3.14)

set(PROJECT_NAME "webview_sniff")
project(${PROJECT_NAME} LANGUAGES CXX)

cmake_policy(VERSION 3.14...3.25)

set(PLUGIN_NAME "webview_sniff_plugin")
set(WEBVIEW2_VERSION "1.0.2849.39")
set(WEBVIEW2_PACKAGE_DIR "${CMAKE_CURRENT_BINARY_DIR}/packages/Microsoft.Web.WebView2.${WEBVIEW2_VERSION}")

if(NOT EXISTS "${WEBVIEW2_PACKAGE_DIR}/build/native/include/WebView2.h")
  set(WEBVIEW2_ZIP "${CMAKE_CURRENT_BINARY_DIR}/webview2.zip")
  file(DOWNLOAD
    "https://www.nuget.org/api/v2/package/Microsoft.Web.WebView2/${WEBVIEW2_VERSION}"
    "${WEBVIEW2_ZIP}"
    STATUS download_status
  )
  list(GET download_status 0 status_code)
  if(NOT status_code EQUAL 0)
    list(GET download_status 1 status_message)
    message(FATAL_ERROR "WebView2 SDK download failed: ${status_message}")
  endif()
  file(MAKE_DIRECTORY "${WEBVIEW2_PACKAGE_DIR}")
  execute_process(
    COMMAND ${CMAKE_COMMAND} -E tar xzf "${WEBVIEW2_ZIP}"
    WORKING_DIRECTORY "${WEBVIEW2_PACKAGE_DIR}"
    RESULT_VARIABLE extract_result
  )
  if(NOT extract_result EQUAL 0)
    message(FATAL_ERROR "WebView2 SDK extract failed")
  endif()
endif()

if(CMAKE_SYSTEM_PROCESSOR STREQUAL "ARM64" OR CMAKE_GENERATOR_PLATFORM STREQUAL "ARM64")
  set(WEBVIEW2_ARCH "arm64")
elseif(CMAKE_SIZEOF_VOID_P EQUAL 8)
  set(WEBVIEW2_ARCH "x64")
else()
  set(WEBVIEW2_ARCH "x86")
endif()

list(APPEND PLUGIN_SOURCES
  "webview_sniff_plugin.cpp"
  "webview_sniff_plugin.h"
)

add_library(${PLUGIN_NAME} SHARED
  "include/webview_sniff/webview_sniff_plugin_c_api.h"
  "webview_sniff_plugin_c_api.cpp"
  ${PLUGIN_SOURCES}
)

apply_standard_settings(${PLUGIN_NAME})

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden)
target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)
target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_include_directories(${PLUGIN_NAME} PRIVATE
  "${WEBVIEW2_PACKAGE_DIR}/build/native/include")
target_link_libraries(${PLUGIN_NAME} PRIVATE
  flutter
  flutter_wrapper_plugin
  "${WEBVIEW2_PACKAGE_DIR}/build/native/${WEBVIEW2_ARCH}/WebView2LoaderStatic.lib"
  version
)

set(webview_sniff_bundled_libraries
  ""
  PARENT_SCOPE
)
