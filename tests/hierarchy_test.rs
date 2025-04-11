// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use slang_rs::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn test_extract_hierarchy() {
        let verilog = str2tmpfile(
            "
            module A;
              B b0();
            endmodule
            module B;
              C c0();
              C c1();
            endmodule
            module C;
            endmodule
            ",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let hierarchy = extract_hierarchy(&cfg);

        let expected = Instance {
            def_name: "A".to_string(),
            inst_name: "A".to_string(),
            hier_prefix: "".to_string(),
            contents: vec![Rc::new(RefCell::new(Instance {
                def_name: "B".to_string(),
                inst_name: "b0".to_string(),
                hier_prefix: "".to_string(),
                contents: vec![
                    Rc::new(RefCell::new(Instance {
                        def_name: "C".to_string(),
                        inst_name: "c0".to_string(),
                        hier_prefix: "".to_string(),
                        contents: vec![],
                    })),
                    Rc::new(RefCell::new(Instance {
                        def_name: "C".to_string(),
                        inst_name: "c1".to_string(),
                        hier_prefix: "".to_string(),
                        contents: vec![],
                    })),
                ],
            }))],
        };

        let expected = HashMap::from([("A".to_string(), expected)]);

        assert_eq!(hierarchy.unwrap(), expected);
    }

    #[test]
    fn test_extract_hierarchy_genblk() {
        // Test verilog adapted from tests/unittests/ast/HierarchyTests.cpp
        // in https://github.com/MikePopoloski/slang

        let verilog = str2tmpfile(
            "
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
            ",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let hierarchy = extract_hierarchy(&cfg);

        let expected = Instance {
            def_name: "top".to_string(),
            inst_name: "top".to_string(),
            hier_prefix: "".to_string(),
            contents: vec![
                Rc::new(RefCell::new(Instance {
                    def_name: "B".to_string(),
                    inst_name: "b".to_string(),
                    hier_prefix: ".genblk1".to_string(),
                    contents: vec![],
                })),
                Rc::new(RefCell::new(Instance {
                    def_name: "B".to_string(),
                    inst_name: "b".to_string(),
                    hier_prefix: ".genblk02".to_string(),
                    contents: vec![],
                })),
                Rc::new(RefCell::new(Instance {
                    def_name: "A".to_string(),
                    inst_name: "a".to_string(),
                    hier_prefix: ".g1[0].genblk1".to_string(),
                    contents: vec![],
                })),
                Rc::new(RefCell::new(Instance {
                    def_name: "A".to_string(),
                    inst_name: "a".to_string(),
                    hier_prefix: ".genblk4[0].genblk1".to_string(),
                    contents: vec![],
                })),
                Rc::new(RefCell::new(Instance {
                    def_name: "A".to_string(),
                    inst_name: "a".to_string(),
                    hier_prefix: ".genblk5".to_string(),
                    contents: vec![],
                })),
            ],
        };

        let expected = HashMap::from([("top".to_string(), expected)]);

        assert_eq!(hierarchy.unwrap(), expected);
    }

    #[test]
    #[should_panic(expected = "slang command failed")]
    fn test_extract_hierarchy_error() {
        let verilog = str2tmpfile(
            "
            module A
            endmodule
            ",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        extract_hierarchy(&cfg).unwrap();
    }
}
