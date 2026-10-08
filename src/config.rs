// SPDX-License-Identifier: Apache-2.0

use std::fmt;
use std::num::NonZeroU32;

/// A source file or named source text held in memory.
///
/// Inputs are processed in the order supplied in [`SlangConfig::sources`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source<'a> {
    /// A file path or pattern, resolved and loaded by Slang.
    File(&'a str),
    /// Source text to parse as a compilation input. The name is used for
    /// diagnostics and relative include resolution; no file is written there.
    Text { name: &'a str, text: &'a str },
}

impl<'a> Source<'a> {
    /// Refer to a source file or pattern without reading it yet.
    pub const fn file(path: &'a str) -> Self {
        Self::File(path)
    }

    /// Supply named source text without creating a temporary file.
    pub const fn text(name: &'a str, text: &'a str) -> Self {
        Self::Text { name, text }
    }
}

/// Language standard used for parsing and elaboration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LanguageVersion {
    Verilog2005,
    #[default]
    SystemVerilog2017,
    SystemVerilog2023,
}

/// Unit used in a default timescale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    #[default]
    Nanoseconds,
    Picoseconds,
    Femtoseconds,
}

/// Multipliers permitted by SystemVerilog timescales.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimeScaleMagnitude {
    #[default]
    One,
    Ten,
    Hundred,
}

/// A timescale value such as ten picoseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TimeScaleValue {
    pub unit: TimeUnit,
    pub magnitude: TimeScaleMagnitude,
}

/// Default time unit and precision for scopes without an explicit timescale.
/// Precision must be at least as fine as the base unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TimeScale {
    pub base: TimeScaleValue,
    pub precision: TimeScaleValue,
}

/// Comment directive words delimiting source text to exclude from parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TranslateOff<'a> {
    pub common: &'a str,
    pub start: &'a str,
    pub end: &'a str,
}

/// Baseline warning policy, before applying individual overrides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WarningPolicy {
    /// Slang's default set of optional warnings.
    #[default]
    Default,
    /// Every optional warning.
    All,
    /// Suppress optional warnings; standards violations remain errors.
    None,
}

/// Severity of a compiler diagnostic or an explicit diagnostic override.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    Ignored,
    Note,
    Warning,
    Error,
    Fatal,
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ignored => "ignored",
            Self::Note => "note",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Fatal => "fatal",
        })
    }
}

/// Assign a severity to a Slang diagnostic or diagnostic group by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticOverride<'a> {
    pub name: &'a str,
    pub severity: DiagnosticSeverity,
}

/// Diagnostic controls applied directly to Slang's diagnostic engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticOptions<'a> {
    pub warnings: WarningPolicy,
    /// Promote enabled warnings unless an explicit override selects a severity.
    pub warnings_as_errors: bool,
    /// Maximum errors reported before stopping. Zero removes the limit.
    pub error_limit: u32,
    /// Overrides applied in order after the baseline warning policy and
    /// protected-envelope suppression. Later entries win.
    pub overrides: &'a [DiagnosticOverride<'a>],
}

impl Default for DiagnosticOptions<'_> {
    fn default() -> Self {
        Self {
            warnings: WarningPolicy::Default,
            warnings_as_errors: false,
            error_limit: 20,
            overrides: &[],
        }
    }
}

/// Inputs and settings for an in-process Slang compilation.
#[derive(Clone, Copy, Debug)]
pub struct SlangConfig<'a> {
    /// File paths or patterns and named text buffers, in compilation order.
    /// File patterns expand at their position in this list. Extension exclusions
    /// apply to file paths or patterns before glob expansion, not to text buffers.
    pub sources: &'a [Source<'a>],
    pub tops: &'a [&'a str],
    pub incdirs: &'a [&'a str],
    /// Macro names and replacement text. Replacement text is SystemVerilog.
    pub defines: &'a [(&'a str, &'a str)],
    /// Parameter names and SystemVerilog expressions evaluated by Slang.
    pub parameters: &'a [(&'a str, &'a str)],
    /// Library file paths or patterns, each parsed as a separate library unit.
    /// Directory prefixes apply; extension exclusions apply only to `sources`.
    pub libfiles: &'a [&'a str],
    pub libdirs: &'a [&'a str],
    pub libexts: &'a [&'a str],
    /// Search prefixes for relative paths in `sources` and `libfiles`.
    pub dir_prefixes: &'a [&'a str],
    /// Filename extensions to exclude from `sources`, without the leading dot.
    pub exclude_extensions: &'a [&'a str],
    pub ignore_unknown_modules: bool,
    /// Suppress diagnostics related to protected envelopes. This does not
    /// enable recognition of legacy protection directives; see
    /// `enable_legacy_protect`.
    pub ignore_protected: bool,
    /// Recognize legacy protection directives in addition to standard protected
    /// envelopes. Diagnostic suppression is controlled by `ignore_protected`.
    pub enable_legacy_protect: bool,
    pub timescale: Option<TimeScale>,
    pub language_version: LanguageVersion,
    /// Parse all regular file sources and source buffers as one compilation
    /// unit, sharing macros and declarations between them in input order.
    pub single_unit: bool,
    /// Let library units inherit macros from regular sources in `single_unit`
    /// mode. Library units otherwise have independent macro state.
    pub libraries_inherit_macros: bool,
    /// Run Slang's post-elaboration semantic analysis.
    pub analysis_enabled: bool,
    /// Lint inputs without automatic top-level instantiation or post-elaboration
    /// analysis. Explicitly selected `tops` are still elaborated.
    pub lint_only: bool,
    pub relax_enum_conversions: bool,
    /// Allow identifiers to be used before their declarations where Slang
    /// normally reports an error.
    pub allow_use_before_declare: bool,
    pub allow_toplevel_interface_ports: bool,
    /// Number of parsing and analysis threads. `None` lets Slang choose automatically.
    pub num_threads: Option<NonZeroU32>,
    pub translate_off: &'a [TranslateOff<'a>],
    pub diagnostics: DiagnosticOptions<'a>,
    /// Omit ports that slang-rs cannot represent from `Compilation::ports()`.
    /// Defaults to `false`, which reports unsupported ports as errors.
    pub skip_unsupported_ports: bool,
    /// Omit parameters with unsupported types from `Compilation::parameters()`.
    /// Defaults to `false`, which reports unsupported parameter types as errors.
    pub skip_unsupported_parameters: bool,
}

impl Default for SlangConfig<'_> {
    fn default() -> Self {
        Self {
            sources: &[],
            tops: &[],
            incdirs: &[],
            defines: &[],
            parameters: &[],
            libfiles: &[],
            libdirs: &[],
            libexts: &[],
            dir_prefixes: &[],
            exclude_extensions: &[],
            ignore_unknown_modules: true,
            ignore_protected: true,
            enable_legacy_protect: true,
            timescale: None,
            language_version: LanguageVersion::default(),
            single_unit: false,
            libraries_inherit_macros: false,
            analysis_enabled: true,
            lint_only: false,
            relax_enum_conversions: false,
            allow_use_before_declare: false,
            allow_toplevel_interface_ports: false,
            num_threads: None,
            translate_off: &[],
            diagnostics: DiagnosticOptions::default(),
            skip_unsupported_ports: false,
            skip_unsupported_parameters: false,
        }
    }
}
