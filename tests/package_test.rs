// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use num_bigint::{BigInt, BigUint};
    use slang_rs::*;
    use std::collections::HashMap;

    fn integer(width: usize, signed: bool, bits: impl Into<BigUint>) -> IntegerValue {
        IntegerValue {
            width,
            signed,
            bits: bits.into(),
            x_mask: BigUint::from(0u8),
            z_mask: BigUint::from(0u8),
        }
    }

    fn integer_constant(width: usize, signed: bool, bits: impl Into<BigUint>) -> ConstantValue {
        ConstantValue::Integer(integer(width, signed, bits))
    }

    #[test]
    fn test_extract_packages() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
            package pkg_a;
              localparam int a=22;
            endpackage
            package pkg_b;
              localparam int b=123;
              localparam int c=b+pkg_a::a;
              typedef logic [33:22] my_t;
            endpackage
            ",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let pkgs = Compilation::new(&cfg).unwrap().packages().unwrap();

        let expected = HashMap::from([
            (
                "pkg_a".to_string(),
                Package {
                    name: "pkg_a".to_string(),
                    parameters: HashMap::from([(
                        "a".to_string(),
                        integer_constant(32, true, 22u32),
                    )]),
                    types: HashMap::new(),
                },
            ),
            (
                "pkg_b".to_string(),
                Package {
                    name: "pkg_b".to_string(),
                    parameters: HashMap::from([
                        ("b".to_string(), integer_constant(32, true, 123u32)),
                        ("c".to_string(), integer_constant(32, true, 145u32)),
                    ]),
                    types: HashMap::from([(
                        "my_t".to_string(),
                        Ok(Type::Integral {
                            kind: IntegralKind::Logic,
                            four_state: true,
                            signed: false,
                            packed_dimensions: vec![Range {
                                left: 33,
                                right: 22,
                            }],
                            unpacked_dimensions: vec![],
                            bit_width: Some(12),
                        }),
                    )]),
                },
            ),
        ]);

        assert_eq!(pkgs, expected);

        assert_eq!(i32::try_from(&pkgs["pkg_a"].parameters["a"]).unwrap(), 22);
        let b: i32 = (&pkgs["pkg_b"].parameters["b"]).try_into().unwrap();
        assert_eq!(b, 123);
        assert_eq!(i32::try_from(&pkgs["pkg_b"].parameters["c"]).unwrap(), 145);
    }

    #[test]
    fn test_extract_packages_error() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
            package A
            endpackage
            ",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let error = Compilation::new(&cfg).unwrap_err();
        assert!(error.to_string().contains("expected"), "{error}");
        assert!(!error.diagnostics.is_empty());
    }

    #[test]
    fn test_extract_type_alias() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
            package my_pkg;
              typedef struct packed {
                logic [2:0] a;
                logic [1:0] b;
                logic c;
              } my_struct_t;
              typedef enum logic [1:0] {
                Red = 0,
                Green = 1,
                Blue = 2
              } my_enum_t;
              typedef my_struct_t [3:0] my_array_t;
            endpackage
            ",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let pkgs = Compilation::new(&cfg).unwrap().packages().unwrap();

        assert_eq!(
            pkgs["my_pkg"].types["my_struct_t"]
                .as_ref()
                .unwrap()
                .width()
                .unwrap(),
            6
        );

        assert_eq!(
            pkgs["my_pkg"].types["my_enum_t"]
                .as_ref()
                .unwrap()
                .width()
                .unwrap(),
            2
        );

        assert_eq!(
            pkgs["my_pkg"].types["my_array_t"]
                .as_ref()
                .unwrap()
                .width()
                .unwrap(),
            24
        );
    }

    #[test]
    fn package_types_resolve_aliases_ranges_and_composite_members() {
        let source = Source::Text {
            name: "source.sv",
            text: "package base;
               localparam int WIDTH = 4, COUNT = 2, BASE = -2;
               typedef logic signed [BASE:BASE+WIDTH-1] lane_t;
             endpackage
             package shapes;
               typedef base::lane_t lane_alias_t;
               typedef lane_alias_t [base::COUNT-1:0] packet_t;
               typedef packet_t matrix_t [base::BASE:base::BASE+base::COUNT-1];
               typedef int signed count_t;
               typedef struct { int count; byte tag; } record_t;
               typedef longint unsigned timestamp_t;
               typedef struct packed {
                 base::lane_t high;
                 lane_alias_t low;
               } pair_t;
               typedef union packed {
                 pair_t fields;
                 logic [7:0] raw;
               } word_t;
               typedef enum logic signed [79:0] {
                 Negative = -80'sd18446744073709551617,
                 Positive = 80'sd1099511627776
               } wide_enum_t;
             endpackage",
        };
        let packages = Compilation::new(&SlangConfig {
            sources: &[source],
            ..Default::default()
        })
        .unwrap()
        .packages()
        .unwrap();
        let types = &packages["shapes"].types;

        assert_eq!(
            types["lane_alias_t"].as_ref().unwrap(),
            &Type::Integral {
                kind: IntegralKind::Logic,
                four_state: true,
                signed: true,
                packed_dimensions: vec![Range { left: -2, right: 1 }],
                unpacked_dimensions: vec![],
                bit_width: Some(4),
            }
        );
        assert_eq!(types["packet_t"].as_ref().unwrap().width().unwrap(), 8);
        let record = types["record_t"].as_ref().unwrap();
        assert!(record.width().is_err());
        assert!(matches!(
            record,
            Type::Struct {
                is_packed: false,
                ..
            }
        ));
        let matrix = types["matrix_t"].as_ref().unwrap();
        assert_eq!(
            matrix,
            &Type::Integral {
                kind: IntegralKind::Logic,
                four_state: true,
                signed: true,
                packed_dimensions: vec![Range { left: 1, right: 0 }, Range { left: -2, right: 1 },],
                unpacked_dimensions: vec![Range {
                    left: -2,
                    right: -1
                }],
                bit_width: None,
            }
        );
        assert!(matrix.width().is_err());
        for (name, kind, signed, bits) in [
            ("count_t", IntegralKind::Int, true, 32),
            ("timestamp_t", IntegralKind::LongInt, false, 64),
        ] {
            assert_eq!(
                types[name].as_ref().unwrap(),
                &Type::Integral {
                    kind,
                    four_state: false,
                    signed,
                    packed_dimensions: vec![],
                    unpacked_dimensions: vec![],
                    bit_width: Some(bits as usize),
                }
            );
        }

        let word = types["word_t"].as_ref().unwrap();
        assert_eq!(word.width().unwrap(), 8);
        let Type::Union {
            fields, is_packed, ..
        } = word
        else {
            panic!("expected a union, got {word:?}");
        };
        assert!(is_packed);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "fields");
        assert_eq!(fields[1].name, "raw");
        assert_eq!(fields[1].ty.width().unwrap(), 8);
        let Type::Struct {
            fields, is_packed, ..
        } = &fields[0].ty
        else {
            panic!("expected a nested struct");
        };
        assert!(is_packed);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "high");
        assert_eq!(fields[1].name, "low");
        assert_eq!(fields[0].ty, *types["lane_alias_t"].as_ref().unwrap());
        assert_eq!(fields[1].ty, *types["lane_alias_t"].as_ref().unwrap());

        let enumeration = types["wide_enum_t"].as_ref().unwrap();
        assert_eq!(enumeration.width().unwrap(), 80);
        let Type::Enum { variants, .. } = enumeration else {
            panic!("expected an enum, got {enumeration:?}");
        };
        assert_eq!(
            variants,
            &vec![
                Variant {
                    name: "Negative".to_string(),
                    value: integer(
                        80,
                        true,
                        (BigUint::from(1u8) << 80usize)
                            - (BigUint::from(1u8) << 64usize)
                            - BigUint::from(1u8)
                    ),
                },
                Variant {
                    name: "Positive".to_string(),
                    value: integer(80, true, 1u64 << 40usize),
                },
            ]
        );
    }

    #[test]
    fn native_types_preserve_primitive_kinds_dimensions_and_enum_bases() {
        let packages = Compilation::new(&SlangConfig {
            sources: &[Source::Text {
                name: "native_types.sv",
                text: "package primitives;
                  typedef bit bit_t;
                  typedef logic logic_t;
                  typedef reg reg_t;
                  typedef byte byte_t;
                  typedef shortint shortint_t;
                  typedef int int_t;
                  typedef longint longint_t;
                  typedef integer integer_t;
                  typedef time time_t;
                  typedef bit signed [-2:3] signed_bits_t;
                  typedef int unsigned unsigned_int_t;
                  typedef time signed signed_time_t;
                  typedef int_t [1:0] int_pair_t;
                  typedef int_t int_array_t [-1:2];
                  typedef struct packed signed { bit [3:0] field; } signed_struct_t;
                  typedef union packed signed { bit [3:0] bits; logic [3:0] value; } signed_union_t;
                  typedef enum { DefaultA, DefaultB } default_enum_t;
                  typedef enum bit signed [3:0] { Negative = -1, Zero = 0 } bit_enum_t;
                endpackage",
            }],
            ..Default::default()
        })
        .unwrap()
        .packages()
        .unwrap();
        let types = &packages["primitives"].types;
        for (name, kind, signed, four_state, width) in [
            ("bit_t", IntegralKind::Bit, false, false, 1),
            ("logic_t", IntegralKind::Logic, false, true, 1),
            ("reg_t", IntegralKind::Reg, false, true, 1),
            ("byte_t", IntegralKind::Byte, true, false, 8),
            ("shortint_t", IntegralKind::ShortInt, true, false, 16),
            ("int_t", IntegralKind::Int, true, false, 32),
            ("longint_t", IntegralKind::LongInt, true, false, 64),
            ("integer_t", IntegralKind::Integer, true, true, 32),
            ("time_t", IntegralKind::Time, false, true, 64),
            ("unsigned_int_t", IntegralKind::Int, false, false, 32),
            ("signed_time_t", IntegralKind::Time, true, true, 64),
        ] {
            assert_eq!(
                types[name].as_ref().unwrap(),
                &Type::Integral {
                    kind,
                    signed,
                    four_state,
                    packed_dimensions: vec![],
                    unpacked_dimensions: vec![],
                    bit_width: Some(width),
                },
                "{name}"
            );
        }
        assert_eq!(
            types["signed_bits_t"].as_ref().unwrap(),
            &Type::Integral {
                kind: IntegralKind::Bit,
                signed: true,
                four_state: false,
                packed_dimensions: vec![Range { left: -2, right: 3 }],
                unpacked_dimensions: vec![],
                bit_width: Some(6),
            }
        );
        assert_eq!(
            types["int_pair_t"].as_ref().unwrap(),
            &Type::Integral {
                kind: IntegralKind::Int,
                signed: true,
                four_state: false,
                packed_dimensions: vec![Range { left: 1, right: 0 }],
                unpacked_dimensions: vec![],
                bit_width: Some(64),
            }
        );
        assert_eq!(
            types["int_array_t"].as_ref().unwrap(),
            &Type::Integral {
                kind: IntegralKind::Int,
                signed: true,
                four_state: false,
                packed_dimensions: vec![],
                unpacked_dimensions: vec![Range { left: -1, right: 2 }],
                bit_width: None,
            }
        );
        assert!(matches!(
            types["signed_struct_t"].as_ref().unwrap(),
            Type::Struct {
                signed: true,
                four_state: false,
                ..
            }
        ));
        assert!(matches!(
            types["signed_union_t"].as_ref().unwrap(),
            Type::Union {
                signed: true,
                four_state: true,
                ..
            }
        ));
        for (name, expected_kind, expected_bits, expected_dimensions) in [
            ("default_enum_t", IntegralKind::Int, 32, vec![]),
            (
                "bit_enum_t",
                IntegralKind::Bit,
                4,
                vec![Range { left: 3, right: 0 }],
            ),
        ] {
            let Type::Enum {
                signed,
                four_state,
                base_type,
                packed_dimensions,
                unpacked_dimensions,
                ..
            } = types[name].as_ref().unwrap()
            else {
                panic!("expected enum {name}");
            };
            assert!(*signed);
            assert!(!four_state);
            assert!(packed_dimensions.is_empty());
            assert!(unpacked_dimensions.is_empty());
            assert_eq!(
                base_type.as_ref(),
                &Type::Integral {
                    kind: expected_kind,
                    signed: true,
                    four_state: false,
                    packed_dimensions: expected_dimensions,
                    unpacked_dimensions: vec![],
                    bit_width: Some(expected_bits),
                }
            );
        }
    }

    #[test]
    fn unsupported_package_types_do_not_hide_supported_types_or_constants() {
        let source = Source::Text {
            name: "package_types.sv",
            text: "package mixed;
               localparam int VALUE = 17;
               typedef logic [5:0] supported_t;
               typedef string text_t;
               typedef int samples_t[];
             endpackage",
        };
        let packages = Compilation::new(&SlangConfig {
            sources: &[source],
            ..Default::default()
        })
        .unwrap()
        .packages()
        .unwrap();
        let package = &packages["mixed"];
        assert_eq!(
            package.parameters["VALUE"],
            integer_constant(32, true, 17u32)
        );
        assert_eq!(package.parameters.len(), 1);
        assert_eq!(package.types.len(), 3);
        assert_eq!(
            package.types["supported_t"]
                .as_ref()
                .unwrap()
                .width()
                .unwrap(),
            6
        );
        for name in ["text_t", "samples_t"] {
            let error = package.types[name].as_ref().unwrap_err().to_string();
            assert!(error.contains(name), "{error}");
            assert!(error.contains("package_types.sv"), "{error}");
        }
    }
    #[test]
    fn native_constants_preserve_integer_width_sign_and_four_state_bits() {
        let source = Source::Text {
            name: "source.sv",
            text: r"package constants;
                localparam logic signed [4:0] SMALL_NEGATIVE = -5'sd3;
                localparam logic signed [79:0] SIGNED_WIDE = -80'sd18446744073709551617;
                localparam logic [79:0] UNSIGNED_WIDE = 80'h8000_0000_0000_0000_0001;
                localparam logic [69:0] FOUR_STATE = {4'b10xz, 62'b0, 4'bzx10};
                typedef enum logic [3:0] { Unknown = 4'bx1z0 } four_state_t;
              endpackage",
        };
        let packages = Compilation::new(&SlangConfig {
            sources: &[source],
            ..Default::default()
        })
        .unwrap()
        .packages()
        .unwrap();
        let package = &packages["constants"];
        let small = package.parameters["SMALL_NEGATIVE"].as_integer().unwrap();
        assert_eq!(small, &integer(5, true, 29u8));
        assert_eq!(BigInt::try_from(small).unwrap(), BigInt::from(-3));

        let signed = package.parameters["SIGNED_WIDE"].as_integer().unwrap();
        assert_eq!(
            signed,
            &integer(
                80,
                true,
                (BigUint::from(1u8) << 80usize)
                    - (BigUint::from(1u8) << 64usize)
                    - BigUint::from(1u8)
            )
        );
        assert_eq!(
            BigInt::try_from(signed).unwrap(),
            -((BigInt::from(1u8) << 64usize) + BigInt::from(1u8))
        );
        let unsigned = package.parameters["UNSIGNED_WIDE"].as_integer().unwrap();
        assert_eq!(
            unsigned,
            &integer(
                80,
                false,
                (BigUint::from(1u8) << 79usize) + BigUint::from(1u8)
            )
        );
        assert_eq!(
            BigInt::try_from(unsigned).unwrap(),
            (BigInt::from(1u8) << 79usize) + BigInt::from(1u8)
        );

        let four_state = package.parameters["FOUR_STATE"].as_integer().unwrap();
        assert_eq!(
            four_state,
            &IntegerValue {
                width: 70,
                signed: false,
                bits: (BigUint::from(1u8) << 69usize) | BigUint::from(2u8),
                x_mask: (BigUint::from(1u8) << 67usize) | BigUint::from(4u8),
                z_mask: (BigUint::from(1u8) << 66usize) | BigUint::from(8u8),
            }
        );
        assert!(BigInt::try_from(four_state).is_err());
        let Type::Enum { variants, .. } = package.types["four_state_t"].as_ref().unwrap() else {
            panic!("expected an enum");
        };
        assert_eq!(
            variants,
            &vec![Variant {
                name: "Unknown".into(),
                value: IntegerValue {
                    width: 4,
                    signed: false,
                    bits: BigUint::from(4u8),
                    x_mask: BigUint::from(8u8),
                    z_mask: BigUint::from(2u8),
                },
            }]
        );
    }

    #[test]
    fn native_constants_preserve_real_values_and_string_bytes() {
        let source = Source::Text {
            name: "source.sv",
            text: r#"package constants;
                 localparam real DOUBLE = -3.25;
                 localparam shortreal SINGLE = 1.5;
                 localparam string TEXT = "A\377\200Z";
               endpackage"#,
        };
        let packages = Compilation::new(&SlangConfig {
            sources: &[source],
            ..Default::default()
        })
        .unwrap()
        .packages()
        .unwrap();
        let package = &packages["constants"];
        assert_eq!(package.parameters["DOUBLE"], ConstantValue::Real(-3.25));
        assert_eq!(package.parameters["SINGLE"], ConstantValue::ShortReal(1.5));
        assert_eq!(
            package.parameters["TEXT"],
            ConstantValue::String(b"A\xff\x80Z".to_vec())
        );
        assert!(package.parameters["DOUBLE"].as_integer().is_none());
    }

    #[test]
    fn native_constants_preserve_nested_arrays_queues_maps_and_unions() {
        let source = Source::Text {
            name: "source.sv",
            text: r"package constants;
                localparam int MATRIX [2][2] = '{'{1, 2}, '{3, 4}};
                localparam int ITEMS [$:3] = '{7, 8};
                localparam int LOOKUP [int] = '{7:70, 3:30, default:99};
                typedef union { int count; logic [7:0] byte_value; } payload_t;
                function automatic payload_t make_payload();
                  payload_t value;
                  value.count = 42;
                  return value;
                endfunction
                localparam payload_t PAYLOAD = make_payload();
              endpackage",
        };
        let packages = Compilation::new(&SlangConfig {
            sources: &[source],
            ..Default::default()
        })
        .unwrap()
        .packages()
        .unwrap();
        let package = &packages["constants"];
        assert_eq!(
            package.parameters["MATRIX"],
            ConstantValue::Elements(vec![
                ConstantValue::Elements(vec![
                    integer_constant(32, true, 1u8),
                    integer_constant(32, true, 2u8)
                ]),
                ConstantValue::Elements(vec![
                    integer_constant(32, true, 3u8),
                    integer_constant(32, true, 4u8)
                ]),
            ])
        );
        assert_eq!(
            package.parameters["ITEMS"],
            ConstantValue::Queue {
                elements: vec![
                    integer_constant(32, true, 7u8),
                    integer_constant(32, true, 8u8)
                ],
                max_bound: 3,
            }
        );
        let ConstantValue::Map { entries, default } = &package.parameters["LOOKUP"] else {
            panic!("expected an associative array");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(**default, integer_constant(32, true, 99u8));
        for (key, expected) in [(3u8, 30u8), (7, 70)] {
            assert_eq!(
                entries
                    .iter()
                    .find(|(value, _)| *value == integer_constant(32, true, key))
                    .map(|(_, value)| value),
                Some(&integer_constant(32, true, expected))
            );
        }
        assert_eq!(
            package.parameters["PAYLOAD"],
            ConstantValue::Union {
                value: Box::new(integer_constant(32, true, 42u8)),
                active_member: Some(0),
            }
        );
    }
}
