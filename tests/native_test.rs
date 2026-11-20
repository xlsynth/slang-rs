// SPDX-License-Identifier: Apache-2.0

use num_bigint::BigInt;
use slang_rs::{Compilation, ConstantValue, PortDir, SlangConfig, Source, Type};
use std::fs;
use std::process::Command;
use std::sync::{Arc, Barrier};

#[test]
fn native_queries_reuse_session_without_external_tools() {
    // Isolate the empty PATH in a child process so the parallel test harness
    // retains its environment while this test proves no external tools are used.
    const CHILD_MARKER: &str = "SLANG_RS_NATIVE_TEST_CHILD";
    if std::env::var_os(CHILD_MARKER).is_none() {
        let empty_path = tempfile::tempdir().unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native_queries_reuse_session_without_external_tools",
            ])
            .env(CHILD_MARKER, "1")
            .env("PATH", empty_path.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "native queries required an external program:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    }

    let source = "package config_pkg; localparam int ANSWER = 42; endpackage
         module child; endmodule
         module top #(parameter int WIDTH = 8) (input [WIDTH-1:0] data);
           child inst();
         endmodule";
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(source)],
        tops: &["top"],
        ..Default::default()
    })
    .unwrap();

    // Query in different orders, twice, to catch destructive AST traversal or
    // result storage accidentally reused between API calls.
    let ports = compilation.ports().unwrap();
    let packages = compilation.packages().unwrap();
    let parameters = compilation.parameters().unwrap();
    let hierarchy = compilation.hierarchy().unwrap();
    let mut modules = compilation.modules().unwrap();
    modules.sort();
    assert_eq!(modules, ["child", "top"]);
    assert_eq!(ports["top"][0].name, "data");
    assert_eq!(ports["top"][0].dir, PortDir::Input);
    assert_eq!(ports["top"][0].ty.width().unwrap(), 8);
    assert_eq!(parameters["top"][0].name, "WIDTH");
    assert_eq!(
        BigInt::try_from(&packages["config_pkg"].parameters["ANSWER"]).unwrap(),
        BigInt::from(42)
    );
    assert_eq!(hierarchy["top"].contents[0].inst_name, "inst");
    assert_eq!(compilation.hierarchy().unwrap(), hierarchy);
    assert_eq!(compilation.ports().unwrap(), ports);
    assert_eq!(compilation.packages().unwrap(), packages);
}

#[test]
fn selective_queries_snapshot_only_the_named_declaration() {
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(
            "package first; localparam int W = 4; endpackage
             package second; typedef logic [4:0] word_t; endpackage
             interface bus; logic x; endinterface
             module bad(bus b); endmodule
             module child; endmodule
             module good(input logic [first::W-1:0] a); child c(); endmodule",
        )],
        allow_toplevel_interface_ports: true,
        ..Default::default()
    })
    .unwrap();
    let mut tops = compilation.top_names().unwrap();
    tops.sort();
    assert_eq!(tops, ["bad", "good"]);
    assert!(compilation.ports().is_err());
    assert_eq!(
        compilation.ports_for("good").unwrap()[0]
            .ty
            .width()
            .unwrap(),
        4
    );
    for name in ["child", ""] {
        let missing = slang_rs::LookupError::NotFound {
            kind: "top",
            name: name.into(),
        };
        assert_eq!(compilation.ports_for(name).unwrap_err(), missing);
        assert_eq!(compilation.hierarchy_for(name).unwrap_err(), missing);
    }
    assert_eq!(
        compilation.hierarchy_for("good").unwrap(),
        compilation.hierarchy().unwrap()["good"]
    );
    let mut names = compilation.package_names().unwrap();
    names.sort();
    assert_eq!(names, ["first", "second"]);
    let packages = compilation.packages().unwrap();
    for name in names {
        assert_eq!(
            compilation.package(&name).unwrap().snapshot().unwrap(),
            packages[&name]
        );
    }
    for name in ["missing", "std", ""] {
        assert_eq!(
            compilation.package(name).unwrap_err(),
            slang_rs::LookupError::NotFound {
                kind: "package",
                name: name.into()
            }
        );
    }
}

#[test]
fn query_results_outlive_sources_and_compilation() {
    let (ports, parameters, modules, packages, hierarchy) = {
        let source = String::from(
            "package p;
               localparam int VALUE = 19;
               localparam string MESSAGE = \"owned bytes\";
               typedef logic [VALUE-1:0] word_t;
             endpackage
             module leaf; endmodule
             module top #(parameter N = 5) (input [N-1:0] a);
               leaf child();
             endmodule",
        );
        let compilation = Compilation::new(&SlangConfig {
            sources: &[Source::text(&source)],
            tops: &["top"],
            ..Default::default()
        })
        .unwrap();
        (
            compilation.ports().unwrap(),
            compilation.parameters().unwrap(),
            compilation.modules().unwrap(),
            compilation.packages().unwrap(),
            compilation.hierarchy().unwrap(),
        )
    };

    assert_eq!(ports["top"][0].ty.width().unwrap(), 5);
    assert_eq!(parameters["top"][0].name, "N");
    assert!(modules.iter().any(|name| name == "leaf"));
    assert_eq!(
        BigInt::try_from(&packages["p"].parameters["VALUE"]).unwrap(),
        BigInt::from(19)
    );
    assert_eq!(
        packages["p"].types["word_t"]
            .as_ref()
            .unwrap()
            .width()
            .unwrap(),
        19
    );
    assert_eq!(
        packages["p"].parameters["MESSAGE"],
        ConstantValue::String(b"owned bytes".to_vec())
    );
    assert_eq!(hierarchy["top"].contents[0].def_name, "leaf");
    assert_eq!(hierarchy["top"].contents[0].path, "top.child");
}

#[test]
fn malformed_source_returns_a_contextual_error() {
    let error = Compilation::new(&SlangConfig {
        sources: &[Source::named_text(
            "broken.sv",
            "module broken(input logic a;\nendmodule\n",
        )],
        ..Default::default()
    })
    .expect_err("malformed source should return an error, not an incomplete compilation");
    let message = error.to_string();
    assert!(message.contains("broken.sv"), "{message}");
    assert!(message.contains("error"), "{message}");
    assert!(
        error.diagnostics.iter().any(|diagnostic| {
            diagnostic.location.as_ref().is_some_and(|location| {
                location.file.ends_with("broken.sv")
                    && location.line == 1
                    && location.column > 0
                    && location.source_line.contains("module broken")
            })
        }),
        "expected structured source coordinates: {error:?}"
    );
}

#[test]
fn diagnostics_preserve_context_when_a_source_file_contains_non_utf8_comments() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("non_utf8.sv");
    fs::write(&path, b"module broken(input logic a; // \xff\nendmodule\n").unwrap();
    let error = Compilation::new(&SlangConfig {
        sources: &[Source::file(path.to_str().unwrap())],
        ..Default::default()
    })
    .expect_err("the missing parenthesis should produce a compiler diagnostic");

    assert!(
        error.diagnostics.iter().any(|diagnostic| {
            diagnostic.location.as_ref().is_some_and(|location| {
                location.file.ends_with("non_utf8.sv")
                    && location.line == 1
                    && location.source_line.contains("module broken")
                    && location.source_line.contains('\u{fffd}')
            })
        }),
        "expected structured source context despite the comment: {error:?}"
    );
}

#[test]
fn native_configuration_selects_top_and_applies_includes_defines_and_parameters() {
    let directory = tempfile::tempdir().unwrap();
    let include_dir = directory.path().join("include files");
    fs::create_dir(&include_dir).unwrap();
    fs::write(include_dir.join("width.svh"), "`define DEFAULT_WIDTH 2\n").unwrap();
    let source = directory.path().join("design with spaces.sv");
    fs::write(
        &source,
        "`include \"width.svh\"
         module selected #(parameter int WIDTH = `DEFAULT_WIDTH)
           (input [WIDTH + `EXTRA - 1:0] a);
         endmodule
         module another_root(input b); endmodule",
    )
    .unwrap();
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::file(source.to_str().unwrap())],
        tops: &["selected"],
        incdirs: &[include_dir.to_str().unwrap()],
        defines: &[("EXTRA", "3")],
        parameters: &[("WIDTH", "7")],
        ..Default::default()
    })
    .unwrap();
    let ports = compilation.ports().unwrap();
    assert_eq!(ports.len(), 1);
    assert_eq!(ports["selected"][0].ty.width().unwrap(), 10);
    assert!(
        compilation
            .modules()
            .unwrap()
            .contains(&"another_root".into())
    );
}

#[test]
fn native_configuration_resolves_library_files_and_search_directories() {
    let directory = tempfile::tempdir().unwrap();
    let libdir = directory.path().join("lib");
    fs::create_dir(&libdir).unwrap();
    let source = directory.path().join("top.sv");
    let library = directory.path().join("library.sv");
    fs::write(
        &source,
        "module top; from_file first(); from_search second(); endmodule",
    )
    .unwrap();
    fs::write(&library, "module from_file; endmodule").unwrap();
    fs::write(
        libdir.join("from_search.cell"),
        "module from_search; endmodule",
    )
    .unwrap();
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::file(source.to_str().unwrap())],
        tops: &["top"],
        libfiles: &[library.to_str().unwrap()],
        libdirs: &[libdir.to_str().unwrap()],
        libexts: &[".cell"],
        ignore_unknown_modules: false,
        ..Default::default()
    })
    .unwrap();
    let hierarchy = compilation.hierarchy().unwrap();
    let children = &hierarchy["top"].contents;
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].def_name, "from_file");
    assert_eq!(children[1].def_name, "from_search");
}

#[test]
fn independent_compilations_can_run_concurrently() {
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (1..=4)
        .map(|width| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                // A compilation stays on its owning thread; this exercises
                // separate Slang contexts, without requiring Compilation: Send.
                let source = format!("module top(input [{}:0] a); endmodule", width - 1);
                barrier.wait();
                let compilation = Compilation::new(&SlangConfig {
                    sources: &[Source::text(&source)],
                    ..Default::default()
                })
                .unwrap();
                assert_eq!(
                    compilation.ports().unwrap()["top"][0].ty.width().unwrap(),
                    width
                );
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

#[test]
fn unsupported_ports_and_parameters_can_be_skipped_independently() {
    let source = "interface bus; logic data; endinterface
         module top #(parameter int WIDTH = 8, parameter string LABEL = \"top\")
           (input logic clk, bus link); endmodule";
    let config = SlangConfig {
        sources: &[Source::text(source)],
        tops: &["top"],
        allow_toplevel_interface_ports: true,
        ..Default::default()
    };
    let compilation = Compilation::new(&config).unwrap();
    let error = compilation.ports().unwrap_err().to_string();
    assert!(error.to_lowercase().contains("interface"), "{error}");
    assert!(error.contains("link"), "{error}");
    let error = compilation
        .parameters()
        .err()
        .expect("unsupported parameter types should fail by default")
        .to_string();
    assert!(error.contains("LABEL"), "{error}");
    assert!(error.contains("StringType"), "{error}");

    let compilation = Compilation::new(&SlangConfig {
        skip_unsupported_ports: true,
        ..config
    })
    .unwrap();
    let ports = compilation.ports().unwrap();
    assert_eq!(ports["top"].len(), 1);
    assert_eq!(ports["top"][0].name, "clk");
    assert!(compilation.parameters().is_err());

    let compilation = Compilation::new(&SlangConfig {
        skip_unsupported_parameters: true,
        ..config
    })
    .unwrap();
    let parameters = compilation.parameters().unwrap();
    assert_eq!(parameters["top"].len(), 1);
    assert_eq!(parameters["top"][0].name, "WIDTH");
    assert_eq!(parameters["top"][0].ty.width().unwrap(), 32);
    assert!(compilation.ports().is_err());
}

#[test]
fn anonymous_semantic_types_do_not_get_synthetic_names() {
    let source = "module top(input struct packed {logic [2:0] value;} data); endmodule";
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(source)],
        ..Default::default()
    })
    .unwrap();
    let ports = compilation.ports().unwrap();
    let Type::Struct {
        name, is_packed, ..
    } = &ports["top"][0].ty
    else {
        panic!("expected a packed struct");
    };
    assert_eq!(name, &None);
    assert!(is_packed);
    assert_eq!(ports["top"][0].ty.width().unwrap(), 3);
}

#[test]
fn ports_support_name_lookup_and_iteration_in_port_list_order() {
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(
            "module top(zebra, alpha, middle);
               input alpha;
               output [3:0] middle;
               input [7:0] zebra;
             endmodule",
        )],
        ..Default::default()
    })
    .unwrap();
    let mut ports = compilation.ports().unwrap();
    let top = &ports["top"];

    assert!(std::ptr::eq(&top["zebra"], &top[0]));
    assert!(std::ptr::eq(top.get_by_name("middle").unwrap(), &top[2]));
    assert_eq!(top["zebra"].ty.width().unwrap(), 8);
    assert!(top.get_by_name("missing").is_none());
    assert!(top.get_by_name("").is_none());

    let mut names = Vec::new();
    for port in top {
        names.push(port.name.as_str());
    }
    assert_eq!(names, ["zebra", "alpha", "middle"]);

    let names: Vec<_> = ports
        .remove("top")
        .unwrap()
        .into_iter()
        .map(|port| port.name)
        .collect();
    assert_eq!(names, ["zebra", "alpha", "middle"]);
}

#[test]
fn unnamed_ports_keep_their_positions() {
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(
            "module top(bus[3:2], bus[0], clk);
               input [3:0] bus;
               input clk;
             endmodule",
        )],
        ..Default::default()
    })
    .unwrap();
    let mut ports = compilation.ports().unwrap();
    let top = &ports["top"];

    assert_eq!(top.len(), 3);
    assert_eq!(top[0].name, "");
    assert_eq!(top[1].name, "");
    assert!(std::ptr::eq(&top["clk"], &top[2]));
    assert!(top.get_by_name("").is_none());
    assert!(top.get_by_name("bus").is_none());

    let widths: Vec<_> = top
        .into_iter()
        .map(|port| port.ty.width().unwrap())
        .collect();
    assert_eq!(widths, [2, 1, 1]);
    let widths: Vec<_> = ports
        .remove("top")
        .unwrap()
        .into_iter()
        .map(|port| port.ty.width().unwrap())
        .collect();
    assert_eq!(widths, [2, 1, 1]);
}

#[test]
fn diagnostics_and_errors_share_readable_formatting() {
    use slang_rs::{Diagnostic, DiagnosticSeverity, Error, SourceLocation};

    let cases = [
        (
            Diagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "UnusedVariable".into(),
                message: "unused variable 'data'".into(),
                location: Some(SourceLocation {
                    file: "top.sv".into(),
                    line: 2,
                    column: 9,
                    source_line: "  logic data;".into(),
                }),
            },
            "top.sv:2:9: warning: unused variable 'data'\n  logic data;",
        ),
        (
            Diagnostic {
                severity: DiagnosticSeverity::Note,
                code: "NotePreviousDefinition".into(),
                message: "previous definition here".into(),
                location: Some(SourceLocation {
                    file: "other.sv".into(),
                    line: 4,
                    column: 1,
                    source_line: String::new(),
                }),
            },
            "other.sv:4:1: note: previous definition here",
        ),
        (
            Diagnostic {
                severity: DiagnosticSeverity::Error,
                code: "NoTopModules".into(),
                message: "no top-level modules found".into(),
                location: None,
            },
            "error: no top-level modules found",
        ),
    ];
    for (diagnostic, expected) in &cases {
        assert_eq!(diagnostic.to_string(), *expected);
    }

    let error = Error {
        message: "compilation failed".into(),
        diagnostics: cases
            .into_iter()
            .map(|(diagnostic, _)| diagnostic)
            .collect(),
    };
    assert_eq!(
        error.to_string(),
        "compilation failed\n\
         top.sv:2:9: warning: unused variable 'data'\n  logic data;\n\
         other.sv:4:1: note: previous definition here\n\
         error: no top-level modules found"
    );
    assert_eq!(
        Error {
            message: "extraction failed".into(),
            diagnostics: Vec::new(),
        }
        .to_string(),
        "extraction failed"
    );
}

#[test]
fn selected_parameter_values_belong_to_the_elaborated_top() {
    let source = r#"module Leaf #(parameter W=4, parameter MSG="ab\000cd",
                                 localparam L=W+1) (); endmodule
                    module Other; endmodule"#;
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(source)],
        parameters: &[("W", "4 + 4")],
        ..Default::default()
    })
    .unwrap();
    let requested = ["W", "L", "missing", "MSG"];
    let values = compilation
        .parameter_values_for("Leaf", &requested)
        .unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(
        i32::try_from(values["W"].value.as_integer().unwrap()).unwrap(),
        8
    );
    let msg = values["MSG"].value.as_integer().unwrap();
    assert_eq!(msg.width, 40);
    assert_eq!(msg.bits, num_bigint::BigUint::from_bytes_be(b"ab\0cd"));
    assert!(values["MSG"].is_implicit_string);
    assert!(!values["W"].is_implicit_string);
    assert_eq!(values["W"].ty.as_ref().unwrap().width().unwrap(), 32);
    assert!(
        compilation
            .parameter_values_for("Other", &requested)
            .unwrap()
            .is_empty()
    );
    assert!(
        compilation
            .parameter_values_for("Absent", &requested)
            .is_err()
    );
}

#[test]
fn selected_parameter_retains_enum_type_and_unknown_string_bits() {
    let source = r#"package p;
                       typedef enum logic [1:0] { A=0, B=1 } mode_t;
                    endpackage
                    module Leaf #(parameter p::mode_t MODE=p::A,
                                  parameter V=1'bx ? "x" : "y",
                                  parameter string TEXT="abc",
                                  parameter real R=1.5) (); endmodule"#;
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(source)],
        parameters: &[("MODE", "p::B")],
        ..Default::default()
    })
    .unwrap();
    let values = compilation
        .parameter_values_for("Leaf", &["MODE", "V", "TEXT", "R"])
        .unwrap();
    assert!(
        matches!(&values["MODE"].ty, Ok(slang_rs::Type::Enum { name: Some(name), .. }) if name == "p::mode_t")
    );
    assert_eq!(
        u32::try_from(values["MODE"].value.as_integer().unwrap()).unwrap(),
        1
    );
    assert!(values["V"].is_implicit_string);
    assert!(!values["MODE"].is_implicit_string);
    let bits = values["V"].value.as_integer().unwrap();
    assert_eq!(bits.width, 8);
    assert_eq!(bits.bits, 0x78u32.into());
    assert_eq!(bits.x_mask, 1u32.into());
    assert_eq!(bits.z_mask, 0u32.into());
    assert_eq!(values["TEXT"].value, ConstantValue::String(b"abc".to_vec()));
    assert!(values["TEXT"].ty.is_err());
    assert_eq!(values["R"].value, ConstantValue::Real(1.5));
    assert!(values["R"].ty.is_err());
}

#[test]
fn implicit_string_and_packed_integer_have_distinct_conversion_properties() {
    let source = r#"module Leaf #(parameter V=1'bx ? "x" : "y",
                                  parameter logic [7:0] RAW=8'b0111100x,
                                  parameter string TEXT="abc",
                                  parameter enum logic { A, B } MODE=A,
                                  localparam enum logic { C, D } LOCAL=C) (); endmodule
                    module Other; endmodule"#;
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(source)],
        ..Default::default()
    })
    .unwrap();
    let values = compilation
        .parameter_values_for("Leaf", &["V", "RAW"])
        .unwrap();
    assert_eq!(values["V"].value, values["RAW"].value);
    assert!(values["V"].is_implicit_string);
    assert!(!values["RAW"].is_implicit_string);
    let parameters = compilation.enum_parameters_for("Leaf").unwrap();
    assert_eq!(
        parameters
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["MODE"]
    );
    assert!(compilation.enum_parameters_for("Other").unwrap().is_empty());
    assert!(compilation.enum_parameters_for("Absent").is_err());
}

#[test]
fn enum_queries_include_body_parameters_and_leave_inferred_values_lazy() {
    let source = "package p; typedef enum { OFF, ON } mode_t; endpackage
        module Header #(parameter type T=p::mode_t, parameter T MODE=p::OFF,
                        parameter INFERRED=p::ON)(); localparam int LOCAL=1; endmodule
        module Body; parameter p::mode_t MODE=p::OFF; localparam p::mode_t LOCAL=p::ON; endmodule";
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(source)],
        ..Default::default()
    })
    .unwrap();
    let all = compilation.parameters().unwrap();
    for (top, expected) in [
        ("Header", vec!["MODE", "INFERRED", "LOCAL"]),
        ("Body", vec!["MODE", "LOCAL"]),
    ] {
        let enums = compilation.enum_parameters_for(top).unwrap();
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "MODE");
        assert_eq!(
            all[top].iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            expected
        );
    }

    let partial = Compilation::new(&SlangConfig {
        sources: &[Source::text(
            "module Partial #(parameter UNUSED=missing)(); endmodule",
        )],
        check_level: slang_rs::CheckLevel::Parse,
        ..Default::default()
    })
    .unwrap();
    assert!(partial.enum_parameters_for("Partial").unwrap().is_empty());
}

/// Re-elaborate a reconstructed header at both the original and a different specialization.
#[test]
fn expanded_parameter_tokens_retain_their_lexical_boundaries() {
    let source = r#"
        `define ALL(x) &(x)
        `define TYPE bit
        `define NAME B
        `define JOIN(a,b) a``b
        module Leaf #(parameter `TYPE [1:0] `NAME=2'b11,
                      parameter `TYPE`JOIN(RED,UCED)=1 &`ALL(B),
                      parameter \x.y =REDUCED)();
        endmodule"#;
    let compile = |text: &str| {
        Compilation::new(&SlangConfig {
            sources: &[Source::text(text)],
            ..Default::default()
        })
        .unwrap()
    };
    let original = compile(source);
    let decl = original.parameter_declarations_for("Leaf").unwrap();
    let observed = |comp: &Compilation| {
        let values = comp
            .parameter_values_for("Leaf", &["REDUCED", "x.y"])
            .unwrap();
        ["REDUCED", "x.y"].map(|n| u32::try_from(values[n].value.as_integer().unwrap()).unwrap())
    };
    let list = decl.port_list.unwrap();
    let expr = decl.parameters["REDUCED"].default.as_deref().unwrap();
    let make_module = |header: &str| format!("module Leaf {header} (); endmodule");
    assert_eq!(observed(&original), [1, 1]);
    assert_eq!(observed(&compile(&make_module(&list))), [1, 1]);

    // The two operators agree at 2'b11. At 2'b10 a logical AND incorrectly returns one.
    let alternate = Compilation::new(&SlangConfig {
        sources: &[Source::text(source)],
        parameters: &[("B", "2'b10")],
        ..Default::default()
    })
    .unwrap();
    assert_eq!(observed(&alternate), [0, 0]);
    let range = decl.parameters["B"].port_list_initializer.clone().unwrap();
    let mut changed = list;
    changed.replace_range(range, "=2'b10");
    assert_eq!(observed(&compile(&make_module(&changed))), [0, 0]);
    let from_fragment =
        format!("#(parameter B=2'b10, parameter REDUCED={expr}, parameter \\x.y =REDUCED)");
    assert_eq!(observed(&compile(&make_module(&from_fragment))), [0, 0]);
}

#[test]
fn division_before_a_comment_survives_macro_expansion() {
    let source = r"`define DIV /
        module Leaf #(parameter A=0, parameter W=A `DIV/* divisor */ 2
                                                  + 1)(); endmodule";
    let compile = |source: &str, parameters: &[(&str, &str)]| {
        Compilation::new(&SlangConfig {
            sources: &[Source::text(source)],
            parameters,
            ..Default::default()
        })
        .unwrap()
    };
    let declarations = compile(source, &[])
        .parameter_declarations_for("Leaf")
        .unwrap();
    let header = declarations.port_list.unwrap();
    let default = declarations.parameters["W"].default.as_deref().unwrap();
    let from_header = format!("module Leaf {header} (); endmodule");
    let from_fragment = format!("module Leaf #(parameter A=0, parameter W={default})(); endmodule");

    for candidate in [source, &from_header, &from_fragment] {
        for (parameters, expected) in [(&[][..], 1), (&[("A", "8")][..], 5)] {
            let values = compile(candidate, parameters)
                .parameter_values_for("Leaf", &["W"])
                .unwrap();
            assert_eq!(usize::try_from(&values["W"].value).unwrap(), expected);
        }
    }
}

#[test]
fn original_parameter_declarations_survive_specialization() {
    let package = r#"package p;
        typedef struct packed {logic mode;} cfg_t;
        localparam cfg_t RESET = '{mode:1'b1};
        typedef cfg_t \cfg.done ;
        localparam \reset.done = '0;
    endpackage"#;
    let source = r#"
        `define BYTE logic [7:0]
        module Leaf import p::*; #(
          parameter int W,
          parameter V=string'("a"),
          parameter string NAME="a", OTHER_NAME="b",
          parameter `BYTE MASK = (W + 1),
          parameter signed [W-1:0] DELTA = 0,
          parameter cfg_t CFG=RESET,
          parameter \cfg.done ESC=p::\reset.done ,
          parameter logic TABLE[2] = '{default:1'b0},
          localparam HIDDEN=9,
          parameter type T=bit
        ) (); endmodule
        module Bare; parameter \x.y = 1; endmodule"#;
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::text(package), Source::text(source)],
        tops: &["Leaf", "Bare"],
        parameters: &[("W", "8"), ("V", "5")],
        ..Default::default()
    })
    .unwrap();
    let declarations = compilation.parameter_declarations_for("Leaf").unwrap();
    assert_eq!(declarations.imports, ["import p::*;"]);
    // Recompile the header in a new compilation unit, with no macro definitions.
    let mut header = declarations.port_list.unwrap();
    for (name, value) in [("ESC", "'1"), ("OTHER_NAME", r#""edited""#), ("W", "12")] {
        let range = declarations.parameters[name]
            .port_list_initializer
            .clone()
            .unwrap();
        header.replace_range(range, &format!(" = {value}"));
    }
    assert!(header.contains("localparam HIDDEN=9"));
    assert!(header.contains("parameter type T=bit"));
    let rebuilt = format!(
        "module Rebuilt {} {header} (); endmodule",
        declarations.imports.join(" ")
    );
    let copy = Compilation::new(&SlangConfig {
        sources: &[Source::text(package), Source::text(&rebuilt)],
        ..Default::default()
    })
    .unwrap();
    let cfg = copy
        .parameter_values_for("Rebuilt", &["CFG", "W", "OTHER_NAME", "ESC"])
        .unwrap();
    assert_eq!(
        u32::try_from(cfg["W"].value.as_integer().unwrap()).unwrap(),
        12
    );
    assert_eq!(
        cfg["OTHER_NAME"].value,
        ConstantValue::String("edited".into())
    );
    assert_eq!(
        u32::try_from(cfg["ESC"].value.as_integer().unwrap()).unwrap(),
        1
    );
    assert_eq!(
        u32::try_from(cfg["CFG"].value.as_integer().unwrap()).unwrap(),
        1
    );
    let parameters = declarations.parameters;
    assert_eq!(
        parameters.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "W",
            "V",
            "NAME",
            "OTHER_NAME",
            "MASK",
            "DELTA",
            "CFG",
            "ESC",
            "TABLE"
        ]
    );
    assert_eq!(parameters["W"].data_type.as_deref(), Some("int"));
    assert_eq!(parameters["W"].default, None);
    assert!(
        parameters["W"]
            .port_list_initializer
            .as_ref()
            .unwrap()
            .is_empty()
    );
    assert_eq!(parameters["V"].data_type, None);
    assert_eq!(parameters["V"].default.as_deref(), Some(r#"string'("a")"#));
    assert_eq!(parameters["NAME"].data_type.as_deref(), Some("string"));
    assert_eq!(
        parameters["OTHER_NAME"].data_type.as_deref(),
        Some("string")
    );
    assert_eq!(parameters["MASK"].data_type.as_deref(), Some("logic [7:0]"));
    assert_eq!(parameters["MASK"].default.as_deref(), Some("(W + 1)"));
    assert_eq!(
        parameters["DELTA"].data_type.as_deref(),
        Some("signed [W-1:0]")
    );
    assert_eq!(parameters["CFG"].data_type.as_deref(), Some("cfg_t"));
    assert_eq!(
        parameters["CFG"].package_type,
        Some(("p".into(), "cfg_t".into()))
    );
    assert_eq!(parameters["TABLE"].dimensions, "[2]");
    assert_eq!(parameters["ESC"].data_type.as_deref(), Some(r"\cfg.done "));
    assert_eq!(
        parameters["ESC"].default.as_deref(),
        Some(r"p::\reset.done ")
    );
    assert_eq!(
        compilation
            .parameter_declarations_for("Bare")
            .unwrap()
            .parameters["x.y"]
            .default
            .as_deref(),
        Some("1")
    );
    assert!(matches!(
        compilation.parameter_declarations_for("Missing"),
        Err(slang_rs::LookupError::NotFound { .. })
    ));
}
