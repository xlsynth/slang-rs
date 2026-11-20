// SPDX-License-Identifier: Apache-2.0

use slang_rs::{Compilation, LookupError, SlangConfig, Source};

#[test]
fn body_declarations_keep_their_scope_order_and_editable_initializers() {
    let package = "package p; typedef logic [7:0] word_t; endpackage";
    let source = r"module Leaf(output [7:0] q);
        import p::word_t;
        parameter type Word=word_t;
        parameter int W=$bits(q), DEPTH=W+1;
        localparam int PRIVATE=99;
    endmodule";
    let original = Compilation::new(&SlangConfig {
        sources: &[Source::text(package), Source::text(source)],
        ..Default::default()
    })
    .unwrap();
    let declarations = original.parameter_declarations_for("Leaf").unwrap();
    assert!(declarations.port_list.is_none());
    assert_eq!(declarations.parameter_count, 3);
    assert_eq!(declarations.declaration_names, ["Word", "W", "DEPTH"]);
    assert!(!declarations.body.contains("PRIVATE"));
    let mut body = declarations.body;
    let parameter = &declarations.parameters["W"];
    assert!(parameter.port_list_initializer.is_none());
    let range = parameter.body_initializer.clone().unwrap();
    assert_eq!(&body[range.clone()], "=$bits(q)");
    body.replace_range(range, " = 5");
    let client = format!(
        "module Leaf(output [7:0] q); {body}
           if (DEPTH != 6 || $bits(Word) != 2) $error(\"unexpected parameters\");
         endmodule
         module Top; Leaf #(.Word(logic[1:0])) u(); endmodule"
    );
    Compilation::new(&SlangConfig {
        sources: &[Source::text(package), Source::text(&client)],
        tops: &["Top"],
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn parameter_state_observes_the_elaborated_interface_at_each_specialization() {
    let source = r"module Leaf #(parameter int W=4, parameter type T=logic [W-1:0],
              localparam int Bits=$bits(T), localparam type L=T, parameter type Text=string)();
              localparam int UNUSED=7;
          endmodule";
    for (argument, width) in [("4", 4), ("8", 8)] {
        let compilation = Compilation::new(&SlangConfig {
            sources: &[Source::text(source)],
            parameters: &[("W", argument)],
            ..Default::default()
        })
        .unwrap();
        let state = compilation.parameter_state_for("Leaf").unwrap();
        let declarations = compilation.parameter_declarations_for("Leaf").unwrap();
        assert_eq!(declarations.parameter_count, 5);
        assert_eq!(
            declarations.declaration_names,
            ["W", "T", "Bits", "L", "Text"]
        );
        assert_eq!(
            declarations.parameter_count,
            state.values.len() + state.types.len()
        );
        assert_eq!(usize::try_from(&state.values["Bits"].value).unwrap(), width);
        assert_eq!(state.types["T"].as_ref().unwrap().width().unwrap(), width);
        assert_eq!(state.types["L"].as_ref().unwrap().width().unwrap(), width);
        assert!(state.types["Text"].is_err());
        assert!(!state.values.contains_key("UNUSED"));
        assert!(matches!(
            compilation.parameter_state_for("NoTop"),
            Err(LookupError::NotFound { kind: "top", .. })
        ));
    }
}

#[test]
fn parameter_declarations_distinguish_inherited_and_module_timescales() {
    use slang_rs::{TimeScale, TimeScaleValue, TimeUnit};
    let ps = TimeScaleValue {
        unit: TimeUnit::Picoseconds,
        ..Default::default()
    };
    let config = TimeScale {
        base: ps,
        precision: ps,
    };
    for (source, timescale, expected) in [
        ("module Leaf; endmodule", None, None),
        (
            "module Leaf; timeunit 10ps; timeprecision 100fs; endmodule",
            None,
            Some("10ps/100fs"),
        ),
        (
            "`timescale 10ps/1ps\nmodule Leaf; endmodule",
            Some(config),
            Some("10ps/1ps"),
        ),
        ("module Leaf; endmodule", Some(config), Some("1ps/1ps")),
    ] {
        let compilation = Compilation::new(&SlangConfig {
            sources: &[Source::text(source)],
            timescale,
            ..Default::default()
        })
        .unwrap();
        let scope = compilation
            .parameter_declarations_for("Leaf")
            .unwrap()
            .timescale
            .map(|s| s.replace(' ', ""));
        assert_eq!(scope.as_deref(), expected);
    }
}

#[test]
fn replacement_tops_retain_source_packages_and_isolate_their_compilation_unit() {
    let dependencies = r"`define PACKAGE_WIDTH 4
        typedef logic [`PACKAGE_WIDTH-1:0] unit_word_t;
        interface Bus; logic valid; endinterface
        module Device; logic original_only; endmodule
        package p;
            typedef logic [`PACKAGE_WIDTH-1:0] word_t;
            typedef virtual Bus bus_t;
        endpackage
        module Monitor; initial $display($root.Device.original_only); endmodule
        bind Device Monitor monitor();";
    let replacement = "module Device #(parameter W=$bits(p::word_t), parameter V=0)(); endmodule";
    for single_unit in [false, true] {
        let cfg = SlangConfig {
            sources: &[Source::text(dependencies)],
            tops: &["Device"],
            parameters: &[("Device.V", "9")],
            single_unit,
            ..Default::default()
        };
        let compiled = Compilation::with_replacement_tops(&cfg, replacement).unwrap();
        let state = compiled.parameter_state_for("Device").unwrap();
        assert_eq!(usize::try_from(&state.values["W"].value).unwrap(), 4);
        assert_eq!(usize::try_from(&state.values["V"].value).unwrap(), 9);
        let missing = Compilation::with_replacement_tops(
            &cfg,
            "module Device #(parameter V=0, parameter W=$bits(unit_word_t))(); endmodule",
        )
        .unwrap_err();
        assert!(missing.to_string().contains("unit_word_t"), "{missing}");
    }
}

#[test]
fn replacement_keeps_directives_in_effect_for_later_packages() {
    let original = r"`timescale 1ps/1ps
        module Device; endmodule
        package p; parameter time T=1ns; endpackage";
    let replacement = "module Device #(parameter T=p::T)(); timeunit 1ns/1ps; endmodule";
    let cfg = SlangConfig {
        sources: &[Source::text(original)],
        tops: &["Device"],
        ..Default::default()
    };
    let compiled = Compilation::with_replacement_tops(&cfg, replacement).unwrap();
    let state = compiled.parameter_state_for("Device").unwrap();
    assert_eq!(usize::try_from(&state.values["T"].value).unwrap(), 1000);
}

#[test]
fn replacement_tops_honor_library_search_and_explicit_top_selection() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("shared.sv"),
        "package shared; parameter N=7; endpackage",
    )
    .unwrap();
    let config = SlangConfig {
        libdirs: &[directory.path().to_str().unwrap()],
        libexts: &["sv"],
        hierarchy_mode: slang_rs::HierarchyMode::ExplicitTopsOnly,
        ..Default::default()
    };
    let replacement = "module Top #(parameter W=shared::N)(); endmodule";
    let unselected = Compilation::with_replacement_tops(&config, replacement).unwrap();
    assert!(unselected.top_names().unwrap().is_empty());
    let selected = SlangConfig {
        tops: &["Top"],
        ..config
    };
    let compilation = Compilation::with_replacement_tops(&selected, replacement).unwrap();
    let state = compilation.parameter_state_for("Top").unwrap();
    assert_eq!(usize::try_from(&state.values["W"].value).unwrap(), 7);
}

#[test]
fn replacement_preserves_package_enum_declaration_identity() {
    let package = "package p; typedef enum logic [1:0] {Idle, Busy} state_t; endpackage";
    let module =
        "module Device #(parameter type State=p::state_t, parameter State S=p::Busy)(); endmodule";
    let source = format!("{package} {module}");
    let config = SlangConfig {
        sources: &[Source::text(&source)],
        tops: &["Device"],
        ..Default::default()
    };
    let original = Compilation::new(&config)
        .unwrap()
        .parameter_state_for("Device")
        .unwrap();
    assert!(matches!(
        original.types["State"],
        Ok(slang_rs::Type::Enum { .. })
    ));
    assert_eq!(usize::try_from(&original.values["S"].value).unwrap(), 1);
    let replacement = Compilation::with_replacement_tops(&config, module).unwrap();
    assert_eq!(replacement.parameter_state_for("Device").unwrap(), original);
}
