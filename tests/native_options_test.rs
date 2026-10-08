// SPDX-License-Identifier: Apache-2.0

use slang_rs::{
    Compilation, ConstantValue, DiagnosticOptions, DiagnosticOverride, DiagnosticSeverity,
    LanguageVersion, SlangConfig, Source, TimeScale, TimeScaleMagnitude, TimeScaleValue, TimeUnit,
    TranslateOff, WarningPolicy,
};
use std::num::NonZeroU32;

const MULTIPLE_DRIVERS: Source<'static> = Source::Text {
    name: "drivers.sv",
    text: "module top(input logic a, b, output logic q);
             always_comb q = a;
             always_comb q = b;
           endmodule",
};

const REDEFINED_MACRO: Source<'static> = Source::Text {
    name: "macros.sv",
    text: "`define WIDTH 1
           `define WIDTH 2
           module top(input [`WIDTH-1:0] data); endmodule",
};

#[test]
fn analysis_diagnostics_respect_structured_warning_policy() {
    let overrides = [DiagnosticOverride {
        name: "multiple-always-assigns",
        severity: DiagnosticSeverity::Error,
    }];
    let error = Compilation::new(&SlangConfig {
        sources: &[MULTIPLE_DRIVERS],
        diagnostics: DiagnosticOptions {
            overrides: &overrides,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Error
                && diagnostic.code == "MultipleAlwaysAssigns"
        }),
        "expected an analysis diagnostic promoted to an error: {error:?}"
    );

    let baseline_error = Compilation::new(&SlangConfig {
        sources: &[MULTIPLE_DRIVERS],
        diagnostics: DiagnosticOptions {
            warnings: WarningPolicy::None,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap_err();
    assert!(baseline_error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "MultipleAlwaysAssigns"
            && diagnostic.severity == DiagnosticSeverity::Error
    }));

    let compilation = Compilation::new(&SlangConfig {
        sources: &[MULTIPLE_DRIVERS],
        analysis_enabled: false,
        diagnostics: DiagnosticOptions {
            overrides: &overrides,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    assert_eq!(compilation.ports().unwrap()["top"].len(), 3);
    assert!(
        compilation
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code != "MultipleAlwaysAssigns")
    );
}

#[test]
fn warning_defaults_all_none_and_explicit_overrides_are_distinct() {
    let sources = [Source::Text {
        name: "unused.sv",
        text: "module top; logic spare; endmodule",
    }];
    let default = Compilation::new(&SlangConfig {
        sources: &sources,
        ..Default::default()
    })
    .unwrap();
    assert!(
        default
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code != "UnusedVariable")
    );
    let all = Compilation::new(&SlangConfig {
        sources: &sources,
        diagnostics: DiagnosticOptions {
            warnings: WarningPolicy::All,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    assert!(all.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "UnusedVariable" && diagnostic.severity == DiagnosticSeverity::Warning
    }));
    let warnings = all.diagnostics().to_vec();
    all.ports().unwrap();
    all.hierarchy().unwrap();
    assert_eq!(all.diagnostics(), warnings);
    drop(all);
    assert!(warnings.iter().any(|diagnostic| {
        diagnostic.location.as_ref().is_some_and(|location| {
            location.file.ends_with("unused.sv") && location.source_line.contains("logic spare")
        })
    }));

    let defaults = Compilation::new(&SlangConfig {
        sources: &[REDEFINED_MACRO],
        ..Default::default()
    })
    .unwrap();
    assert!(defaults.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "RedefiningMacro" && diagnostic.severity == DiagnosticSeverity::Warning
    }));
    let none = Compilation::new(&SlangConfig {
        sources: &[REDEFINED_MACRO],
        diagnostics: DiagnosticOptions {
            warnings: WarningPolicy::None,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    assert!(none.diagnostics().is_empty());

    let error = Compilation::new(&SlangConfig {
        sources: &sources,
        diagnostics: DiagnosticOptions {
            warnings: WarningPolicy::None,
            overrides: &[DiagnosticOverride {
                name: "unused",
                severity: DiagnosticSeverity::Error,
            }],
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "UnusedVariable" && diagnostic.severity == DiagnosticSeverity::Error
    }));
}

#[test]
fn explicit_severity_overrides_global_promotion_and_later_overrides_win() {
    let error = Compilation::new(&SlangConfig {
        sources: &[REDEFINED_MACRO],
        diagnostics: DiagnosticOptions {
            warnings_as_errors: true,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "RedefiningMacro" && diagnostic.severity == DiagnosticSeverity::Error
    }));

    let compilation = Compilation::new(&SlangConfig {
        sources: &[REDEFINED_MACRO],
        diagnostics: DiagnosticOptions {
            warnings_as_errors: true,
            overrides: &[
                DiagnosticOverride {
                    name: "redef-macro",
                    severity: DiagnosticSeverity::Ignored,
                },
                DiagnosticOverride {
                    name: "redef-macro",
                    severity: DiagnosticSeverity::Error,
                },
                DiagnosticOverride {
                    name: "redef-macro",
                    severity: DiagnosticSeverity::Warning,
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    assert!(compilation.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "RedefiningMacro" && diagnostic.severity == DiagnosticSeverity::Warning
    }));
}

#[test]
fn protected_suppression_survives_global_warning_errors_but_can_be_overridden() {
    let sources = [Source::Text {
        name: "protected.sv",
        text: "module top;
                 `protected
                 encrypted_placeholder
                 `endprotected
               endmodule\n",
    }];
    Compilation::new(&SlangConfig {
        sources: &sources,
        diagnostics: DiagnosticOptions {
            warnings: WarningPolicy::All,
            warnings_as_errors: true,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();

    let compilation = Compilation::new(&SlangConfig {
        sources: &sources,
        ignore_protected: false,
        ..Default::default()
    })
    .unwrap();
    assert!(compilation.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "ProtectedEnvelope" && diagnostic.severity == DiagnosticSeverity::Warning
    }));

    let error = Compilation::new(&SlangConfig {
        sources: &sources,
        diagnostics: DiagnosticOptions {
            overrides: &[DiagnosticOverride {
                name: "protected-envelope",
                severity: DiagnosticSeverity::Error,
            }],
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "ProtectedEnvelope" && diagnostic.severity == DiagnosticSeverity::Error
    }));
}

#[test]
fn language_version_controls_keywords_and_new_syntax() {
    let verilog = [Source::Text {
        name: "keywords.v",
        text: "module top(input wire logic); endmodule",
    }];
    let compilation = Compilation::new(&SlangConfig {
        sources: &verilog,
        language_version: LanguageVersion::Verilog2005,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(compilation.ports().unwrap()["top"][0].name, "logic");
    assert!(
        Compilation::new(&SlangConfig {
            sources: &verilog,
            ..Default::default()
        })
        .is_err()
    );

    let systemverilog = [Source::Text {
        name: "strings.sv",
        text: "package p; localparam string MESSAGE = \"\"\"hello\"\"\"; endpackage",
    }];
    let error = Compilation::new(&SlangConfig {
        sources: &systemverilog,
        language_version: LanguageVersion::SystemVerilog2017,
        ..Default::default()
    })
    .unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "WrongLanguageVersion")
    );
    let compilation = Compilation::new(&SlangConfig {
        sources: &systemverilog,
        language_version: LanguageVersion::SystemVerilog2023,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        compilation.packages().unwrap()["p"].parameters["MESSAGE"],
        ConstantValue::String(b"hello".to_vec())
    );
}

#[test]
fn configuration_strings_are_owned_and_translate_markers_are_structured() {
    let compilation = {
        let top = String::from("selected_top");
        let common = String::from("custom");
        let start = String::from("omit_begin");
        let end = String::from("omit_end");
        let sources = [Source::Text {
            name: "translate.sv",
            text: "module selected_top(input logic [6:0] data);
                   // custom omit_begin
                   this is not valid SystemVerilog
                   // custom omit_end
                   endmodule
                   module other_top; endmodule",
        }];
        Compilation::new(&SlangConfig {
            sources: &sources,
            tops: &[&top],
            translate_off: &[TranslateOff {
                common: &common,
                start: &start,
                end: &end,
            }],
            ..Default::default()
        })
        .unwrap()
    };
    let ports = compilation.ports().unwrap();
    assert_eq!(ports.len(), 1);
    assert_eq!(ports["selected_top"][0].ty.width().unwrap(), 7);
    assert_eq!(compilation.hierarchy().unwrap().len(), 1);
}

#[test]
fn invalid_structured_settings_return_contextual_errors() {
    let sources = [Source::Text {
        name: "top.sv",
        text: "module top; endmodule",
    }];
    let error = Compilation::new(&SlangConfig {
        sources: &sources,
        diagnostics: DiagnosticOptions {
            overrides: &[DiagnosticOverride {
                name: "not-a-diagnostic",
                severity: DiagnosticSeverity::Error,
            }],
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.to_string().contains("not-a-diagnostic"), "{error}");

    let error = Compilation::new(&SlangConfig {
        sources: &sources,
        libraries_inherit_macros: true,
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.to_string().contains("single_unit"), "{error}");

    let error = Compilation::new(&SlangConfig {
        sources: &sources,
        timescale: Some(TimeScale {
            base: TimeScaleValue {
                unit: TimeUnit::Picoseconds,
                magnitude: TimeScaleMagnitude::One,
            },
            precision: TimeScaleValue {
                unit: TimeUnit::Nanoseconds,
                magnitude: TimeScaleMagnitude::One,
            },
        }),
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.to_string().contains("precision"), "{error}");
}

#[test]
fn missing_source_returns_a_contextual_error() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.sv");
    let error = Compilation::new(&SlangConfig {
        sources: &[Source::file(missing.to_str().unwrap())],
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.to_string().contains("missing.sv"), "{error}");
}

#[test]
fn lint_only_does_not_automatically_instantiate_modules() {
    let sources = [Source::Text {
        name: "recursive.sv",
        text: "module recursive(input logic [3:0] data);
                 recursive child(.data(data));
               endmodule\n",
    }];
    let mut config = SlangConfig {
        sources: &sources,
        tops: &["recursive"],
        num_threads: Some(NonZeroU32::new(1).unwrap()),
        ..Default::default()
    };
    let error = Compilation::new(&config).unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "InfinitelyRecursiveHierarchy" })
    );
    config.lint_only = true;
    let error = Compilation::new(&config).unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "InfinitelyRecursiveHierarchy" })
    );
    config.tops = &[];
    let compilation = Compilation::new(&config).unwrap();
    assert_eq!(compilation.modules().unwrap(), ["recursive"]);
    assert!(compilation.ports().unwrap().is_empty());
}

#[test]
fn named_warning_overrides_accept_native_diagnostics_and_groups() {
    let sources = [Source::Text {
        name: "connections.sv",
        text: "module leaf(input wire a, output wire b, inout wire c); endmodule
               module top; leaf child(); endmodule\n",
    }];
    let compilation = Compilation::new(&SlangConfig {
        sources: &sources,
        tops: &["top"],
        ..Default::default()
    })
    .unwrap();
    assert!(compilation.diagnostics().iter().any(|diagnostic| {
        diagnostic.code.starts_with("Unconnected")
            && diagnostic.severity == DiagnosticSeverity::Warning
    }));

    let overrides: Vec<_> = [
        "duplicate-definition",
        "protected-envelope",
        "unknown-protect-keyword",
        "unconnected-port",
        "implicit-conv",
        "implicit-port-type-mismatch",
    ]
    .into_iter()
    .map(|name| DiagnosticOverride {
        name,
        severity: DiagnosticSeverity::Ignored,
    })
    .collect();
    let compilation = Compilation::new(&SlangConfig {
        sources: &sources,
        tops: &["top"],
        diagnostics: DiagnosticOptions {
            overrides: &overrides,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    assert!(
        compilation
            .diagnostics()
            .iter()
            .all(|diagnostic| { !diagnostic.code.starts_with("Unconnected") })
    );
}

#[test]
fn use_before_declaration_can_be_enabled_for_implicit_port_connections() {
    let sources = [Source::Text {
        name: "declarations.sv",
        text: "module leaf(input logic data); endmodule
               module top;
                 leaf child(.data);
                 logic data;
               endmodule\n",
    }];
    let mut config = SlangConfig {
        sources: &sources,
        tops: &["top"],
        ..Default::default()
    };
    let error = Compilation::new(&config).unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "UsedBeforeDeclared" })
    );
    config.allow_use_before_declare = true;
    let compilation = Compilation::new(&config).unwrap();
    assert_eq!(
        compilation.hierarchy().unwrap()["top"].contents[0].def_name,
        "leaf"
    );
}
