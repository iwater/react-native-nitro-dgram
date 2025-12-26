cmake_minimum_required(VERSION 3.9.0)
project(RustCUdp)

set(PACKAGE_NAME RustCUdp)
set(CMAKE_VERBOSE_MAKEFILE ON)
set(CMAKE_CXX_STANDARD 20)

add_library(${PACKAGE_NAME} SHARED
        ../cpp/HybridUdpDriver.cpp
        ../cpp/HybridUdpSocketDriver.cpp
        OnLoad.cpp
)

include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/../cpp
)

include(${CMAKE_CURRENT_SOURCE_DIR}/../nitrogen/generated/android/RustCUdp+autolinking.cmake)

target_link_directories(${PACKAGE_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/libs/${ANDROID_ABI})
target_link_libraries(${PACKAGE_NAME} 
    rust_c_udp
    log
)
