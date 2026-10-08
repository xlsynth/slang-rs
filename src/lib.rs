// SPDX-License-Identifier: Apache-2.0

mod native;
pub use native::{Compilation, Diagnostic, Error, SourceLocation};

mod config;
pub use config::{
    DiagnosticOptions, DiagnosticOverride, DiagnosticSeverity, LanguageVersion, SlangConfig,
    Source, TimeScale, TimeScaleMagnitude, TimeScaleValue, TimeUnit, TranslateOff, WarningPolicy,
};

mod extract;
pub use extract::{Parameter, Port, PortDir, PortList};

mod types;
pub use types::{Field, IntegralKind, Range, Type, Variant};

mod values;
pub use values::{ConstantValue, IntegerConversionError, IntegerValue};

mod hierarchy;
pub use hierarchy::Instance;

mod package;
pub use package::Package;
