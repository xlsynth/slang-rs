// SPDX-License-Identifier: Apache-2.0

#include "slang-rs/src/native.rs.h"
#include "slang/analysis/AnalysisManager.h"
#include "slang/ast/Compilation.h"
#include "slang/ast/Scope.h"
#include "slang/ast/symbols/BlockSymbols.h"
#include "slang/ast/symbols/CompilationUnitSymbols.h"
#include "slang/ast/symbols/InstanceSymbols.h"
#include "slang/ast/symbols/ParameterSymbols.h"
#include "slang/ast/symbols/PortSymbols.h"
#include "slang/ast/symbols/VariableSymbols.h"
#include "slang/ast/types/AllTypes.h"
#include "slang/diagnostics/DiagnosticClient.h"
#include "slang/diagnostics/DiagnosticEngine.h"
#include "slang/diagnostics/LexerDiags.h"
#include "slang/diagnostics/PreprocessorDiags.h"
#include "slang/driver/CompatSettings.h"
#include "slang/driver/SourceLoader.h"
#include "slang/numeric/Time.h"
#include "slang/parsing/Lexer.h"
#include "slang/parsing/Parser.h"
#include "slang/parsing/Preprocessor.h"
#include "slang/syntax/SyntaxTree.h"
#include "slang/text/CharInfo.h"
#include "slang/text/SourceManager.h"
#include "slang/util/ThreadPool.h"

#include <algorithm>
#include <limits>
#include <stdexcept>
#include <string>
#include <unordered_set>
#include <vector>

namespace slang_rs {
using namespace slang;
using namespace slang::ast;

namespace {

rust::String rustString(std::string_view value) {
    return rust::String(value.data(), value.size());
}

NativeSeverity snapshotSeverity(DiagnosticSeverity severity) {
    switch (severity) {
        case DiagnosticSeverity::Ignored: return NativeSeverity::Ignored;
        case DiagnosticSeverity::Note: return NativeSeverity::Note;
        case DiagnosticSeverity::Warning: return NativeSeverity::Warning;
        case DiagnosticSeverity::Error: return NativeSeverity::Error;
        case DiagnosticSeverity::Fatal: return NativeSeverity::Fatal;
    }
    throw std::logic_error("Unknown diagnostic severity");
}

DiagnosticSeverity diagnosticSeverity(NativeSeverity severity) {
    switch (severity) {
        case NativeSeverity::Ignored: return DiagnosticSeverity::Ignored;
        case NativeSeverity::Note: return DiagnosticSeverity::Note;
        case NativeSeverity::Warning: return DiagnosticSeverity::Warning;
        case NativeSeverity::Error: return DiagnosticSeverity::Error;
        case NativeSeverity::Fatal: return DiagnosticSeverity::Fatal;
        default: throw std::logic_error("Unknown diagnostic severity");
    }
}

LanguageVersion languageVersion(NativeLanguageVersion version) {
    switch (version) {
        case NativeLanguageVersion::Verilog2005: return LanguageVersion::v1364_2005;
        case NativeLanguageVersion::SystemVerilog2017: return LanguageVersion::v1800_2017;
        case NativeLanguageVersion::SystemVerilog2023: return LanguageVersion::v1800_2023;
        default: throw std::logic_error("Unknown language version");
    }
}

TimeScaleValue timeScaleValue(const NativeTimeScaleValue& value) {
    TimeUnit unit;
    switch (value.unit) {
        case NativeTimeUnit::Seconds: unit = TimeUnit::Seconds; break;
        case NativeTimeUnit::Milliseconds: unit = TimeUnit::Milliseconds; break;
        case NativeTimeUnit::Microseconds: unit = TimeUnit::Microseconds; break;
        case NativeTimeUnit::Nanoseconds: unit = TimeUnit::Nanoseconds; break;
        case NativeTimeUnit::Picoseconds: unit = TimeUnit::Picoseconds; break;
        case NativeTimeUnit::Femtoseconds: unit = TimeUnit::Femtoseconds; break;
        default: throw std::logic_error("Unknown time unit");
    }
    TimeScaleMagnitude magnitude;
    switch (value.magnitude) {
        case NativeTimeScaleMagnitude::One: magnitude = TimeScaleMagnitude::One; break;
        case NativeTimeScaleMagnitude::Ten: magnitude = TimeScaleMagnitude::Ten; break;
        case NativeTimeScaleMagnitude::Hundred: magnitude = TimeScaleMagnitude::Hundred; break;
        default: throw std::logic_error("Unknown timescale magnitude");
    }
    return {unit, magnitude};
}

class DiagnosticCollector final : public DiagnosticClient {
public:
    rust::Vec<NativeDiagnostic> diagnostics;

    void report(const ReportedDiagnostic& diagnostic) override {
        append(snapshotSeverity(diagnostic.severity),
               std::string(toString(diagnostic.originalDiagnostic.code)),
               std::string(diagnostic.formattedMessage), diagnostic.location);
    }

    void append(NativeSeverity severity, std::string code, std::string message,
                SourceLocation location = {}) {
        NativeDiagnostic diagnostic{};
        diagnostic.severity = severity;
        diagnostic.code = code;
        // File inputs may contain bytes that are not UTF-8. Preserve the
        // diagnostic even if its human-readable text needs replacement chars.
        diagnostic.message = rust::String::lossy(message);
        if (location && location != SourceLocation::NoLocation) {
            location = sourceManager->getFullyExpandedLoc(location);
            diagnostic.has_location = true;
            diagnostic.file = rust::String::lossy(getFileName(location));
            diagnostic.line = sourceManager->getLineNumber(location);
            diagnostic.column = getColumnNumber(location);
            const auto line = getSourceLine(location, diagnostic.column);
            diagnostic.source_line = rust::String::lossy(line.data(), line.size());
        }
        diagnostics.push_back(std::move(diagnostic));
    }
};

// A user-facing unsupported feature is distinct from a C++ library failure.
class Unsupported final : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

NativeInteger snapshotInteger(const SVInt& integer) {
    NativeInteger result{};
    result.width = integer.getBitWidth();
    result.is_signed = integer.isSigned();
    const size_t wordCount = (size_t(result.width) + 63) / 64;
    const auto* words = integer.getRawPtr();
    result.words.reserve(wordCount);
    result.x_mask.reserve(wordCount);
    result.z_mask.reserve(wordCount);
    for (size_t index = 0; index < wordCount; index++) {
        // Slang stores value words followed by unknown words: an unknown bit
        // with value zero is X, and an unknown bit with value one is Z.
        const uint64_t unknown = integer.hasUnknown() ? words[wordCount + index] : 0;
        result.words.push_back(words[index] & ~unknown);
        result.x_mask.push_back(~words[index] & unknown);
        result.z_mask.push_back(words[index] & unknown);
    }
    return result;
}

// Copy evaluated values into an owned arena without formatting or reevaluating
// them. Compound values refer to other entries by index across the FFI boundary.
size_t appendValue(const ConstantValue& value, rust::Vec<ValueNode>& nodes, size_t depth = 0) {
    if (depth > 256)
        throw Unsupported("Constant value nesting exceeds 256 levels");

    ValueNode result{};
    if (value.bad()) {
        result.kind = ValueKind::Invalid;
    }
    else if (value.isInteger()) {
        result.kind = ValueKind::Integer;
        result.integer = snapshotInteger(value.integer());
    }
    else if (value.isReal()) {
        result.kind = ValueKind::Real;
        result.real = value.real().v;
    }
    else if (value.isShortReal()) {
        result.kind = ValueKind::ShortReal;
        result.short_real = value.shortReal().v;
    }
    else if (value.isString()) {
        result.kind = ValueKind::String;
        const auto& bytes = value.str();
        result.bytes.reserve(bytes.size());
        for (const unsigned char byte : bytes)
            result.bytes.push_back(byte);
    }
    else if (value.isUnpacked()) {
        result.kind = ValueKind::Elements;
        for (const auto& element : value.elements())
            result.elements.push_back(appendValue(element, nodes, depth + 1));
    }
    else if (value.isMap()) {
        result.kind = ValueKind::Map;
        const auto& array = *value.map();
        for (const auto& [key, element] : array) {
            const auto keyIndex = appendValue(key, nodes, depth + 1);
            const auto valueIndex = appendValue(element, nodes, depth + 1);
            result.entries.push_back(NativeMapEntry{keyIndex, valueIndex});
        }
        result.default_value = appendValue(array.defaultValue, nodes, depth + 1);
    }
    else if (value.isQueue()) {
        result.kind = ValueKind::Queue;
        const auto& queue = *value.queue();
        result.max_bound = queue.maxBound;
        for (const auto& element : queue)
            result.elements.push_back(appendValue(element, nodes, depth + 1));
    }
    else if (value.isUnion()) {
        result.kind = ValueKind::Union;
        const auto& unionValue = *value.unionVal();
        result.elements.push_back(appendValue(unionValue.value, nodes, depth + 1));
        result.has_active_member = unionValue.activeMember.has_value();
        result.active_member = unionValue.activeMember.value_or(0);
    }
    else if (value.isNullHandle()) {
        result.kind = ValueKind::Null;
    }
    else if (value.isUnbounded()) {
        result.kind = ValueKind::Unbounded;
    }
    else {
        throw Unsupported("Unsupported constant value kind");
    }
    const size_t index = nodes.size();
    nodes.push_back(std::move(result));
    return index;
}

// Flatten semantic types, dimensions and fields into an owned arena.
size_t appendType(const Type& original, rust::Vec<TypeNode>& nodes, size_t depth = 0) {
    if (depth > 256)
        throw Unsupported("Type nesting exceeds 256 levels");

    TypeNode result{};
    result.bit_width = original.getBitWidth();
    const Type* current = &original;
    std::string name;
    size_t wrappers = 0;
    while (true) {
        if (++wrappers > 512)
            throw Unsupported("Type alias or array nesting exceeds 512 levels");
        if (current->kind == SymbolKind::TypeAlias) {
            if (name.empty())
                name = current->getLexicalPath();
            current = &current->as<TypeAliasType>().targetType.getType();
        }
        else if (current->kind == SymbolKind::PackedArrayType) {
            const auto& array = current->as<PackedArrayType>();
            result.packed.push_back(Dimension{array.range.left, array.range.right});
            current = &array.elementType;
        }
        else if (current->kind == SymbolKind::FixedSizeUnpackedArrayType) {
            const auto& array = current->as<FixedSizeUnpackedArrayType>();
            result.unpacked.push_back(Dimension{array.range.left, array.range.right});
            current = &array.elementType;
        }
        else {
            break;
        }
    }

    const auto& type = current->getCanonicalType();
    result.is_signed = type.isSigned();
    result.four_state = type.isFourState();
    auto fields = [&](const Scope& scope) {
        for (const auto& field : scope.membersOfType<FieldSymbol>()) {
            auto fieldType = appendType(field.getType(), nodes, depth + 1);
            result.fields.push_back(NativeField{rustString(field.name), fieldType});
        }
    };
    switch (type.kind) {
        case SymbolKind::ScalarType:
            result.kind = TypeKind::Integral;
            switch (type.as<ScalarType>().scalarKind) {
                case ScalarType::Bit: result.primitive = IntegralKind::Bit; break;
                case ScalarType::Logic: result.primitive = IntegralKind::Logic; break;
                case ScalarType::Reg: result.primitive = IntegralKind::Reg; break;
            }
            break;
        case SymbolKind::PredefinedIntegerType:
            result.kind = TypeKind::Integral;
            switch (type.as<PredefinedIntegerType>().integerKind) {
                case PredefinedIntegerType::Byte: result.primitive = IntegralKind::Byte; break;
                case PredefinedIntegerType::ShortInt: result.primitive = IntegralKind::ShortInt; break;
                case PredefinedIntegerType::Int: result.primitive = IntegralKind::Int; break;
                case PredefinedIntegerType::LongInt: result.primitive = IntegralKind::LongInt; break;
                case PredefinedIntegerType::Integer: result.primitive = IntegralKind::Integer; break;
                case PredefinedIntegerType::Time: result.primitive = IntegralKind::Time; break;
            }
            break;
        case SymbolKind::PackedStructType: {
            const auto& structure = type.as<PackedStructType>();
            result.kind = TypeKind::Struct;
            result.is_packed = true;
            fields(structure);
            break;
        }
        case SymbolKind::UnpackedStructType: {
            const auto& structure = type.as<UnpackedStructType>();
            result.kind = TypeKind::Struct;
            fields(structure);
            break;
        }
        case SymbolKind::PackedUnionType: {
            const auto& unionType = type.as<PackedUnionType>();
            if (unionType.isTagged)
                throw Unsupported("Tagged unions are not supported");
            result.kind = TypeKind::Union;
            result.is_packed = true;
            fields(unionType);
            break;
        }
        case SymbolKind::UnpackedUnionType: {
            const auto& unionType = type.as<UnpackedUnionType>();
            if (unionType.isTagged)
                throw Unsupported("Tagged unions are not supported");
            result.kind = TypeKind::Union;
            fields(unionType);
            break;
        }
        case SymbolKind::EnumType: {
            const auto& enumeration = type.as<EnumType>();
            result.kind = TypeKind::Enum;
            result.base_type = appendType(enumeration.baseType, nodes, depth + 1);
            for (const auto& variant : enumeration.values()) {
                const auto& value = variant.getValue();
                if (!value.isInteger())
                    throw Unsupported("Enum value is not an integer");
                result.variants.push_back(NativeVariant{rustString(variant.name),
                                                        snapshotInteger(value.integer())});
            }
            break;
        }
        default:
            throw Unsupported("Unsupported type kind: " + std::string(toString(type.kind)));
    }
    result.name = name;
    const size_t index = nodes.size();
    nodes.push_back(std::move(result));
    return index;
}

uint8_t portDirection(ArgumentDirection direction) {
    switch (direction) {
        case ArgumentDirection::In: return 0;
        case ArgumentDirection::Out: return 1;
        case ArgumentDirection::InOut: return 2;
        default: throw Unsupported("Reference ports are not supported");
    }
}

void appendScopeHierarchy(const Scope& scope, int64_t parent,
                          rust::Vec<HierarchyNode>& nodes, size_t depth);

void appendInstance(const InstanceSymbol& instance, int64_t parent,
                    rust::Vec<HierarchyNode>& nodes, size_t depth) {
    const auto index = int64_t(nodes.size());
    nodes.push_back(HierarchyNode{rustString(instance.getDefinition().name),
                                  rustString(instance.getArrayName()),
                                  rust::String(instance.getHierarchicalPath()), parent});
    appendScopeHierarchy(instance.body, index, nodes, depth + 1);
}

void appendInstanceArray(const InstanceArraySymbol& array, int64_t parent,
                         rust::Vec<HierarchyNode>& nodes, size_t depth) {
    if (depth > 4096)
        throw std::runtime_error("Hierarchy nesting exceeds 4096 levels");
    for (const auto* symbol : array.elements) {
        const auto& element = *symbol;
        if (element.kind == SymbolKind::Instance)
            appendInstance(element.as<InstanceSymbol>(), parent, nodes, depth);
        else if (element.kind == SymbolKind::InstanceArray)
            appendInstanceArray(element.as<InstanceArraySymbol>(), parent, nodes, depth + 1);
    }
}

void appendScopeHierarchy(const Scope& scope, int64_t parent,
                          rust::Vec<HierarchyNode>& nodes, size_t depth) {
    if (depth > 4096)
        throw std::runtime_error("Hierarchy nesting exceeds 4096 levels");
    for (const auto& member : scope.members()) {
        switch (member.kind) {
            case SymbolKind::Instance:
                appendInstance(member.as<InstanceSymbol>(), parent, nodes, depth);
                break;
            case SymbolKind::InstanceArray:
                appendInstanceArray(member.as<InstanceArraySymbol>(), parent, nodes, depth);
                break;
            case SymbolKind::UninstantiatedDef: {
                const auto& unknown = member.as<UninstantiatedDefSymbol>();
                nodes.push_back(HierarchyNode{rustString(unknown.definitionName),
                    rustString(unknown.name), rust::String(unknown.getHierarchicalPath()), parent});
                break;
            }
            case SymbolKind::GenerateBlock: {
                const auto& block = member.as<GenerateBlockSymbol>();
                if (!block.isUninstantiated)
                    appendScopeHierarchy(block, parent, nodes, depth + 1);
                break;
            }
            case SymbolKind::GenerateBlockArray: {
                const auto& array = member.as<GenerateBlockArraySymbol>();
                for (const auto* block : array.entries) {
                    if (block->isUninstantiated) continue;
                    appendScopeHierarchy(*block, parent, nodes, depth + 1);
                }
                break;
            }
            default: break;
        }
    }
}

} // namespace

struct NativeCompilation::Impl {
    // Native option structures borrow string_views from this owned config.
    NativeConfig config;
    SourceManager sourceManager;
    driver::SourceLoader sourceLoader;
    DiagnosticEngine diagEngine;
    std::shared_ptr<DiagnosticCollector> diagnostics;
    std::shared_ptr<ThreadPool> threadPool;
    driver::SourceLoader::SyntaxTreeList syntaxTrees;
    std::unique_ptr<Compilation> compilation;
    bool valid = false;

    explicit Impl(NativeConfig config) : config(std::move(config)),
        sourceLoader(sourceManager), diagEngine(sourceManager),
        diagnostics(std::make_shared<DiagnosticCollector>()) {
        diagEngine.addClient(diagnostics);
    }
};

NativeCompilation::NativeCompilation(NativeConfig config) :
    impl(std::make_unique<Impl>(std::move(config))) {}
NativeCompilation::~NativeCompilation() = default;

std::unique_ptr<NativeCompilation> create_compilation(NativeConfig input) {
    auto owner = std::make_unique<NativeCompilation>(std::move(input));
    auto& state = *owner->impl;
    const auto& config = state.config;
    auto& loader = state.sourceLoader;
    auto& engine = state.diagEngine;
    auto error = [&](std::string message, std::string code = "ConfigError") {
        state.diagnostics->append(NativeSeverity::Error, std::move(code), std::move(message));
    };
    auto view = [](const rust::String& value) {
        return std::string_view(value.data(), value.size());
    };

    if (config.libraries_inherit_macros && !config.single_unit) {
        error("libraries_inherit_macros requires single_unit");
        return owner;
    }
    if (config.diagnostics.error_limit > uint32_t(std::numeric_limits<int>::max())) {
        error("diagnostics.error_limit exceeds the native diagnostic limit");
        return owner;
    }
    engine.setErrorLimit(int(config.diagnostics.error_limit));
    engine.setIgnoreAllWarnings(config.diagnostics.warnings != NativeWarningPolicy::All);
    engine.setWarningsAsErrors(config.diagnostics.warnings_as_errors);
    if (config.diagnostics.warnings == NativeWarningPolicy::Default) {
        const auto severity = config.diagnostics.warnings_as_errors ? DiagnosticSeverity::Error
                                                                  : DiagnosticSeverity::Warning;
        for (auto code : engine.findDiagGroup("default")->getDiags())
            engine.setSeverity(code, severity);
    }
    // Keep Slang's default errors for standards violations, independently of
    // which optional warnings are enabled. Explicit overrides below can change them.
    driver::CompatSettings{}.configureDiagnostics(engine);
    if (config.ignore_protected) {
        for (auto code : {
                 diag::ProtectedEnvelope, diag::ExpectedProtectArg, diag::ExpectedProtectKeyword,
                 diag::ExtraProtectEnd, diag::InvalidEncodingByte, diag::InvalidPragmaNumber,
                 diag::InvalidPragmaViewport, diag::NestedProtectBegin, diag::ProtectArgList,
                 diag::ProtectEncodingBytes, diag::RawProtectEOF, diag::UnknownProtectEncoding,
                 diag::UnknownProtectKeyword, diag::UnknownProtectOption}) {
            engine.setSeverity(code, DiagnosticSeverity::Ignored);
        }
    }
    for (const auto& rule : config.diagnostics.overrides) {
        const auto name = view(rule.name);
        const auto severity = diagnosticSeverity(rule.severity);
        if (const auto* group = engine.findDiagGroup(name)) {
            for (auto code : group->getDiags()) engine.setSeverity(code, severity);
        }
        else if (const auto codes = engine.findFromOptionName(name); !codes.empty()) {
            for (auto code : codes) engine.setSeverity(code, severity);
        }
        else {
            error("unknown diagnostic name or group '" + std::string(name) + "'");
            return owner;
        }
    }

    const auto version = languageVersion(config.language_version);
    driver::SourceOptions sources{};
    if (config.num_threads != 0) sources.numThreads = config.num_threads;
    sources.singleUnit = config.single_unit;
    sources.onlyLint = config.lint_only;
    sources.librariesInheritMacros = config.libraries_inherit_macros;

    parsing::PreprocessorOptions preprocessor;
    preprocessor.languageVersion = version;
    for (const auto& define : config.defines)
        preprocessor.predefines.push_back(std::string(view(define.name)) + "=" +
                                          std::string(view(define.value)));

    parsing::LexerOptions lexer;
    lexer.languageVersion = version;
    lexer.enableLegacyProtect = config.enable_legacy_protect;
    if (config.enable_legacy_protect)
        lexer.commentHandlers["pragma"]["protect"] = {parsing::CommentHandler::Protect};
    for (const auto& marker : config.translate_off) {
        for (const auto part : {view(marker.common), view(marker.start), view(marker.end)}) {
            if (part.empty() || !std::all_of(part.begin(), part.end(), [](char c) {
                    return isAlphaNumeric(c) || c == '_';
                })) {
                error("translate_off markers must contain only letters, digits or underscores");
                return owner;
            }
        }
        lexer.commentHandlers[view(marker.common)][view(marker.start)] =
            {parsing::CommentHandler::TranslateOff, view(marker.end)};
    }
    lexer.commentHandlers["slang"]["lint_off"] = {parsing::CommentHandler::LintOff};
    lexer.commentHandlers["slang"]["lint_on"] = {parsing::CommentHandler::LintOn};
    lexer.commentHandlers["slang"]["lint_save"] = {parsing::CommentHandler::LintSave};
    lexer.commentHandlers["slang"]["lint_restore"] = {parsing::CommentHandler::LintRestore};

    parsing::ParserOptions parser;
    parser.languageVersion = version;
    CompilationOptions compilation;
    compilation.languageVersion = version;
    compilation.flags = CompilationFlags::None;
    if (config.ignore_unknown_modules) compilation.flags |= CompilationFlags::IgnoreUnknownModules;
    if (config.lint_only) compilation.flags |= CompilationFlags::LintMode;
    if (config.relax_enum_conversions) compilation.flags |= CompilationFlags::RelaxEnumConversions;
    if (config.allow_use_before_declare) compilation.flags |= CompilationFlags::AllowUseBeforeDeclare;
    if (config.allow_toplevel_interface_ports)
        compilation.flags |= CompilationFlags::AllowTopLevelIfacePorts;
    compilation.errorLimit = config.diagnostics.error_limit;
    for (const auto& top : config.tops) compilation.topModules.emplace(view(top));
    for (const auto& parameter : config.parameters)
        compilation.paramOverrides.push_back(std::string(view(parameter.name)) + "=" +
                                              std::string(view(parameter.value)));
    if (config.has_timescale) {
        const auto base = timeScaleValue(config.timescale.base);
        const auto precision = timeScaleValue(config.timescale.precision);
        if (precision > base) {
            error("timescale precision must not be coarser than its base");
            return owner;
        }
        compilation.defaultTimeScale = TimeScale(base, precision);
    }
    const Bag options(sources, preprocessor, lexer, parser, compilation);

    for (const auto& directory : config.incdirs) {
        if (const auto ec = state.sourceManager.addUserDirectories(view(directory))) {
            error("include directory '" + std::string(view(directory)) + "': " + ec.message(),
                  "SourceLoadError");
            return owner;
        }
    }
    for (const auto& directory : config.libdirs) loader.addSearchDirectories(view(directory));
    for (const auto& extension : config.libexts) loader.addSearchExtension(view(extension));
    for (const auto& prefix : config.dir_prefixes) loader.addDirPrefix(view(prefix));
    std::unordered_set<std::string_view> excludedExtensions;
    for (const auto& extension : config.exclude_extensions)
        excludedExtensions.emplace(view(extension));
    for (const auto& source : config.sources) {
        switch (source.kind) {
            case NativeSourceKind::File: {
                const auto path = view(source.name);
                if (const auto dot = path.find_last_of('.'); dot != std::string_view::npos) {
                    if (excludedExtensions.contains(path.substr(dot + 1)))
                        continue;
                }
                loader.addFiles(path);
                break;
            }
            case NativeSourceKind::Text:
                loader.addBuffer(state.sourceManager.assignText(view(source.name), view(source.text)));
                break;
            default:
                error("unknown source input kind");
                return owner;
        }
    }
    for (const auto& file : config.libfiles) loader.addLibraryFiles({}, view(file));

    auto loadFailed = [&] {
        for (const auto& message : loader.getErrors()) error(message, "SourceLoadError");
        return !loader.getErrors().empty();
    };
    if (loadFailed()) return owner;
    if (!loader.hasFiles()) {
        error("no input files", "SourceLoadError");
        return owner;
    }
    if (config.num_threads != 1)
        state.threadPool = std::make_shared<ThreadPool>(config.num_threads);
    state.syntaxTrees = loader.loadAndParseSources(options, state.threadPool.get());
    if (loadFailed()) return owner;
    engine.issue(engine.setMappingsFromPragmas());

    auto* defaultLibrary = loader.getOrAddLibrary("work");
    defaultLibrary->isDefault = true;
    state.compilation = std::make_unique<Compilation>(options, defaultLibrary);
    for (auto& tree : state.syntaxTrees) state.compilation->addSyntaxTree(tree);
    engine.issue(state.compilation->getAllDiagnostics());
    if (config.analysis_enabled && !config.lint_only) {
        state.compilation->freeze();
        analysis::AnalysisOptions analysisOptions;
        analysisOptions.flags |= analysis::AnalysisFlags::CheckUnused |
                                 analysis::AnalysisFlags::CheckShadow;
        analysis::AnalysisManager analysis(analysisOptions, state.threadPool);
        analysis.analyze(*state.compilation);
        engine.issue(analysis.getDiagnostics());
        state.compilation->unfreeze();
    }
    state.valid = engine.getNumErrors() == 0;
    return owner;
}

bool compilation_valid(const NativeCompilation& compilation) noexcept {
    return compilation.impl->valid;
}

rust::Vec<NativeDiagnostic> take_compilation_diagnostics(NativeCompilation& compilation) {
    return std::move(compilation.impl->diagnostics->diagnostics);
}

InterfaceSnapshot compilation_interfaces(const NativeCompilation& owner, bool parameters,
                                         bool skip_unsupported) {
    InterfaceSnapshot snapshot{};
    DiagnosticCollector diagnostics;
    diagnostics.setEngine(owner.impl->diagEngine);
    auto unsupported = [&](const Symbol& symbol, const Unsupported& error) {
        if (!skip_unsupported)
            diagnostics.append(NativeSeverity::Error, "UnsupportedInterface",
                               symbol.getHierarchicalPath() + ": " + error.what(), symbol.location);
    };
    for (const auto* instance : owner.impl->compilation->getRoot().topInstances) {
        if (instance->name.empty()) continue;
        NativeModule module{};
        module.name = rustString(instance->name);
        if (parameters) {
            for (const auto& parameter : instance->body.membersOfType<ParameterSymbol>()) {
                try {
                    auto index = appendType(parameter.getType(), snapshot.types);
                    module.parameters.push_back(NativeParameter{rustString(parameter.name), index});
                }
                catch (const Unsupported& error) { unsupported(parameter, error); }
            }
        }
        else {
            for (const auto* symbol : instance->body.getPortList()) {
                try {
                    if (symbol->kind == SymbolKind::InterfacePort)
                        throw Unsupported("Interface ports are not currently supported.");
                    if (symbol->kind != SymbolKind::Port)
                        throw Unsupported("Unsupported port kind: " + std::string(toString(symbol->kind)));
                    const auto& port = symbol->as<PortSymbol>();
                    auto direction = portDirection(port.direction);
                    auto index = appendType(port.getType(), snapshot.types);
                    module.ports.push_back(NativePort{rustString(port.name), direction, index});
                }
                catch (const Unsupported& error) { unsupported(*symbol, error); }
            }
        }
        snapshot.modules.push_back(std::move(module));
    }
    snapshot.diagnostics = std::move(diagnostics.diagnostics);
    return snapshot;
}

rust::Vec<rust::String> compilation_modules(const NativeCompilation& owner) {
    rust::Vec<rust::String> modules;
    for (const auto* symbol : owner.impl->compilation->getDefinitions()) {
        if (symbol->kind == SymbolKind::Definition &&
            symbol->as<DefinitionSymbol>().definitionKind == DefinitionKind::Module)
            modules.push_back(rustString(symbol->name));
    }
    return modules;
}

rust::Vec<HierarchyNode> compilation_hierarchy(const NativeCompilation& owner) {
    rust::Vec<HierarchyNode> result;
    for (const auto* instance : owner.impl->compilation->getRoot().topInstances)
        appendInstance(*instance, -1, result, 0);
    return result;
}

PackageSnapshot compilation_packages(const NativeCompilation& owner) {
    PackageSnapshot snapshot{};
    for (const auto* package : owner.impl->compilation->getPackages()) {
        if (package == &owner.impl->compilation->getStdPackage()) continue;
        NativePackage output{};
        output.name = rustString(package->name);
        for (const auto& member : package->members()) {
            if (member.kind == SymbolKind::Parameter) {
                const auto& value = member.as<ParameterSymbol>().getValue();
                output.values.push_back(NativePackageValue{rustString(member.name),
                    appendValue(value, snapshot.values)});
            }
            else if (member.kind == SymbolKind::TypeAlias) {
                NativePackageType type{};
                type.name = rustString(member.name);
                try {
                    type.ty = appendType(member.as<TypeAliasType>(), snapshot.types);
                }
                catch (const Unsupported& error) {
                    DiagnosticCollector diagnostics;
                    diagnostics.setEngine(owner.impl->diagEngine);
                    diagnostics.append(NativeSeverity::Error, "UnsupportedPackageType",
                                       member.getLexicalPath() + ": " + error.what(), member.location);
                    type.diagnostics = std::move(diagnostics.diagnostics);
                }
                output.types.push_back(std::move(type));
            }
        }
        snapshot.packages.push_back(std::move(output));
    }
    return snapshot;
}
} // namespace slang_rs
