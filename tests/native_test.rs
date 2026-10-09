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
        sources: &[Source::Text {
            name: "session.sv",
            text: source,
        }],
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
            sources: &[Source::Text {
                name: "owned.sv",
                text: &source,
            }],
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
        sources: &[Source::Text {
            name: "broken.sv",
            text: "module broken(input logic a;\nendmodule\n",
        }],
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
                    sources: &[Source::Text {
                        name: "thread.sv",
                        text: &source,
                    }],
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
        sources: &[Source::Text {
            name: "interface.sv",
            text: source,
        }],
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
        sources: &[Source::Text {
            name: "anonymous.sv",
            text: source,
        }],
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
            "ordered.sv",
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
            "unnamed.sv",
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
