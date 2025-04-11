// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use slang_rs::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn test_extract_all() {
        let verilog = str2tmpfile(
            "
            module B;
            endmodule
            module A(
              input x,
              output y
            );
              B b();
            endmodule
            package mypkg;
              localparam int myparam=42;
            endpackage
            ",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let value = run_slang(&cfg).unwrap();

        // check ports

        let ports = extract_ports_from_value(&value, false);

        let expected_ports = HashMap::from([(
            "A".to_string(),
            vec![
                Port {
                    name: "x".to_string(),
                    dir: PortDir::Input,
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    name: "y".to_string(),
                    dir: PortDir::Output,
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
            ],
        )]);

        assert_eq!(ports, expected_ports);

        // check hierarchy

        let hierarchy = extract_hierarchy_from_value(&value);

        let expected_hierarchy = HashMap::from([(
            "A".to_string(),
            Instance {
                def_name: "A".to_string(),
                inst_name: "A".to_string(),
                hier_prefix: "".to_string(),
                contents: vec![Rc::new(RefCell::new(Instance {
                    def_name: "B".to_string(),
                    inst_name: "b".to_string(),
                    hier_prefix: "".to_string(),
                    contents: vec![],
                }))],
            },
        )]);

        assert_eq!(hierarchy, expected_hierarchy);

        // check packages

        let packages = extract_packages_from_value(&value);

        let expected_packages = HashMap::from([(
            "mypkg".to_string(),
            Package {
                name: "mypkg".to_string(),
                parameters: HashMap::from([(
                    "myparam".to_string(),
                    Parameter {
                        name: "myparam".to_string(),
                        value: "42".to_string(),
                    },
                )]),
            },
        )]);

        assert_eq!(packages, expected_packages);
    }
}
