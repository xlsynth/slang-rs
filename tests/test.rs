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
                        packed_dimensions: vec![],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "b".to_string(),
                    ty: Type::Logic {
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
                        packed_dimensions: vec![Range { msb: 2, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "d".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 3, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "e".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 4, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "f".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 5, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "g".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 6, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "h".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 7, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "i".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 8, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Input,
                    name: "j".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 9, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "k".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 10, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::InOut,
                    name: "l".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 0, lsb: 11 }],
                        unpacked_dimensions: vec![],
                    },
                },
                Port {
                    dir: PortDir::Output,
                    name: "m".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![Range { msb: 41, lsb: 0 }],
                        unpacked_dimensions: vec![],
                    },
                },
            ]
        );
    }

    #[test]
    fn test_ignore_union() {
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
        println!("{:?}", definitions);
        assert_eq!(
            definitions["foo"],
            vec![Port {
                dir: PortDir::Input,
                name: "clk".to_string(),
                ty: Type::Logic {
                    packed_dimensions: vec![],
                    unpacked_dimensions: vec![],
                },
            }]
        );
    }

    #[test]
    #[should_panic(expected = "expected allowed_type")]
    fn test_panic_on_unsupported() {
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

        let _definitions = extract_ports(&cfg, false);
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
        println!("{:?}", definitions);

        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Logic {
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
                                    packed_dimensions: vec![Range { msb: 7, lsb: 0 }],
                                    unpacked_dimensions: vec![],
                                },
                            },
                            Field {
                                name: "valid".to_string(),
                                ty: Type::Logic {
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
        println!("{:?}", definitions);

        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "clk".to_string(),
                    ty: Type::Logic {
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
}
