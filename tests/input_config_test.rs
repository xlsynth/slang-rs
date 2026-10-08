// SPDX-License-Identifier: Apache-2.0

use slang_rs::{Compilation, ConstantValue, DiagnosticSeverity, SlangConfig, Source};
use std::fs;

#[test]
fn source_text_is_owned_without_creating_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("virtual source.sv");
    let compilation = {
        let name = path.to_str().unwrap().to_owned();
        let text = String::from("module top(input logic [12:0] data); endmodule");
        Compilation::new(&SlangConfig {
            sources: &[Source::Text {
                name: &name,
                text: &text,
            }],
            ..Default::default()
        })
        .unwrap()
    };
    assert!(!path.exists());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    let ports = compilation.ports().unwrap();
    drop(compilation);
    assert_eq!(ports["top"][0].ty.width().unwrap(), 13);
}

#[test]
fn source_text_and_files_share_a_compilation_and_resolve_local_includes() {
    let directory = tempfile::tempdir().unwrap();
    let source_dir = directory.path().join("virtual source directory");
    let include_dir = directory.path().join("include, files");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&include_dir).unwrap();
    fs::write(source_dir.join("local.svh"), "`define LOCAL_WIDTH 5\n").unwrap();
    fs::write(include_dir.join("extra.svh"), "`define EXTRA_WIDTH 2\n").unwrap();
    let file = directory.path().join("leaf.sv");
    fs::write(&file, "module leaf; endmodule").unwrap();
    let virtual_path = source_dir.join("top.sv");
    let compilation = Compilation::new(&SlangConfig {
        sources: &[
            Source::file(file.to_str().unwrap()),
            Source::Text {
                name: virtual_path.to_str().unwrap(),
                text: "`include \"local.svh\"
                   `include \"extra.svh\"
                   module top(input [`LOCAL_WIDTH + `EXTRA_WIDTH - 1:0] data);
                     leaf child();
                   endmodule",
            },
        ],
        incdirs: &[include_dir.to_str().unwrap()],
        tops: &["top"],
        ignore_unknown_modules: false,
        ..Default::default()
    })
    .unwrap();
    assert!(!virtual_path.exists());
    assert_eq!(
        compilation.ports().unwrap()["top"][0].ty.width().unwrap(),
        7
    );
    assert_eq!(
        compilation.hierarchy().unwrap()["top"].contents[0].def_name,
        "leaf"
    );
}

#[test]
fn literal_paths_and_definitions_preserve_spaces_and_punctuation() {
    let directory = tempfile::Builder::new()
        .prefix("--input config ")
        .tempdir_in(".")
        .unwrap();
    let source = directory.path().join("source with spaces.sv");
    let library = directory.path().join("library=with spaces.sv");
    fs::write(
        &source,
        "package text_pkg; localparam string MESSAGE = `MESSAGE; endpackage
         module top #(parameter WIDTH = 1) (input [WIDTH-1:0] data);
           leaf child();
         endmodule",
    )
    .unwrap();
    fs::write(&library, "module leaf; endmodule").unwrap();
    let source = source
        .strip_prefix(std::env::current_dir().unwrap())
        .unwrap();
    assert!(source.to_str().unwrap().starts_with("--"));
    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::file(source.to_str().unwrap())],
        libfiles: &[library.to_str().unwrap()],
        defines: &[("MESSAGE", "\"--option-like value with spaces\"")],
        parameters: &[("WIDTH", "2 + 5")],
        ignore_unknown_modules: false,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        compilation.ports().unwrap()["top"][0].ty.width().unwrap(),
        7
    );
    assert_eq!(
        compilation.packages().unwrap()["text_pkg"].parameters["MESSAGE"],
        ConstantValue::String(b"--option-like value with spaces".to_vec())
    );
    assert_eq!(
        compilation.hierarchy().unwrap()["top"].contents[0].def_name,
        "leaf"
    );
}

#[test]
fn source_text_errors_keep_the_virtual_filename_and_source_line() {
    let error = {
        let name = String::from("--virtual broken source.sv");
        let text = String::from("module broken(input logic data;\nendmodule");
        Compilation::new(&SlangConfig {
            sources: &[Source::Text {
                name: &name,
                text: &text,
            }],
            ..Default::default()
        })
        .expect_err("invalid in-memory source should return diagnostics")
    };
    assert!(
        error.diagnostics.iter().any(|diagnostic| {
            diagnostic.location.as_ref().is_some_and(|location| {
                location.file.ends_with("--virtual broken source.sv")
                    && location.line == 1
                    && location.column > 0
                    && location.source_line == "module broken(input logic data;"
            })
        }),
        "{error:?}"
    );
}

#[test]
fn invalid_explicit_include_directory_returns_a_structured_error() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing include directory");
    let error = Compilation::new(&SlangConfig {
        sources: &[Source::Text {
            name: "top.sv",
            text: "module top; endmodule",
        }],
        incdirs: &[missing.to_str().unwrap()],
        ..Default::default()
    })
    .expect_err("an invalid include directory should return an error");
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic.code == "SourceLoadError"
            && diagnostic.message.contains("missing include directory")
    }));
}

#[test]
fn directory_prefixes_resolve_configured_source_and_library_paths_in_order() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first prefix");
    let second = directory.path().join("second prefix");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let source = "--prefixed source.sv";
    let library = "--prefixed library=with spaces.sv";
    fs::write(
        first.join(source),
        "module top(input [4:0] data); leaf child(); endmodule",
    )
    .unwrap();
    fs::write(
        second.join(source),
        "module top(input [8:0] data); leaf child(); endmodule",
    )
    .unwrap();
    fs::write(second.join(library), "module leaf; endmodule").unwrap();

    let compilation = Compilation::new(&SlangConfig {
        sources: &[Source::file(source)],
        libfiles: &[library],
        tops: &["top"],
        ignore_unknown_modules: false,
        dir_prefixes: &[first.to_str().unwrap(), second.to_str().unwrap()],
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        compilation.ports().unwrap()["top"][0].ty.width().unwrap(),
        5
    );
    assert_eq!(
        compilation.hierarchy().unwrap()["top"].contents[0].def_name,
        "leaf"
    );
}

#[test]
fn excluded_extensions_filter_configured_sources_before_lookup() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("top.sv");
    let excluded = directory.path().join("excluded.skip");
    let missing = directory.path().join("missing.skip");
    let pattern = directory.path().join("*.skip");
    let library = directory.path().join("library.skip");
    fs::write(
        &source,
        "module top; disk_leaf disk_child(); memory_leaf memory_child(); endmodule",
    )
    .unwrap();
    fs::write(&excluded, "this is not valid SystemVerilog").unwrap();
    fs::write(&library, "module disk_leaf; endmodule").unwrap();

    let compilation = Compilation::new(&SlangConfig {
        sources: &[
            Source::file(source.to_str().unwrap()),
            Source::file(excluded.to_str().unwrap()),
            Source::text("memory.skip", "module memory_leaf; endmodule"),
            Source::file(missing.to_str().unwrap()),
            Source::file(pattern.to_str().unwrap()),
        ],
        libfiles: &[library.to_str().unwrap()],
        tops: &["top"],
        ignore_unknown_modules: false,
        exclude_extensions: &["skip"],
        ..Default::default()
    })
    .unwrap();
    let hierarchy = compilation.hierarchy().unwrap();
    let contents = &hierarchy["top"].contents;
    assert_eq!(contents.len(), 2);
    assert_eq!(contents[0].def_name, "disk_leaf");
    assert_eq!(contents[1].def_name, "memory_leaf");
}

#[test]
fn single_unit_preserves_interleaved_text_and_file_order() {
    let directory = tempfile::tempdir().unwrap();
    let configured = directory.path().join("configured.sv");
    fs::write(
        &configured,
        "module file_top(input [`INPUT_WIDTH-1:0] data); endmodule
         `undef INPUT_WIDTH
         `define INPUT_WIDTH 7\n",
    )
    .unwrap();

    let compilation = Compilation::new(&SlangConfig {
        sources: &[
            Source::text("definitions.sv", "`define INPUT_WIDTH 3\n"),
            Source::file(configured.to_str().unwrap()),
            Source::text(
                "buffer.sv",
                "module buffer_top(input [`INPUT_WIDTH-1:0] data); endmodule",
            ),
        ],
        single_unit: true,
        ..Default::default()
    })
    .unwrap();
    let ports = compilation.ports().unwrap();
    assert_eq!(ports["file_top"][0].ty.width().unwrap(), 3);
    assert_eq!(ports["buffer_top"][0].ty.width().unwrap(), 7);
}
#[test]
fn libraries_can_inherit_macros_from_the_shared_compilation_unit() {
    let directory = tempfile::tempdir().unwrap();
    let library = directory.path().join("leaf.sv");
    fs::write(
        &library,
        "module leaf(input [`DATA_WIDTH-1:0] data); endmodule",
    )
    .unwrap();
    let sources = [Source::Text {
        name: "top.sv",
        text: "`define DATA_WIDTH 5
               module top(input [`DATA_WIDTH-1:0] data);
                 leaf child(.data(data));
               endmodule",
    }];
    let libraries = [library.to_str().unwrap()];
    let mut config = SlangConfig {
        sources: &sources,
        libfiles: &libraries,
        tops: &["top"],
        ignore_unknown_modules: false,
        single_unit: true,
        libraries_inherit_macros: true,
        ..Default::default()
    };
    let compilation = Compilation::new(&config).unwrap();
    assert_eq!(
        compilation.hierarchy().unwrap()["top"].contents[0].def_name,
        "leaf"
    );
    config.libraries_inherit_macros = false;
    let error = Compilation::new(&config).unwrap_err();
    assert!(error.to_string().contains("DATA_WIDTH"), "{error}");
}
