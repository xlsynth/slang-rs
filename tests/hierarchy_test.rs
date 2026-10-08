// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use indexmap::indexmap;
    use slang_rs::*;
    use std::collections::HashMap;

    #[test]
    fn test_extract_hierarchy() {
        let verilog = "
            module A;
              B b0();
            endmodule
            module B;
              C c0();
              C c1();
            endmodule
            module C;
            endmodule
            ";

        let cfg = SlangConfig {
            sources: &[Source::Text {
                name: "design.sv",
                text: verilog,
            }],
            ..Default::default()
        };

        let hierarchy = Compilation::new(&cfg).unwrap().hierarchy();

        let expected = Instance {
            def_name: "A".to_string(),
            inst_name: "A".to_string(),
            path: "A".to_string(),
            contents: indexmap! {
                "b0".to_string() => Instance {
                    def_name: "B".to_string(),
                    inst_name: "b0".to_string(),
                    path: "A.b0".to_string(),
                    contents: indexmap! {
                        "c0".to_string() => Instance {
                            def_name: "C".to_string(),
                            inst_name: "c0".to_string(),
                            path: "A.b0.c0".to_string(),
                            contents: Default::default(),
                        },
                        "c1".to_string() => Instance {
                            def_name: "C".to_string(),
                            inst_name: "c1".to_string(),
                            path: "A.b0.c1".to_string(),
                            contents: Default::default(),
                        },
                    },
                },
            },
        };

        let expected = HashMap::from([("A".to_string(), expected)]);

        let hierarchy = hierarchy.unwrap();
        assert_eq!(hierarchy, expected);

        let top = &hierarchy["A"];
        assert_eq!(hierarchy["A"]["b0"]["c1"].path, "A.b0.c1");
        assert_eq!(hierarchy["A"][0][1].path, "A.b0.c1");
        assert!(std::ptr::eq(top.child("b0").unwrap(), &top.contents[0]));
        assert!(top.child("").is_none());
        assert!(top.child("missing").is_none());
        assert!(top.child("A.b0").is_none());
        assert!(top.child("b0.c1").is_none());
        assert!(top["b0"]["c0"].child("missing").is_none());
        for (index, child) in top["b0"].values().enumerate() {
            assert!(std::ptr::eq(&top["b0"][index], child));
        }
        let mut names = Vec::new();
        for child in &hierarchy["A"]["b0"] {
            assert!(std::ptr::eq(&top["b0"][names.len()], child));
            names.push(child.inst_name.as_str());
        }
        assert_eq!(names, ["c0", "c1"]);
    }

    #[test]
    fn test_extract_hierarchy_genblk() {
        // Test verilog adapted from tests/unittests/ast/HierarchyTests.cpp
        // in https://github.com/MikePopoloski/slang

        let verilog = "
module A;
endmodule
module B;
endmodule
module top;
    parameter genblk2 = 0;
    genvar i;

    // The following generate block is implicitly named genblk1
    if (genblk2) A a(); // top.genblk1.a
    else B b(); // top.genblk1.b

    // The following generate block is implicitly named genblk02
    // as genblk2 is already a declared identifier
    if (genblk2) A a(); // top.genblk02.a
    else B b(); // top.genblk02.b

    // The following generate block would have been named genblk3
    // but is explicitly named g1
    for (i = 0; i < 1; i = i + 1) begin : g1 // block name
        // The following generate block is implicitly named genblk1
        // as the first nested scope inside g1
        if (1) A a(); // top.g1[0].genblk1.a
    end

    // The following generate block is implicitly named genblk4 since
    // it belongs to the fourth generate construct in scope 'top'.
    // The previous generate block would have been
    // named genblk3 if it had not been explicitly named g1
    for (i = 0; i < 1; i = i + 1)
        // The following generate block is implicitly named genblk1
        // as the first nested generate block in genblk4
        if (1) A a(); // top.genblk4[0].genblk1.a

    // The following generate block is implicitly named genblk5
    if (1) A a(); // top.genblk5.a
endmodule
            ";

        let cfg = SlangConfig {
            sources: &[Source::Text {
                name: "design.sv",
                text: verilog,
            }],
            ..Default::default()
        };

        let hierarchy = Compilation::new(&cfg).unwrap().hierarchy();

        let expected = Instance {
            def_name: "top".to_string(),
            inst_name: "top".to_string(),
            path: "top".to_string(),
            contents: indexmap! {
                "genblk1.b".to_string() => Instance {
                    def_name: "B".to_string(),
                    inst_name: "b".to_string(),
                    path: "top.genblk1.b".to_string(),
                    contents: Default::default(),
                },
                "genblk02.b".to_string() => Instance {
                    def_name: "B".to_string(),
                    inst_name: "b".to_string(),
                    path: "top.genblk02.b".to_string(),
                    contents: Default::default(),
                },
                "g1[0].genblk1.a".to_string() => Instance {
                    def_name: "A".to_string(),
                    inst_name: "a".to_string(),
                    path: "top.g1[0].genblk1.a".to_string(),
                    contents: Default::default(),
                },
                "genblk4[0].genblk1.a".to_string() => Instance {
                    def_name: "A".to_string(),
                    inst_name: "a".to_string(),
                    path: "top.genblk4[0].genblk1.a".to_string(),
                    contents: Default::default(),
                },
                "genblk5.a".to_string() => Instance {
                    def_name: "A".to_string(),
                    inst_name: "a".to_string(),
                    path: "top.genblk5.a".to_string(),
                    contents: Default::default(),
                },
            },
        };

        let expected = HashMap::from([("top".to_string(), expected)]);

        let hierarchy = hierarchy.unwrap();
        assert_eq!(hierarchy, expected);

        let top = &hierarchy["top"];
        assert_eq!(top["genblk1.b"].path, "top.genblk1.b");
        assert_eq!(top["genblk02.b"].path, "top.genblk02.b");
        assert_eq!(top["g1[0].genblk1.a"].path, "top.g1[0].genblk1.a");
        assert!(top.child("b").is_none());
        assert!(top.child("g1[0]").is_none());
        for (index, child) in top.contents.values().enumerate() {
            assert!(std::ptr::eq(&top[index], child));
        }
    }

    #[test]
    fn test_extract_hierarchy_error() {
        let verilog = "
            module A
            endmodule
            ";

        let cfg = SlangConfig {
            sources: &[Source::Text {
                name: "design.sv",
                text: verilog,
            }],
            ..Default::default()
        };

        let error = Compilation::new(&cfg).unwrap_err();
        assert!(error.to_string().contains("expected"));
        assert!(!error.diagnostics.is_empty());
    }

    #[test]
    fn test_unknown_module() {
        let verilog = "
            module E;
            endmodule
            module A(
              input clk
            );
              B b();
              if (1) begin
                C c();
              end
              if (0) begin
                D d();
              end
              E e();
            endmodule
            ";

        let cfg = SlangConfig {
            sources: &[Source::Text {
                name: "design.sv",
                text: verilog,
            }],
            ..Default::default()
        };

        let hierarchy = Compilation::new(&cfg).unwrap().hierarchy().unwrap();

        let expected = HashMap::from([(
            "A".to_string(),
            Instance {
                def_name: "A".to_string(),
                inst_name: "A".to_string(),
                path: "A".to_string(),
                contents: indexmap! {
                    "b".to_string() => Instance {
                        def_name: "B".to_string(),
                        inst_name: "b".to_string(),
                        path: "A.b".to_string(),
                        contents: Default::default(),
                    },
                    "genblk1.c".to_string() => Instance {
                        def_name: "C".to_string(),
                        inst_name: "c".to_string(),
                        path: "A.genblk1.c".to_string(),
                        contents: Default::default(),
                    },
                    "e".to_string() => Instance {
                        def_name: "E".to_string(),
                        inst_name: "e".to_string(),
                        path: "A.e".to_string(),
                        contents: Default::default(),
                    },
                },
            },
        )]);

        assert_eq!(hierarchy, expected);
    }

    #[test]
    fn native_paths_preserve_escaped_identifiers_and_nested_generate_scopes() {
        let hierarchy = {
            let source = String::from(
                r"
                module leaf;
                endmodule
                module branch;
                    leaf \nested.leaf ();
                endmodule
                module top;
                    for (genvar i = -1; i <= 0; i++) begin : \stage.with.dot
                        if (i == -1) begin : chosen
                            branch \child.with.dot ();
                        end else begin : other
                            leaf \other.leaf ();
                        end
                        if (0) begin : inactive
                            leaf hidden();
                        end
                    end
                    missing \unknown.with.dot ();
                endmodule
                ",
            );
            let compilation = Compilation::new(&SlangConfig {
                sources: &[Source::Text {
                    name: "escaped.sv",
                    text: &source,
                }],
                tops: &["top"],
                ..Default::default()
            })
            .unwrap();
            compilation.hierarchy().unwrap()
        };

        let top = &hierarchy["top"];
        assert_eq!(top.path, "top");
        assert_eq!(top.contents.len(), 3);
        let branch = &top.contents[0];
        assert_eq!(branch.inst_name, "child.with.dot");
        assert_eq!(
            branch.path,
            r"top.\stage.with.dot [-1].chosen.\child.with.dot "
        );
        assert_eq!(branch.contents.len(), 1);
        assert_eq!(branch.contents[0].inst_name, "nested.leaf");
        assert_eq!(
            branch.contents[0].path,
            r"top.\stage.with.dot [-1].chosen.\child.with.dot .\nested.leaf "
        );
        assert_eq!(top.contents[1].inst_name, "other.leaf");
        assert_eq!(
            top.contents[1].path,
            r"top.\stage.with.dot [0].other.\other.leaf "
        );
        assert_eq!(top.contents[2].def_name, "missing");
        assert_eq!(top.contents[2].inst_name, "unknown.with.dot");
        assert_eq!(top.contents[2].path, r"top.\unknown.with.dot ");

        let branch_key = r"\stage.with.dot [-1].chosen.\child.with.dot ";
        assert!(std::ptr::eq(top.child(branch_key).unwrap(), branch));
        assert_eq!(
            hierarchy["top"][branch_key][r"\nested.leaf "].path,
            branch.contents[0].path
        );
        assert!(std::ptr::eq(
            &top[r"\stage.with.dot [0].other.\other.leaf "],
            &top.contents[1]
        ));
        assert!(std::ptr::eq(&top[r"\unknown.with.dot "], &top.contents[2]));
        assert!(top.child("child.with.dot").is_none());
        assert!(top.child(branch_key.trim_end()).is_none());
        assert!(branch.child("nested.leaf").is_none());
        assert!(branch.child(r"\nested.leaf").is_none());
    }

    #[test]
    fn native_paths_preserve_multidimensional_instance_array_indices() {
        let hierarchy = Compilation::new(&SlangConfig {
            sources: &[Source::Text {
                name: "instance_arrays.sv",
                text: "
                    module leaf;
                    endmodule
                    module branch;
                        leaf child();
                    endmodule
                    module top;
                        branch elements[2:1][-2:-1]();
                        leaf ascending[4:5]();
                        leaf descending[-1:-2]();
                    endmodule
                ",
            }],
            tops: &["top"],
            ..Default::default()
        })
        .unwrap()
        .hierarchy()
        .unwrap();

        let top = &hierarchy["top"];
        let children = &top.contents;
        assert_eq!(
            children
                .values()
                .map(|child| child.path.as_str())
                .collect::<Vec<_>>(),
            [
                "top.elements[1][-2]",
                "top.elements[1][-1]",
                "top.elements[2][-2]",
                "top.elements[2][-1]",
                "top.ascending[4]",
                "top.ascending[5]",
                "top.descending[-2]",
                "top.descending[-1]",
            ]
        );
        for element in children.values().take(4) {
            assert_eq!(element.inst_name, "elements");
            assert_eq!(element.contents.len(), 1);
            assert_eq!(element.contents[0].inst_name, "child");
            assert_eq!(element.contents[0].path, format!("{}.child", element.path));
        }
        assert!(
            children
                .values()
                .skip(4)
                .take(2)
                .all(|child| child.inst_name == "ascending")
        );
        assert!(
            children
                .values()
                .skip(6)
                .all(|child| child.inst_name == "descending")
        );
        assert_eq!(
            hierarchy["top"]["elements[2][-1]"]["child"].path,
            "top.elements[2][-1].child"
        );
        assert!(top.child("elements").is_none());
        assert!(top.child("elements[2]").is_none());
        assert!(top.child("elements[0][-1]").is_none());
        for (index, child) in children.values().enumerate() {
            let relative_path = child.path.strip_prefix("top.").unwrap();
            assert!(std::ptr::eq(top.child(relative_path).unwrap(), child));
            assert!(std::ptr::eq(&top[relative_path], &top[index]));
        }
    }
}
