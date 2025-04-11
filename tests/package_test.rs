// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use num_bigint::BigInt;
    use slang_rs::*;
    use std::collections::HashMap;

    #[test]
    fn test_extract_packages() {
        let verilog = str2tmpfile(
            "
            package pkg_a;
              localparam int a=22;
            endpackage
            package pkg_b;
              localparam int b=123;
              localparam int c=b+pkg_a::a;
              typedef logic [33:22] my_t;
            endpackage
            ",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let pkgs = extract_packages(&cfg).unwrap();

        let expected = HashMap::from([
            (
                "pkg_a".to_string(),
                Package {
                    name: "pkg_a".to_string(),
                    parameters: HashMap::from([(
                        "a".to_string(),
                        Parameter {
                            name: "a".to_string(),
                            value: "22".to_string(),
                        },
                    )]),
                },
            ),
            (
                "pkg_b".to_string(),
                Package {
                    name: "pkg_b".to_string(),
                    parameters: HashMap::from([
                        (
                            "b".to_string(),
                            Parameter {
                                name: "b".to_string(),
                                value: "123".to_string(),
                            },
                        ),
                        (
                            "c".to_string(),
                            Parameter {
                                name: "c".to_string(),
                                value: "145".to_string(),
                            },
                        ),
                    ]),
                },
            ),
        ]);

        assert_eq!(pkgs, expected);

        assert_eq!(
            pkgs["pkg_a"]["a"].parse::<BigInt>().unwrap(),
            BigInt::from(22)
        );
        assert_eq!(pkgs["pkg_b"]["b"].parse::<i32>().unwrap(), 123);
        assert_eq!(pkgs["pkg_b"]["c"].parse::<u64>().unwrap(), 145u64);
    }

    #[test]
    #[should_panic(expected = "slang command failed")]
    fn test_extract_packages_error() {
        let verilog = str2tmpfile(
            "
            package A
            endpackage
            ",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        extract_packages(&cfg).unwrap();
    }
}
