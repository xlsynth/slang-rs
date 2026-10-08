// SPDX-License-Identifier: Apache-2.0
#pragma once

#include "rust/cxx.h"
#include <memory>

namespace slang_rs {
struct NativeDiagnostic;
struct NativeConfig;
struct InterfaceSnapshot;
struct HierarchyNode;
struct PackageSnapshot;

// The implementation owns configuration, source buffers, and syntax trees for
// longer than the semantic compilation and all of its AST symbols.
class NativeCompilation {
public:
    struct Impl;
    std::unique_ptr<Impl> impl;
    explicit NativeCompilation(NativeConfig config);
    ~NativeCompilation();
    NativeCompilation(const NativeCompilation&) = delete;
    NativeCompilation& operator=(const NativeCompilation&) = delete;
};

std::unique_ptr<NativeCompilation> create_compilation(NativeConfig config);
bool compilation_valid(const NativeCompilation& compilation) noexcept;
rust::Vec<NativeDiagnostic> take_compilation_diagnostics(NativeCompilation& compilation);
InterfaceSnapshot compilation_interfaces(const NativeCompilation& compilation, bool parameters,
                                         bool skip_unsupported);
rust::Vec<rust::String> compilation_modules(const NativeCompilation& compilation);
rust::Vec<HierarchyNode> compilation_hierarchy(const NativeCompilation& compilation);
PackageSnapshot compilation_packages(const NativeCompilation& compilation);
} // namespace slang_rs
