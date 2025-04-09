// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use slang_rs::*;
    use std::cell::RefCell;
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
            contents: vec![Rc::new(RefCell::new(Instance {
                def_name: "B".to_string(),
                inst_name: "b0".to_string(),
                contents: vec![
                    Rc::new(RefCell::new(Instance {
                        def_name: "C".to_string(),
                        inst_name: "c0".to_string(),
                        contents: vec![],
                    })),
                    Rc::new(RefCell::new(Instance {
                        def_name: "C".to_string(),
                        inst_name: "c1".to_string(),
                        contents: vec![],
                    })),
                ],
            }))],
        };

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

    #[test]
    #[should_panic(expected = "Top-level module not found")]
    fn test_top_level_not_found() {
        let verilog = str2tmpfile("").unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        extract_hierarchy(&cfg).unwrap();
    }
}
