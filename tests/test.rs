// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use num_bigint::BigInt;
    use slang_rs::*;

    #[test]
    fn test_extract_ports() {
        let verilog = str2tmpfile(
            "
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
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            parameters: &[("O", "42")],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);
        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "a".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "b".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![
                            Range { msb: 1, lsb: 0 },
                            Range { msb: 11, lsb: 0 }
                        ],
                        unpacked_dimensions: vec![
                            Range { msb: 0, lsb: 111 },
                            Range { msb: 1111, lsb: 0 }
                        ],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "c".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 2, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "d".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 3, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "e".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 4, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "f".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 5, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "g".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 6, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "h".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 7, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "i".to_string(),
                    ty: Type::Logic {
                        signed: true,
                        packed_dimensions: vec![Range { msb: 8, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "j".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 9, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "k".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 10, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::InOut,
                    name: "l".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 0, lsb: 11 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "m".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![Range { msb: 41, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
            ]
        );
    }

    #[test]
    fn test_union() {
        let verilog = str2tmpfile(
            "
        typedef union {
            logic [7:0] data;
            logic valid;
        } bus_t;

        module foo (
            input clk,
            input bus_t bus
        );
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, true);
        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "bus".to_string(),
                    ty: Type::Union {
                        name: "bus_t".to_string(),
                        fields: vec![
                            Field {
                                name: "data".to_string(),
                                ty: Type::Logic {
                                    signed: false,
                                    packed_dimensions: vec![Range { msb: 7, lsb: 0 }],
                                    unpacked_dimensions: vec![],
                                },
                            },
                            Field {
                                name: "valid".to_string(),
                                ty: Type::Logic {
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

        assert_eq!(definitions["foo"][1].ty.width().unwrap(), 8);
    }

    #[test]
    fn test_struct() {
        let verilog = str2tmpfile(
            "
        typedef struct {
            logic [7:0] data;
            logic valid;
        } bus_t;

        module foo (
            input clk,
            output bus_t bus
        );
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);
        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "bus".to_string(),
                    ty: Type::Struct {
                        name: "bus_t".to_string(),
                        fields: vec![
                            Field {
                                name: "data".to_string(),
                                ty: Type::Logic {
                                    signed: false,
                                    packed_dimensions: vec![Range { msb: 7, lsb: 0 }],
                                    unpacked_dimensions: vec![],
                                },
                            },
                            Field {
                                name: "valid".to_string(),
                                ty: Type::Logic {
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

        assert_eq!(definitions["foo"][1].ty.width().unwrap(), 9);
    }

    #[test]
    fn test_struct_array() {
        let verilog = str2tmpfile(
            "
        typedef struct packed {
            logic [7:0] data;
        } bus_t;

        module foo (
            output bus_t [3:0] bus
        );
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);

        assert_eq!(
            definitions["foo"],
            vec![Port {
                dir: PortDir::Output,
                name: "bus".to_string(),
                ty: Type::Struct {
                    name: "bus_t".to_string(),
                    fields: vec![Field {
                        name: "data".to_string(),
                        ty: Type::Logic {
                            signed: false,
                            packed_dimensions: vec![Range { msb: 7, lsb: 0 }],
                            unpacked_dimensions: vec![],
                        },
                    },],
                    packed_dimensions: vec![Range { msb: 3, lsb: 0 }],
                    unpacked_dimensions: vec![],
                },
            },]
        );
    }

    #[test]
    fn test_enum_array() {
        let verilog = str2tmpfile(
            "
        typedef enum logic [1:0] {
            RED=0,
            GREEN=1,
            BLUE=2
        } color_t;

        module foo (
            output color_t [3:0] color
        );
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);

        assert_eq!(
            definitions["foo"],
            vec![Port {
                dir: PortDir::Output,
                name: "color".to_string(),
                ty: Type::Enum {
                    name: "color_t".to_string(),
                    variants: vec![
                        Variant {
                            name: "RED".to_string(),
                            width: 2,
                            value: BigInt::from(0),
                        },
                        Variant {
                            name: "GREEN".to_string(),
                            width: 2,
                            value: BigInt::from(1),
                        },
                        Variant {
                            name: "BLUE".to_string(),
                            width: 2,
                            value: BigInt::from(2),
                        },
                    ],
                    packed_dimensions: vec![Range { msb: 3, lsb: 0 }],
                    unpacked_dimensions: vec![],
                },
            },]
        );
    }

    #[test]
    fn test_package() {
        let verilog = str2tmpfile(
            "
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
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);
        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "bus".to_string(),
                    ty: Type::Struct {
                        name: "mypack::bus_t".to_string(),
                        fields: vec![
                            Field {
                                name: "data".to_string(),
                                ty: Type::Logic {
                                    signed: false,
                                    packed_dimensions: vec![Range { msb: 7, lsb: 0 }],
                                    unpacked_dimensions: vec![],
                                },
                            },
                            Field {
                                name: "valid".to_string(),
                                ty: Type::Logic {
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
                        name: "mypack::enum_t".to_string(),
                        variants: vec![
                            Variant {
                                name: "A".to_string(),
                                width: 16,
                                value: BigInt::from(1234),
                            },
                            Variant {
                                name: "B".to_string(),
                                width: 16,
                                value: BigInt::from(2345),
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
        let verilog = str2tmpfile(
            "
        typedef enum logic [15:0] {
            A=1234,
            B=2345
        } enum_t;

        module foo (
            input clk,
            output enum_t data
        );
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);
        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Logic {
                        signed: false,
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "data".to_string(),
                    ty: Type::Enum {
                        name: "enum_t".to_string(),
                        variants: vec![
                            Variant {
                                name: "A".to_string(),
                                width: 16,
                                value: BigInt::from(1234),
                            },
                            Variant {
                                name: "B".to_string(),
                                width: 16,
                                value: BigInt::from(2345),
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
    #[should_panic(expected = "expected 'endmodule'")]
    fn test_informative_parse_error() {
        let verilog = str2tmpfile("module A;").unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        extract_ports(&cfg, false);
    }

    #[test]
    fn test_width_fn() {
        let verilog = str2tmpfile(
            "
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
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);

        assert_eq!(definitions["foo"][0].ty.width().unwrap(), 422);
        assert_eq!(definitions["foo"][1].ty.width().unwrap(), 23632);
        assert_eq!(definitions["foo"][2].ty.width().unwrap(), 1);
    }

    #[test]
    fn test_module_extract() {
        // test verilog includes other kinds of definitions to make sure that the
        // library is only extracting module names
        let verilog = str2tmpfile(
            "
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
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let mut modules = extract_modules(&cfg).unwrap();
        modules.sort();

        assert_eq!(modules, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_timescale_option() {
        let verilog_a = str2tmpfile(
            "
module A(
    input clk
);
    B b();
endmodule
",
        )
        .unwrap();

        let verilog_b = str2tmpfile(
            "
`timescale 1ns/1ps
module B;
endmodule
",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[
                verilog_b.path().to_str().unwrap(),
                verilog_a.path().to_str().unwrap(),
            ],
            tops: &["A"],
            timescale: Some("1ns/1ps"),
            ..Default::default()
        };

        assert_eq!(
            extract_ports(&cfg, false)["A"],
            vec![Port {
                dir: PortDir::Input,
                name: "clk".to_string(),
                ty: Type::Logic {
                    signed: false,
                    packed_dimensions: vec![],
                    unpacked_dimensions: vec![]
                },
            }]
        );
    }

    #[test]
    fn test_protected() {
        let verilog = str2tmpfile(
            "
        module foo(
            input a
        );
            `protected
            asdf
            `endprotected
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);
        assert_eq!(
            definitions["foo"],
            vec![Port {
                dir: PortDir::Input,
                name: "a".to_string(),
                ty: Type::Logic {
                    signed: false,
                    packed_dimensions: vec![],
                    unpacked_dimensions: vec![],
                },
            }]
        );
    }

    #[test]
    #[should_panic(expected = "unknown macro")]
    fn test_protected_panic() {
        let verilog = str2tmpfile(
            "
        module foo(
            input a
        );
            `protected
            asdf
            `endprotected
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            ignore_protected: false,
            ..Default::default()
        };

        extract_ports(&cfg, false);
    }

    #[test]
    fn test_negative_indices() {
        let verilog = str2tmpfile(
            "
        module foo #(
            parameter N=1
        ) (
            input [N-1:0] a
        );
        endmodule",
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            parameters: &[("N", "0")],
            ..Default::default()
        };

        let definitions = extract_ports(&cfg, false);

        assert_eq!(
            definitions["foo"],
            vec![Port {
                dir: PortDir::Input,
                name: "a".to_string(),
                ty: Type::Logic {
                    signed: false,
                    packed_dimensions: vec![Range { msb: -1, lsb: 0 }],
                    unpacked_dimensions: vec![],
                },
            },]
        );

        assert_eq!(definitions["foo"][0].ty.width().unwrap(), 2);
    }

    #[test]
    fn test_enum_conversion() {
        let verilog = str2tmpfile(
            "
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
        )
        .unwrap();

        let cfg = SlangConfig {
            sources: &[verilog.path().to_str().unwrap()],
            extra_arguments: &["--relax-enum-conversions"],
            ..Default::default()
        };

        let ports = extract_ports(&cfg, false);

        assert_eq!(ports["foo"].len(), 2);
        assert_eq!(ports["foo"][0].name, "a");
        assert_eq!(ports["foo"][0].ty.width().unwrap(), 2);
        assert_eq!(ports["foo"][1].name, "b");
        assert_eq!(ports["foo"][1].ty.width().unwrap(), 2);
    }
}
