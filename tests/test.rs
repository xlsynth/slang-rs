// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use num_bigint::BigUint;
    use slang_rs::*;

    #[test]
    fn test_extract_ports() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        `define M 8
        module foo #(
            parameter N=11,
            parameter O=12
        ) (
            input a,
            output [1:0][11:0] b [0:111][1111:0],
            output wire [2:0] c,
            input wire logic [3:0] d,
            output reg [4:0] e,
            output var logic [5:0] f,
            output [6:0] g,
            input [(`M)-1:0] h,
            output signed [8:0] i,
            input unsigned [9:0] j,
            output bit [10:0] k,
            inout wire [0:N] l,
            output wire [O-1:0] m
        );
            bar bar_inst(.*);
        endmodule
        module baz #(
            parameter P=13
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            parameters: &[("O", "42")],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();
        assert_eq!(
            definitions["foo"].as_slice(),
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "a".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(1),
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "b".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: None,
                        signed: false,
                        packed_dimensions: vec![
                            Range { left: 1, right: 0 },
                            Range { left: 11, right: 0 }
                        ],
                        unpacked_dimensions: vec![
                            Range {
                                left: 0,
                                right: 111
                            },
                            Range {
                                left: 1111,
                                right: 0
                            }
                        ],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "c".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(3),
                        signed: false,
                        packed_dimensions: vec![Range { left: 2, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "d".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(4),
                        signed: false,
                        packed_dimensions: vec![Range { left: 3, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "e".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Reg,
                        four_state: true,
                        bit_width: Some(5),
                        signed: false,
                        packed_dimensions: vec![Range { left: 4, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "f".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(6),
                        signed: false,
                        packed_dimensions: vec![Range { left: 5, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "g".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(7),
                        signed: false,
                        packed_dimensions: vec![Range { left: 6, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "h".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(8),
                        signed: false,
                        packed_dimensions: vec![Range { left: 7, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "i".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(9),
                        signed: true,
                        packed_dimensions: vec![Range { left: 8, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "j".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(10),
                        signed: false,
                        packed_dimensions: vec![Range { left: 9, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "k".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Bit,
                        four_state: false,
                        bit_width: Some(11),
                        signed: false,
                        packed_dimensions: vec![Range { left: 10, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::InOut,
                    name: "l".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(12),
                        signed: false,
                        packed_dimensions: vec![Range { left: 0, right: 11 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "m".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(42),
                        signed: false,
                        packed_dimensions: vec![Range { left: 41, right: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
            ]
        );
    }

    #[test]
    fn test_union() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        typedef union {
            logic [7:0] data;
            logic valid;
        } bus_t;

        module foo (
            input clk,
            input bus_t bus
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            skip_unsupported_ports: true,
            ..Default::default()
        };
        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();
        assert_eq!(
            definitions["foo"].as_slice(),
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(1),
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "bus".to_string(),
                    ty: Type::Union {
                        signed: false,
                        four_state: true,
                        bit_width: None,
                        is_packed: false,
                        name: Some("bus_t".to_string()),
                        fields: vec![
                            Field {
                                name: "data".to_string(),
                                ty: Type::Integral {
                                    kind: IntegralKind::Logic,
                                    four_state: true,
                                    bit_width: Some(8),
                                    signed: false,
                                    packed_dimensions: vec![Range { left: 7, right: 0 }],
                                    unpacked_dimensions: vec![],
                                },
                            },
                            Field {
                                name: "valid".to_string(),
                                ty: Type::Integral {
                                    kind: IntegralKind::Logic,
                                    four_state: true,
                                    bit_width: Some(1),
                                    signed: false,
                                    packed_dimensions: vec![],
                                    unpacked_dimensions: vec![],
                                },
                            },
                        ],
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                }
            ]
        );

        assert!(definitions["foo"][1].ty.width().is_err());
    }

    #[test]
    fn test_struct() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        typedef struct {
            logic [7:0] data;
            logic valid;
        } bus_t;

        module foo (
            input clk,
            output bus_t bus
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();
        assert_eq!(
            definitions["foo"].as_slice(),
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(1),
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "bus".to_string(),
                    ty: Type::Struct {
                        signed: false,
                        four_state: true,
                        bit_width: None,
                        is_packed: false,
                        name: Some("bus_t".to_string()),
                        fields: vec![
                            Field {
                                name: "data".to_string(),
                                ty: Type::Integral {
                                    kind: IntegralKind::Logic,
                                    four_state: true,
                                    bit_width: Some(8),
                                    signed: false,
                                    packed_dimensions: vec![Range { left: 7, right: 0 }],
                                    unpacked_dimensions: vec![],
                                },
                            },
                            Field {
                                name: "valid".to_string(),
                                ty: Type::Integral {
                                    kind: IntegralKind::Logic,
                                    four_state: true,
                                    bit_width: Some(1),
                                    signed: false,
                                    packed_dimensions: vec![],
                                    unpacked_dimensions: vec![],
                                },
                            },
                        ],
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
            ]
        );

        assert!(definitions["foo"][1].ty.width().is_err());
    }

    #[test]
    fn test_struct_array() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        typedef struct packed {
            logic [7:0] data;
        } bus_t;

        module foo (
            output bus_t [3:0] bus
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();

        assert_eq!(
            definitions["foo"].as_slice(),
            vec![Port {
                dir: PortDir::Output,
                name: "bus".to_string(),
                ty: Type::Struct {
                    signed: false,
                    four_state: true,
                    bit_width: Some(32),
                    is_packed: true,
                    name: Some("bus_t".to_string()),
                    fields: vec![Field {
                        name: "data".to_string(),
                        ty: Type::Integral {
                            kind: IntegralKind::Logic,
                            four_state: true,
                            bit_width: Some(8),
                            signed: false,
                            packed_dimensions: vec![Range { left: 7, right: 0 }],
                            unpacked_dimensions: vec![],
                        },
                    },],
                    packed_dimensions: vec![Range { left: 3, right: 0 }],
                    unpacked_dimensions: vec![],
                },
            },]
        );
    }

    #[test]
    fn test_enum_array() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        typedef enum logic [1:0] {
            RED=0,
            GREEN=1,
            BLUE=2
        } color_t;

        module foo (
            output color_t [3:0] color
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();

        assert_eq!(
            definitions["foo"].as_slice(),
            vec![Port {
                dir: PortDir::Output,
                name: "color".to_string(),
                ty: Type::Enum {
                    signed: false,
                    four_state: true,
                    base_type: Box::new(Type::Integral {
                        kind: IntegralKind::Logic,
                        signed: false,
                        four_state: true,
                        packed_dimensions: vec![Range { left: 1, right: 0 }],
                        unpacked_dimensions: vec![],
                        bit_width: Some(2),
                    }),
                    bit_width: Some(8),
                    name: Some("color_t".to_string()),
                    variants: vec![
                        Variant {
                            name: "RED".to_string(),
                            value: IntegerValue {
                                width: 2,
                                signed: false,
                                bits: BigUint::from(0u32),
                                x_mask: BigUint::default(),
                                z_mask: BigUint::default(),
                            },
                        },
                        Variant {
                            name: "GREEN".to_string(),
                            value: IntegerValue {
                                width: 2,
                                signed: false,
                                bits: BigUint::from(1u32),
                                x_mask: BigUint::default(),
                                z_mask: BigUint::default(),
                            },
                        },
                        Variant {
                            name: "BLUE".to_string(),
                            value: IntegerValue {
                                width: 2,
                                signed: false,
                                bits: BigUint::from(2u32),
                                x_mask: BigUint::default(),
                                z_mask: BigUint::default(),
                            },
                        },
                    ],
                    packed_dimensions: vec![Range { left: 3, right: 0 }],
                    unpacked_dimensions: vec![],
                },
            },]
        );
    }

    #[test]
    fn test_package() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        package mypack;
            typedef struct {
                logic [7:0] data;
                logic valid;
            } bus_t;
            typedef enum logic [15:0] {
                A=1234,
                B=2345
            } enum_t;
        endpackage

        module foo (
            input clk,
            output mypack::bus_t bus,
            output mypack::enum_t data
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();
        assert_eq!(
            definitions["foo"].as_slice(),
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(1),
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "bus".to_string(),
                    ty: Type::Struct {
                        signed: false,
                        four_state: true,
                        bit_width: None,
                        is_packed: false,
                        name: Some("mypack::bus_t".to_string()),
                        fields: vec![
                            Field {
                                name: "data".to_string(),
                                ty: Type::Integral {
                                    kind: IntegralKind::Logic,
                                    four_state: true,
                                    bit_width: Some(8),
                                    signed: false,
                                    packed_dimensions: vec![Range { left: 7, right: 0 }],
                                    unpacked_dimensions: vec![],
                                },
                            },
                            Field {
                                name: "valid".to_string(),
                                ty: Type::Integral {
                                    kind: IntegralKind::Logic,
                                    four_state: true,
                                    bit_width: Some(1),
                                    signed: false,
                                    packed_dimensions: vec![],
                                    unpacked_dimensions: vec![],
                                },
                            },
                        ],
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "data".to_string(),
                    ty: Type::Enum {
                        signed: false,
                        four_state: true,
                        base_type: Box::new(Type::Integral {
                            kind: IntegralKind::Logic,
                            signed: false,
                            four_state: true,
                            packed_dimensions: vec![Range { left: 15, right: 0 }],
                            unpacked_dimensions: vec![],
                            bit_width: Some(16),
                        }),
                        bit_width: Some(16),
                        name: Some("mypack::enum_t".to_string()),
                        variants: vec![
                            Variant {
                                name: "A".to_string(),
                                value: IntegerValue {
                                    width: 16,
                                    signed: false,
                                    bits: BigUint::from(1234u32),
                                    x_mask: BigUint::default(),
                                    z_mask: BigUint::default(),
                                },
                            },
                            Variant {
                                name: "B".to_string(),
                                value: IntegerValue {
                                    width: 16,
                                    signed: false,
                                    bits: BigUint::from(2345u32),
                                    x_mask: BigUint::default(),
                                    z_mask: BigUint::default(),
                                },
                            },
                        ],
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                }
            ]
        );
    }

    #[test]
    fn test_enum() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        typedef enum logic [15:0] {
            A=1234,
            B=2345
        } enum_t;

        module foo (
            input clk,
            output enum_t data
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();
        assert_eq!(
            definitions["foo"].as_slice(),
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Integral {
                        kind: IntegralKind::Logic,
                        four_state: true,
                        bit_width: Some(1),
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "data".to_string(),
                    ty: Type::Enum {
                        signed: false,
                        four_state: true,
                        base_type: Box::new(Type::Integral {
                            kind: IntegralKind::Logic,
                            signed: false,
                            four_state: true,
                            packed_dimensions: vec![Range { left: 15, right: 0 }],
                            unpacked_dimensions: vec![],
                            bit_width: Some(16),
                        }),
                        bit_width: Some(16),
                        name: Some("enum_t".to_string()),
                        variants: vec![
                            Variant {
                                name: "A".to_string(),
                                value: IntegerValue {
                                    width: 16,
                                    signed: false,
                                    bits: BigUint::from(1234u32),
                                    x_mask: BigUint::default(),
                                    z_mask: BigUint::default(),
                                },
                            },
                            Variant {
                                name: "B".to_string(),
                                value: IntegerValue {
                                    width: 16,
                                    signed: false,
                                    bits: BigUint::from(2345u32),
                                    x_mask: BigUint::default(),
                                    z_mask: BigUint::default(),
                                },
                            },
                        ],
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
            ]
        );
    }

    #[test]
    fn test_informative_parse_error() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "module A;",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let error = Compilation::new(&cfg).unwrap_err();
        assert!(error.to_string().contains("expected 'endmodule'"));
        assert!(!error.diagnostics.is_empty());
    }

    #[test]
    fn test_width_fn() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        typedef struct packed {
            logic [7:0] a; // width: 8
            logic [1:0][2:0] b; // width: 6
        } inner_t; // width: 14

        typedef enum logic [1:0] {
            RED=0,
            GREEN=1,
            BLUE=2
        } color_t; // width: 2

        typedef struct packed {
            inner_t c; // width: 14
            inner_t [3:0] d; // width: 56
            inner_t [4:0][4:0] e; // width: 350
            color_t color; // width: 2
        } outer_t; // width: 422

        module foo (
            output outer_t out0, // width: 422
            output outer_t [6:0][7:0] out1, // width: 23632
            input wire in0
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();

        assert_eq!(definitions["foo"][0].ty.width().unwrap(), 422);
        assert_eq!(definitions["foo"][1].ty.width().unwrap(), 23632);
        assert_eq!(definitions["foo"][2].ty.width().unwrap(), 1);
    }

    #[test]
    fn test_module_extract() {
        // test verilog includes other kinds of definitions to make sure that the
        // library is only extracting module names
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
package my_pack;
endpackage

typedef struct {
    logic [7:0] data;
} my_struct_t;

interface my_intf;
endinterface

module A;
endmodule

module B;
endmodule

module C;
A a0();
A a1();
B b0();
B b1();
endmodule

module D;
endmodule
",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let mut modules = Compilation::new(&cfg).unwrap().modules().unwrap();
        modules.sort();

        assert_eq!(modules, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_timescale_option() {
        let verilog_a = Source::Text {
            name: "verilog_a.sv",
            text: "
module A(
    input clk
);
    B b();
endmodule
",
        };

        let verilog_b = Source::Text {
            name: "verilog_b.sv",
            text: "
`timescale 1ns/1ps
module B;
endmodule
",
        };

        let cfg = SlangConfig {
            sources: &[verilog_b, verilog_a],
            tops: &["A"],
            timescale: Some(TimeScale {
                base: TimeScaleValue {
                    unit: TimeUnit::Nanoseconds,
                    magnitude: TimeScaleMagnitude::One,
                },
                precision: TimeScaleValue {
                    unit: TimeUnit::Picoseconds,
                    magnitude: TimeScaleMagnitude::One,
                },
            }),
            ..Default::default()
        };

        assert_eq!(
            Compilation::new(&cfg).unwrap().ports().unwrap()["A"].as_slice(),
            vec![Port {
                dir: PortDir::Input,
                name: "clk".to_string(),
                ty: Type::Integral {
                    kind: IntegralKind::Logic,
                    four_state: true,
                    bit_width: Some(1),
                    signed: false,
                    packed_dimensions: vec![],
                    unpacked_dimensions: vec![]
                },
            }]
        );
    }

    #[test]
    fn test_protected() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        module foo(
            input a
        );
            `protected
            asdf
            `endprotected
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();
        assert_eq!(
            definitions["foo"].as_slice(),
            vec![Port {
                dir: PortDir::Input,
                name: "a".to_string(),
                ty: Type::Integral {
                    kind: IntegralKind::Logic,
                    four_state: true,
                    bit_width: Some(1),
                    signed: false,
                    packed_dimensions: vec![],
                    unpacked_dimensions: vec![],
                },
            }]
        );
    }

    #[test]
    fn test_disabled_legacy_protect_returns_error() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        module foo(
            input a
        );
            `protected
            asdf
            `endprotected
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            enable_legacy_protect: false,
            ..Default::default()
        };

        let error = Compilation::new(&cfg).unwrap_err();
        assert!(error.to_string().contains("unknown macro"));
        assert!(!error.diagnostics.is_empty());
    }

    #[test]
    fn test_negative_indices() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        module foo #(
            parameter N=1
        ) (
            input [N-1:0] a
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            parameters: &[("N", "0")],
            ..Default::default()
        };

        let definitions = Compilation::new(&cfg).unwrap().ports().unwrap();

        assert_eq!(
            definitions["foo"].as_slice(),
            vec![Port {
                dir: PortDir::Input,
                name: "a".to_string(),
                ty: Type::Integral {
                    kind: IntegralKind::Logic,
                    four_state: true,
                    bit_width: Some(2),
                    signed: false,
                    packed_dimensions: vec![Range { left: -1, right: 0 }],
                    unpacked_dimensions: vec![],
                },
            },]
        );

        assert_eq!(definitions["foo"][0].ty.width().unwrap(), 2);
    }

    #[test]
    fn test_enum_conversion() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        typedef enum logic [1:0] {
            A=0,
            B=1,
            C=2
        } enum_t;

        module foo (
            output enum_t a,
            input logic [1:0] b
        );
            assign a = b;
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            relax_enum_conversions: true,
            ..Default::default()
        };

        let ports = Compilation::new(&cfg).unwrap().ports().unwrap();

        assert_eq!(ports["foo"].len(), 2);
        assert_eq!(ports["foo"][0].name, "a");
        assert_eq!(ports["foo"][0].ty.width().unwrap(), 2);
        assert_eq!(ports["foo"][1].name, "b");
        assert_eq!(ports["foo"][1].ty.width().unwrap(), 2);
    }

    #[test]
    fn test_extract_parameters() {
        let verilog = Source::Text {
            name: "verilog.sv",
            text: "
        module foo #(
            parameter int IntParam = 1,
            parameter int unsigned UnsignedParam = 2,
            parameter longint LongIntParam = 3,
            parameter bit BitParam = 1
        ) (
            input logic [IntParam-1:0] a,
            output logic [UnsignedParam-1:0] b,
            input logic [LongIntParam-1:0] c
        );
        endmodule",
        };

        let cfg = SlangConfig {
            sources: &[verilog],
            ..Default::default()
        };

        let parameters = Compilation::new(&cfg).unwrap().parameters().unwrap();
        assert_eq!(parameters["foo"].len(), 4);
        assert_eq!(parameters["foo"][0].name, "IntParam");
        assert_eq!(
            parameters["foo"][0].ty,
            Type::Integral {
                kind: IntegralKind::Int,
                four_state: false,
                bit_width: Some(32),
                signed: true,
                packed_dimensions: vec![],
                unpacked_dimensions: vec![],
            }
        );
        assert_eq!(parameters["foo"][1].name, "UnsignedParam");
        assert_eq!(
            parameters["foo"][1].ty,
            Type::Integral {
                kind: IntegralKind::Int,
                four_state: false,
                bit_width: Some(32),
                signed: false,
                packed_dimensions: vec![],
                unpacked_dimensions: vec![],
            }
        );
        assert_eq!(parameters["foo"][2].name, "LongIntParam");
        assert_eq!(
            parameters["foo"][2].ty,
            Type::Integral {
                kind: IntegralKind::LongInt,
                four_state: false,
                bit_width: Some(64),
                signed: true,
                packed_dimensions: vec![],
                unpacked_dimensions: vec![],
            }
        );
        assert_eq!(parameters["foo"][3].name, "BitParam");
        assert_eq!(
            parameters["foo"][3].ty,
            Type::Integral {
                kind: IntegralKind::Bit,
                four_state: false,
                bit_width: Some(1),
                signed: false,
                packed_dimensions: vec![],
                unpacked_dimensions: vec![],
            }
        );
    }

    #[test]
    fn semantic_widths_distinguish_packed_and_unpacked_types() {
        let source = Source::Text {
            name: "source.sv",
            text: "typedef struct packed { logic [7:0] data; logic valid; } packed_s;
             typedef struct { logic [7:0] data; logic valid; } unpacked_s;
             typedef union packed { logic [7:0] data; logic [1:0][3:0] lanes; } packed_u;
             typedef union { logic [7:0] data; logic valid; } unpacked_u;
             module widths(
               input packed_s structure,
               input unpacked_s unpacked_structure,
               input packed_u overlay,
               input unpacked_u unpacked_overlay,
               input packed_s [1:0] packed_array,
               input unpacked_s unpacked_array [2],
               input logic [3:0] matrix [-2:1][2:0],
               input struct packed { logic [4:0] field; } anonymous_structure,
               input enum logic [1:0] { Idle, Ready } anonymous_enum
             ); endmodule",
        };
        let ports = Compilation::new(&SlangConfig {
            sources: &[source],
            ..Default::default()
        })
        .unwrap()
        .ports()
        .unwrap();
        let ports = &ports["widths"];
        let expected = [
            Some(9),
            None,
            Some(8),
            None,
            Some(18),
            None,
            None,
            Some(5),
            Some(2),
        ];
        assert_eq!(ports.len(), expected.len());
        for (port, numeric_width) in ports.iter().zip(expected) {
            assert_eq!(port.ty.width().ok(), numeric_width, "{}", port.name);
        }
        assert!(matches!(
            ports[0].ty,
            Type::Struct {
                is_packed: true,
                ..
            }
        ));
        assert!(matches!(
            ports[1].ty,
            Type::Struct {
                is_packed: false,
                ..
            }
        ));
        assert!(matches!(
            ports[2].ty,
            Type::Union {
                is_packed: true,
                ..
            }
        ));
        assert!(matches!(
            ports[3].ty,
            Type::Union {
                is_packed: false,
                ..
            }
        ));
        assert!(matches!(ports[7].ty, Type::Struct { name: None, .. }));
        assert!(matches!(ports[8].ty, Type::Enum { name: None, .. }));
    }
}
