// SPDX-License-Identifier: Apache-2.0

//! Owned snapshots from an in-process Slang compilation.

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::{
    ConstantValue, DiagnosticOptions, DiagnosticSeverity, Field, Instance, IntegerValue,
    IntegralKind, LanguageVersion, Package, Parameter, Port, PortDir, PortList, Range, SlangConfig,
    Source, TimeScale, TimeScaleMagnitude, TimeScaleValue, TimeUnit, Type, Variant, WarningPolicy,
};
use num_bigint::BigUint;

#[cxx::bridge(namespace = "slang_rs")]
mod ffi {
    enum NativeSourceKind {
        File,
        Text,
    }
    struct NativeSource {
        kind: NativeSourceKind,
        name: String,
        text: String,
    }
    struct NativeNameValue {
        name: String,
        value: String,
    }
    enum NativeLanguageVersion {
        Verilog2005,
        SystemVerilog2017,
        SystemVerilog2023,
    }
    enum NativeTimeUnit {
        Seconds,
        Milliseconds,
        Microseconds,
        Nanoseconds,
        Picoseconds,
        Femtoseconds,
    }
    enum NativeTimeScaleMagnitude {
        One,
        Ten,
        Hundred,
    }
    struct NativeTimeScaleValue {
        unit: NativeTimeUnit,
        magnitude: NativeTimeScaleMagnitude,
    }
    struct NativeTimeScale {
        base: NativeTimeScaleValue,
        precision: NativeTimeScaleValue,
    }
    struct NativeTranslateOff {
        common: String,
        start: String,
        end: String,
    }
    enum NativeWarningPolicy {
        Default,
        All,
        None,
    }
    enum NativeSeverity {
        Ignored,
        Note,
        Warning,
        Error,
        Fatal,
    }
    struct NativeDiagnosticOverride {
        name: String,
        severity: NativeSeverity,
    }
    struct NativeDiagnosticOptions {
        warnings: NativeWarningPolicy,
        warnings_as_errors: bool,
        error_limit: u32,
        overrides: Vec<NativeDiagnosticOverride>,
    }
    struct NativeConfig {
        sources: Vec<NativeSource>,
        tops: Vec<String>,
        incdirs: Vec<String>,
        defines: Vec<NativeNameValue>,
        parameters: Vec<NativeNameValue>,
        libfiles: Vec<String>,
        libdirs: Vec<String>,
        libexts: Vec<String>,
        dir_prefixes: Vec<String>,
        exclude_extensions: Vec<String>,
        ignore_unknown_modules: bool,
        ignore_protected: bool,
        enable_legacy_protect: bool,
        timescale: NativeTimeScale,
        has_timescale: bool,
        language_version: NativeLanguageVersion,
        single_unit: bool,
        libraries_inherit_macros: bool,
        analysis_enabled: bool,
        lint_only: bool,
        relax_enum_conversions: bool,
        allow_use_before_declare: bool,
        allow_toplevel_interface_ports: bool,
        num_threads: u32,
        translate_off: Vec<NativeTranslateOff>,
        diagnostics: NativeDiagnosticOptions,
    }
    struct NativeDiagnostic {
        severity: NativeSeverity,
        code: String,
        message: String,
        file: String,
        line: u64,
        column: u64,
        source_line: String,
        has_location: bool,
    }

    struct Dimension {
        left: i32,
        right: i32,
    }
    struct NativeField {
        name: String,
        ty: usize,
    }
    struct NativeInteger {
        width: u32,
        is_signed: bool,
        words: Vec<u64>,
        x_mask: Vec<u64>,
        z_mask: Vec<u64>,
    }
    struct NativeVariant {
        name: String,
        value: NativeInteger,
    }
    enum TypeKind {
        Integral,
        Struct,
        Union,
        Enum,
    }
    enum IntegralKind {
        Bit,
        Logic,
        Reg,
        Byte,
        ShortInt,
        Int,
        LongInt,
        Integer,
        Time,
    }
    struct TypeNode {
        kind: TypeKind,
        primitive: IntegralKind,
        name: String,
        is_signed: bool,
        four_state: bool,
        is_packed: bool,
        bit_width: u64,
        packed: Vec<Dimension>,
        unpacked: Vec<Dimension>,
        fields: Vec<NativeField>,
        variants: Vec<NativeVariant>,
        base_type: usize,
    }
    struct NativePort {
        name: String,
        direction: u8,
        ty: usize,
    }
    struct NativeParameter {
        name: String,
        ty: usize,
    }
    struct NativeModule {
        name: String,
        ports: Vec<NativePort>,
        parameters: Vec<NativeParameter>,
    }
    struct InterfaceSnapshot {
        types: Vec<TypeNode>,
        modules: Vec<NativeModule>,
        diagnostics: Vec<NativeDiagnostic>,
    }
    struct HierarchyNode {
        definition: String,
        instance: String,
        path: String,
        parent: i64,
    }
    struct NativePackageValue {
        name: String,
        value: usize,
    }
    enum ValueKind {
        Invalid,
        Integer,
        Real,
        ShortReal,
        String,
        Elements,
        Map,
        Queue,
        Union,
        Null,
        Unbounded,
    }
    struct NativeMapEntry {
        key: usize,
        value: usize,
    }
    struct ValueNode {
        kind: ValueKind,
        integer: NativeInteger,
        real: f64,
        short_real: f32,
        bytes: Vec<u8>,
        elements: Vec<usize>,
        entries: Vec<NativeMapEntry>,
        default_value: usize,
        max_bound: u32,
        active_member: u32,
        has_active_member: bool,
    }
    struct NativePackageType {
        name: String,
        ty: usize,
        diagnostics: Vec<NativeDiagnostic>,
    }
    struct NativePackage {
        name: String,
        values: Vec<NativePackageValue>,
        types: Vec<NativePackageType>,
    }
    struct PackageSnapshot {
        types: Vec<TypeNode>,
        values: Vec<ValueNode>,
        packages: Vec<NativePackage>,
    }

    unsafe extern "C++" {
        include!("slang-rs/cpp/native.h");
        type NativeCompilation;

        fn create_compilation(config: NativeConfig) -> Result<UniquePtr<NativeCompilation>>;
        fn compilation_valid(compilation: &NativeCompilation) -> bool;
        fn take_compilation_diagnostics(
            compilation: Pin<&mut NativeCompilation>,
        ) -> Result<Vec<NativeDiagnostic>>;
        fn compilation_interfaces(
            compilation: &NativeCompilation,
            parameters: bool,
            skip_unsupported: bool,
        ) -> Result<InterfaceSnapshot>;
        fn compilation_modules(compilation: &NativeCompilation) -> Result<Vec<String>>;
        fn compilation_hierarchy(compilation: &NativeCompilation) -> Result<Vec<HierarchyNode>>;
        fn compilation_packages(compilation: &NativeCompilation) -> Result<PackageSnapshot>;
    }
}

impl From<LanguageVersion> for ffi::NativeLanguageVersion {
    fn from(value: LanguageVersion) -> Self {
        match value {
            LanguageVersion::Verilog2005 => Self::Verilog2005,
            LanguageVersion::SystemVerilog2017 => Self::SystemVerilog2017,
            LanguageVersion::SystemVerilog2023 => Self::SystemVerilog2023,
        }
    }
}

impl From<TimeUnit> for ffi::NativeTimeUnit {
    fn from(value: TimeUnit) -> Self {
        match value {
            TimeUnit::Seconds => Self::Seconds,
            TimeUnit::Milliseconds => Self::Milliseconds,
            TimeUnit::Microseconds => Self::Microseconds,
            TimeUnit::Nanoseconds => Self::Nanoseconds,
            TimeUnit::Picoseconds => Self::Picoseconds,
            TimeUnit::Femtoseconds => Self::Femtoseconds,
        }
    }
}

impl From<TimeScaleMagnitude> for ffi::NativeTimeScaleMagnitude {
    fn from(value: TimeScaleMagnitude) -> Self {
        match value {
            TimeScaleMagnitude::One => Self::One,
            TimeScaleMagnitude::Ten => Self::Ten,
            TimeScaleMagnitude::Hundred => Self::Hundred,
        }
    }
}

impl From<TimeScaleValue> for ffi::NativeTimeScaleValue {
    fn from(value: TimeScaleValue) -> Self {
        Self {
            unit: value.unit.into(),
            magnitude: value.magnitude.into(),
        }
    }
}

impl From<TimeScale> for ffi::NativeTimeScale {
    fn from(value: TimeScale) -> Self {
        Self {
            base: value.base.into(),
            precision: value.precision.into(),
        }
    }
}

impl From<WarningPolicy> for ffi::NativeWarningPolicy {
    fn from(value: WarningPolicy) -> Self {
        match value {
            WarningPolicy::Default => Self::Default,
            WarningPolicy::All => Self::All,
            WarningPolicy::None => Self::None,
        }
    }
}

impl From<DiagnosticSeverity> for ffi::NativeSeverity {
    fn from(value: DiagnosticSeverity) -> Self {
        match value {
            DiagnosticSeverity::Ignored => Self::Ignored,
            DiagnosticSeverity::Note => Self::Note,
            DiagnosticSeverity::Warning => Self::Warning,
            DiagnosticSeverity::Error => Self::Error,
            DiagnosticSeverity::Fatal => Self::Fatal,
        }
    }
}

impl TryFrom<ffi::NativeSeverity> for DiagnosticSeverity {
    type Error = Error;

    fn try_from(value: ffi::NativeSeverity) -> Result<Self, Error> {
        match value {
            ffi::NativeSeverity::Ignored => Ok(Self::Ignored),
            ffi::NativeSeverity::Note => Ok(Self::Note),
            ffi::NativeSeverity::Warning => Ok(Self::Warning),
            ffi::NativeSeverity::Error => Ok(Self::Error),
            ffi::NativeSeverity::Fatal => Ok(Self::Fatal),
            _ => Err(snapshot_error("invalid native diagnostic severity")),
        }
    }
}

impl From<DiagnosticOptions<'_>> for ffi::NativeDiagnosticOptions {
    fn from(value: DiagnosticOptions<'_>) -> Self {
        Self {
            warnings: value.warnings.into(),
            warnings_as_errors: value.warnings_as_errors,
            error_limit: value.error_limit,
            overrides: value
                .overrides
                .iter()
                .map(|item| ffi::NativeDiagnosticOverride {
                    name: item.name.to_owned(),
                    severity: item.severity.into(),
                })
                .collect(),
        }
    }
}

impl From<&SlangConfig<'_>> for ffi::NativeConfig {
    fn from(config: &SlangConfig<'_>) -> Self {
        let strings = |values: &[&str]| values.iter().map(|value| (*value).to_owned()).collect();
        let pairs = |values: &[(&str, &str)]| {
            values
                .iter()
                .map(|(name, value)| ffi::NativeNameValue {
                    name: (*name).to_owned(),
                    value: (*value).to_owned(),
                })
                .collect()
        };
        Self {
            sources: config
                .sources
                .iter()
                .map(|source| match source {
                    Source::File(path) => ffi::NativeSource {
                        kind: ffi::NativeSourceKind::File,
                        name: (*path).to_owned(),
                        text: String::new(),
                    },
                    Source::Text { name, text } => ffi::NativeSource {
                        kind: ffi::NativeSourceKind::Text,
                        name: (*name).to_owned(),
                        text: (*text).to_owned(),
                    },
                })
                .collect(),
            tops: strings(config.tops),
            incdirs: strings(config.incdirs),
            defines: pairs(config.defines),
            parameters: pairs(config.parameters),
            libfiles: strings(config.libfiles),
            libdirs: strings(config.libdirs),
            libexts: strings(config.libexts),
            dir_prefixes: strings(config.dir_prefixes),
            exclude_extensions: strings(config.exclude_extensions),
            ignore_unknown_modules: config.ignore_unknown_modules,
            ignore_protected: config.ignore_protected,
            enable_legacy_protect: config.enable_legacy_protect,
            timescale: config.timescale.unwrap_or_default().into(),
            has_timescale: config.timescale.is_some(),
            language_version: config.language_version.into(),
            single_unit: config.single_unit,
            libraries_inherit_macros: config.libraries_inherit_macros,
            analysis_enabled: config.analysis_enabled,
            lint_only: config.lint_only,
            relax_enum_conversions: config.relax_enum_conversions,
            allow_use_before_declare: config.allow_use_before_declare,
            allow_toplevel_interface_ports: config.allow_toplevel_interface_ports,
            num_threads: config.num_threads.map_or(0, |count| count.get()),
            translate_off: config
                .translate_off
                .iter()
                .map(|item| ffi::NativeTranslateOff {
                    common: item.common.to_owned(),
                    start: item.start.to_owned(),
                    end: item.end.to_owned(),
                })
                .collect(),
            diagnostics: config.diagnostics.into(),
        }
    }
}

/// A diagnostic emitted by Slang, with source coordinates when available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = &self.location {
            write!(
                f,
                "{}:{}:{}: ",
                location.file, location.line, location.column
            )?;
        }
        write!(f, "{}: {}", self.severity, self.message)?;
        if let Some(location) = &self.location {
            if !location.source_line.is_empty() {
                write!(f, "\n{}", location.source_line)?;
            }
        }
        Ok(())
    }
}

/// Source coordinates and the associated line of text for a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: u64,
    pub column: u64,
    pub source_line: String,
}

impl TryFrom<ffi::NativeDiagnostic> for Diagnostic {
    type Error = Error;

    fn try_from(value: ffi::NativeDiagnostic) -> Result<Self, Self::Error> {
        Ok(Self {
            severity: value.severity.try_into()?,
            code: value.code,
            message: value.message,
            location: value.has_location.then_some(SourceLocation {
                file: value.file,
                line: value.line,
                column: value.column,
                source_line: value.source_line,
            }),
        })
    }
}

/// A compilation or extraction failure. No C++ exception escapes into Rust.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        for diagnostic in &self.diagnostics {
            write!(f, "\n{diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<cxx::Exception> for Error {
    fn from(value: cxx::Exception) -> Self {
        Self {
            message: value.what().to_owned(),
            diagnostics: Vec::new(),
        }
    }
}

fn snapshot_error(message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        diagnostics: Vec::new(),
    }
}

/// An in-process Slang compilation that can be queried repeatedly.
///
/// The owner keeps source buffers, syntax trees and semantic symbols alive.
/// Design query results are fully owned Rust values and remain valid after this
/// owner is dropped. Sessions intentionally implement neither `Send` nor `Sync`:
/// Slang may lazily populate internal caches while answering a query.
pub struct Compilation {
    native: cxx::UniquePtr<ffi::NativeCompilation>,
    diagnostics: Vec<Diagnostic>,
    skip_unsupported_ports: bool,
    skip_unsupported_parameters: bool,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

impl fmt::Debug for Compilation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Compilation").finish_non_exhaustive()
    }
}

impl Compilation {
    /// Parse and elaborate the configured sources once using the linked Slang
    /// library. Compiler diagnostics are returned as structured errors.
    pub fn new(config: &SlangConfig<'_>) -> Result<Self, Error> {
        let mut native = ffi::create_compilation(config.into())?;
        let valid = ffi::compilation_valid(&native);
        let diagnostics = ffi::take_compilation_diagnostics(native.pin_mut())?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, Error>>()?;
        if !valid {
            return Err(Error {
                message: "Slang compilation failed".into(),
                diagnostics,
            });
        }
        Ok(Self {
            native,
            diagnostics,
            skip_unsupported_ports: config.skip_unsupported_ports,
            skip_unsupported_parameters: config.skip_unsupported_parameters,
            _not_send_or_sync: PhantomData,
        })
    }

    fn inner(&self) -> &ffi::NativeCompilation {
        self.native
            .as_ref()
            .expect("native compilation owner is non-null")
    }

    /// Diagnostics produced during compilation, including nonfatal warnings.
    ///
    /// These are stable across design queries. Use `.to_vec()` to keep an
    /// owned copy after dropping the compilation.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Ports on elaborated top-level instances, keyed by top-level name.
    ///
    /// Each module's ports preserve Slang's port-list order and support lookup
    /// by name or position. Unsupported ports return an error unless
    /// [`SlangConfig::skip_unsupported_ports`] is enabled.
    /// Skipping ports removes their entries and shifts subsequent indices.
    pub fn ports(&self) -> Result<HashMap<String, PortList>, Error> {
        let snapshot = self.interfaces(false, self.skip_unsupported_ports)?;
        let mut modules = HashMap::new();
        for module in snapshot.modules {
            let mut ports = Vec::new();
            for port in module.ports {
                ports.push(Port {
                    name: port.name,
                    dir: match port.direction {
                        0 => PortDir::Input,
                        1 => PortDir::Output,
                        2 => PortDir::InOut,
                        _ => return Err(snapshot_error("invalid native port direction")),
                    },
                    ty: convert_type(&snapshot.types, port.ty, 0)?,
                });
            }
            if modules.insert(module.name, ports.into()).is_some() {
                return Err(snapshot_error("duplicate top-level module name"));
            }
        }
        Ok(modules)
    }

    /// Names and resolved types of value parameters and localparams on
    /// elaborated top-level instances, keyed by top-level name.
    ///
    /// Values and type parameters are not included. Unsupported types return
    /// an error unless [`SlangConfig::skip_unsupported_parameters`] is enabled.
    pub fn parameters(&self) -> Result<HashMap<String, Vec<Parameter>>, Error> {
        let snapshot = self.interfaces(true, self.skip_unsupported_parameters)?;
        let mut modules = HashMap::new();
        for module in snapshot.modules {
            let parameters = module
                .parameters
                .into_iter()
                .map(|parameter| {
                    Ok(Parameter {
                        name: parameter.name,
                        ty: convert_type(&snapshot.types, parameter.ty, 0)?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            if modules.insert(module.name, parameters).is_some() {
                return Err(snapshot_error("duplicate top-level module name"));
            }
        }
        Ok(modules)
    }

    fn interfaces(
        &self,
        parameters: bool,
        skip_unsupported: bool,
    ) -> Result<ffi::InterfaceSnapshot, Error> {
        let snapshot = ffi::compilation_interfaces(self.inner(), parameters, skip_unsupported)?;
        if !snapshot.diagnostics.is_empty() {
            return Err(Error {
                message: "Unsupported SystemVerilog interface".into(),
                diagnostics: snapshot
                    .diagnostics
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            });
        }
        Ok(snapshot)
    }

    /// Names of module definitions in the configured sources, including
    /// definitions that are not instantiated as a top-level design.
    pub fn modules(&self) -> Result<Vec<String>, Error> {
        Ok(ffi::compilation_modules(self.inner())?)
    }

    /// Elaborated hierarchy, including active generate scopes and unknown
    /// module placeholders when `ignore_unknown_modules` is enabled.
    pub fn hierarchy(&self) -> Result<HashMap<String, Instance>, Error> {
        let nodes = ffi::compilation_hierarchy(self.inner())?;
        let mut instances: Vec<_> = nodes
            .into_iter()
            .map(|node| {
                (
                    node.parent,
                    Some(Instance {
                        def_name: node.definition,
                        inst_name: node.instance,
                        path: node.path,
                        contents: Default::default(),
                    }),
                )
            })
            .collect();
        let mut roots = HashMap::new();
        // Native records are preorder; attach backwards, then restore sibling order.
        for index in (0..instances.len()).rev() {
            let (parent, instance) = &mut instances[index];
            let parent = *parent;
            let mut instance = instance.take().expect("unconsumed hierarchy node");
            instance.contents.reverse();
            if parent < 0 {
                if roots.insert(instance.inst_name.clone(), instance).is_some() {
                    return Err(snapshot_error("duplicate hierarchy root"));
                }
            } else {
                let parent = usize::try_from(parent)
                    .map_err(|_| snapshot_error("invalid hierarchy parent"))?;
                if parent >= index {
                    return Err(snapshot_error("invalid hierarchy order"));
                }
                let parent = instances[parent].1.as_mut().expect("parent precedes child");
                let relative_path = instance
                    .path
                    .strip_prefix(parent.path.as_str())
                    .and_then(|suffix| suffix.strip_prefix('.'))
                    .filter(|suffix| !suffix.is_empty())
                    .ok_or_else(|| snapshot_error("invalid hierarchy child path"))?
                    .to_owned();
                if parent.contents.insert(relative_path, instance).is_some() {
                    return Err(snapshot_error("duplicate hierarchy child path"));
                }
            }
        }
        Ok(roots)
    }

    /// Package value parameters and resolved typedefs.
    ///
    /// Constants are evaluated [`ConstantValue`] snapshots. Typedefs are
    /// resolved [`Type`] values in [`Package::types`], with an individual error
    /// for each unsupported type.
    pub fn packages(&self) -> Result<HashMap<String, Package>, Error> {
        let snapshot = ffi::compilation_packages(self.inner())?;
        snapshot
            .packages
            .into_iter()
            .map(|package| {
                let parameters = package
                    .values
                    .into_iter()
                    .map(|parameter| {
                        Ok((
                            parameter.name,
                            convert_value(&snapshot.values, parameter.value, 0)?,
                        ))
                    })
                    .collect::<Result<_, Error>>()?;
                let types = package
                    .types
                    .into_iter()
                    .map(|alias| {
                        let ty = if alias.diagnostics.is_empty() {
                            convert_type(&snapshot.types, alias.ty, 0)
                        } else {
                            Err(Error {
                                message: format!(
                                    "Unsupported package type {}::{}",
                                    package.name, alias.name
                                ),
                                diagnostics: alias
                                    .diagnostics
                                    .into_iter()
                                    .map(TryInto::try_into)
                                    .collect::<Result<_, _>>()?,
                            })
                        };
                        Ok((alias.name, ty))
                    })
                    .collect::<Result<_, Error>>()?;
                Ok((
                    package.name.clone(),
                    Package {
                        name: package.name,
                        parameters,
                        types,
                    },
                ))
            })
            .collect()
    }
}

fn convert_type(nodes: &[ffi::TypeNode], index: usize, depth: usize) -> Result<Type, Error> {
    if depth > 256 {
        return Err(snapshot_error("native type nesting exceeds 256 levels"));
    }
    let node = nodes
        .get(index)
        .ok_or_else(|| snapshot_error("invalid native type reference"))?;
    let bit_width = if node.bit_width == 0 {
        None
    } else {
        Some(
            usize::try_from(node.bit_width)
                .map_err(|_| snapshot_error("native type width exceeds usize"))?,
        )
    };
    let name = (!node.name.is_empty()).then(|| node.name.clone());
    let packed_dimensions = node
        .packed
        .iter()
        .map(|range| Range {
            left: range.left,
            right: range.right,
        })
        .collect();
    let unpacked_dimensions = node
        .unpacked
        .iter()
        .map(|range| Range {
            left: range.left,
            right: range.right,
        })
        .collect();
    let fields = || {
        node.fields
            .iter()
            .map(|field| {
                Ok(Field {
                    name: field.name.clone(),
                    ty: convert_type(nodes, field.ty, depth + 1)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()
    };
    Ok(match node.kind {
        ffi::TypeKind::Integral => Type::Integral {
            kind: match node.primitive {
                ffi::IntegralKind::Bit => IntegralKind::Bit,
                ffi::IntegralKind::Logic => IntegralKind::Logic,
                ffi::IntegralKind::Reg => IntegralKind::Reg,
                ffi::IntegralKind::Byte => IntegralKind::Byte,
                ffi::IntegralKind::ShortInt => IntegralKind::ShortInt,
                ffi::IntegralKind::Int => IntegralKind::Int,
                ffi::IntegralKind::LongInt => IntegralKind::LongInt,
                ffi::IntegralKind::Integer => IntegralKind::Integer,
                ffi::IntegralKind::Time => IntegralKind::Time,
                _ => return Err(snapshot_error("invalid native integral kind")),
            },
            signed: node.is_signed,
            four_state: node.four_state,
            packed_dimensions,
            unpacked_dimensions,
            bit_width,
        },
        ffi::TypeKind::Struct => Type::Struct {
            name,
            signed: node.is_signed,
            four_state: node.four_state,
            fields: fields()?,
            is_packed: node.is_packed,
            packed_dimensions,
            unpacked_dimensions,
            bit_width,
        },
        ffi::TypeKind::Union => Type::Union {
            name,
            signed: node.is_signed,
            four_state: node.four_state,
            fields: fields()?,
            is_packed: node.is_packed,
            packed_dimensions,
            unpacked_dimensions,
            bit_width,
        },
        ffi::TypeKind::Enum => Type::Enum {
            name,
            signed: node.is_signed,
            four_state: node.four_state,
            base_type: Box::new(convert_type(nodes, node.base_type, depth + 1)?),
            packed_dimensions,
            unpacked_dimensions,
            bit_width,
            variants: node
                .variants
                .iter()
                .map(|variant| Variant {
                    name: variant.name.clone(),
                    value: convert_integer(&variant.value),
                })
                .collect(),
        },
        _ => return Err(snapshot_error("invalid native type kind")),
    })
}

fn convert_integer(value: &ffi::NativeInteger) -> IntegerValue {
    // SVInt words are least significant first, independent of host byte order.
    let from_words = |words: &[u64]| {
        BigUint::new(
            words
                .iter()
                .flat_map(|word| [*word as u32, (word >> 32) as u32])
                .collect(),
        )
    };
    IntegerValue {
        width: value.width as usize,
        signed: value.is_signed,
        bits: from_words(&value.words),
        x_mask: from_words(&value.x_mask),
        z_mask: from_words(&value.z_mask),
    }
}

fn convert_value(
    nodes: &[ffi::ValueNode],
    index: usize,
    depth: usize,
) -> Result<ConstantValue, Error> {
    if depth > 256 {
        return Err(snapshot_error("native value nesting exceeds 256 levels"));
    }
    let node = nodes
        .get(index)
        .ok_or_else(|| snapshot_error("invalid native value reference"))?;
    let child = |index| convert_value(nodes, index, depth + 1);
    let elements = || {
        node.elements
            .iter()
            .map(|index| child(*index))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(match node.kind {
        ffi::ValueKind::Invalid => ConstantValue::Invalid,
        ffi::ValueKind::Integer => ConstantValue::Integer(convert_integer(&node.integer)),
        ffi::ValueKind::Real => ConstantValue::Real(node.real),
        ffi::ValueKind::ShortReal => ConstantValue::ShortReal(node.short_real),
        ffi::ValueKind::String => ConstantValue::String(node.bytes.clone()),
        ffi::ValueKind::Elements => ConstantValue::Elements(elements()?),
        ffi::ValueKind::Map => ConstantValue::Map {
            entries: node
                .entries
                .iter()
                .map(|entry| Ok((child(entry.key)?, child(entry.value)?)))
                .collect::<Result<Vec<_>, Error>>()?,
            default: Box::new(child(node.default_value)?),
        },
        ffi::ValueKind::Queue => ConstantValue::Queue {
            elements: elements()?,
            max_bound: node.max_bound,
        },
        ffi::ValueKind::Union => ConstantValue::Union {
            value: Box::new(child(
                *node
                    .elements
                    .first()
                    .ok_or_else(|| snapshot_error("native union has no value"))?,
            )?),
            active_member: node.has_active_member.then_some(node.active_member),
        },
        ffi::ValueKind::Null => ConstantValue::Null,
        ffi::ValueKind::Unbounded => ConstantValue::Unbounded,
        _ => return Err(snapshot_error("invalid native value kind")),
    })
}
