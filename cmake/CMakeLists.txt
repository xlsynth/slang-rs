# SPDX-License-Identifier: Apache-2.0

cmake_minimum_required(VERSION 3.20...3.29)
project(slang_rs_native LANGUAGES CXX)

# This is the native recipe for both Cargo and tools/native.py. Keep it free of
# Rust or bridge settings: an installation remains reusable across Rust-only
# releases. These options are fixed because the bridge links static libraries
# and must use the matching Slang compile definitions.
set(CMAKE_BUILD_TYPE Release CACHE STRING "Native build type" FORCE)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(CMAKE_INSTALL_LIBDIR lib CACHE PATH "Native library directory" FORCE)
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(SLANG_INCLUDE_INSTALL ON CACHE BOOL "" FORCE)
set(SLANG_INCLUDE_TOOLS OFF CACHE BOOL "" FORCE)
set(SLANG_INCLUDE_TESTS OFF CACHE BOOL "" FORCE)
set(SLANG_INCLUDE_DOCS OFF CACHE BOOL "" FORCE)
set(SLANG_INCLUDE_PYLIB OFF CACHE BOOL "" FORCE)
set(SLANG_INCLUDE_PYTHON_DOCS OFF CACHE BOOL "" FORCE)
set(SLANG_USE_MIMALLOC OFF CACHE BOOL "" FORCE)
set(SLANG_USE_CPPTRACE OFF CACHE BOOL "" FORCE)
set(SLANG_USE_THREADS ON CACHE BOOL "" FORCE)
set(FMT_DOC OFF CACHE BOOL "" FORCE)
set(FMT_TEST OFF CACHE BOOL "" FORCE)
set(FMT_INSTALL ON CACHE BOOL "" FORCE)
# Always use the pinned fmt and Slang's bundled Boost headers, even on systems
# with a different version already installed.
set(FETCHCONTENT_TRY_FIND_PACKAGE_MODE NEVER)
set(CMAKE_DISABLE_FIND_PACKAGE_Boost ON)

include(FetchContent)

# The first declaration wins: Slang's own CMake code will make fmt available
# using this immutable, checksum-verified source instead of its release tag.
# fmt 12.1.0
FetchContent_Declare(
  fmt
  URL https://codeload.github.com/fmtlib/fmt/tar.gz/407c905e45ad75fc29bf0f9bb7c5c2fd3475976f
  URL_HASH SHA256=2bc1fe4a5b6d5d6a614239b4ca1d520e66e152a02d3d262684d26dfd6ab3438a
  TLS_VERIFY TRUE)

# Slang 11.0
FetchContent_Declare(
  slang
  URL https://codeload.github.com/MikePopoloski/slang/tar.gz/7ddf4059f79eff508dd486eb42fd650cdf320d52
  URL_HASH SHA256=1b24be639b4588c5dea5d7bcc60973b999ffa20eb437a39d5369aedc0c7aefc5
  TLS_VERIFY TRUE)

FetchContent_MakeAvailable(slang)
FetchContent_GetProperties(fmt)

# A prefix records its native source pins and options, independent of its
# installation path and of the slang-rs / cxx crate version.
file(SHA256 "${CMAKE_CURRENT_LIST_FILE}" native_recipe_sha256)
get_directory_property(native_slang_version
  DIRECTORY "${slang_SOURCE_DIR}" DEFINITION SLANG_VERSION_STRING)
get_directory_property(native_fmt_version
  DIRECTORY "${fmt_SOURCE_DIR}" DEFINITION FMT_VERSION)
set(native_processor "${CMAKE_SYSTEM_PROCESSOR}")
if(APPLE AND CMAKE_OSX_ARCHITECTURES)
  set(native_processor "${CMAKE_OSX_ARCHITECTURES}")
endif()
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/manifest.txt"
  "native_recipe_sha256=${native_recipe_sha256}\n"
  "slang_version=${native_slang_version}\n"
  "fmt_version=${native_fmt_version}\n"
  "system=${CMAKE_SYSTEM_NAME}\n"
  "processor=${native_processor}\n"
  "cxx_compiler_id=${CMAKE_CXX_COMPILER_ID}\n"
  "cxx_compiler_version=${CMAKE_CXX_COMPILER_VERSION}\n")
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/manifest.txt"
  DESTINATION share)
install(FILES "${slang_SOURCE_DIR}/LICENSE"
  DESTINATION share/licenses RENAME slang-LICENSE)
install(FILES "${fmt_SOURCE_DIR}/LICENSE"
  DESTINATION share/licenses RENAME fmt-LICENSE)
install(FILES "${slang_SOURCE_DIR}/LICENSES/BSL-1.0.txt"
  DESTINATION share/licenses RENAME boost-LICENSE)
