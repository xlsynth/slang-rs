// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
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
            output [1:0] b,
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
        println!("{:?}", definitions);
        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "a".to_string(),
                    dims: Dims {
                        packed: vec![],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "b".to_string(),
                    dims: Dims {
                        packed: vec![(1, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "c".to_string(),
                    dims: Dims {
                        packed: vec![(2, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Input,
                    name: "d".to_string(),
                    dims: Dims {
                        packed: vec![(3, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "e".to_string(),
                    dims: Dims {
                        packed: vec![(4, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "f".to_string(),
                    dims: Dims {
                        packed: vec![(5, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "g".to_string(),
                    dims: Dims {
                        packed: vec![(6, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Input,
                    name: "h".to_string(),
                    dims: Dims {
                        packed: vec![(7, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "i".to_string(),
                    dims: Dims {
                        packed: vec![(8, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Input,
                    name: "j".to_string(),
                    dims: Dims {
                        packed: vec![(9, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "k".to_string(),
                    dims: Dims {
                        packed: vec![(10, 0)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::InOut,
                    name: "l".to_string(),
                    dims: Dims {
                        packed: vec![(0, 11)],
                        unpacked: vec![]
                    }
                },
                Port {
                    dir: PortDir::Output,
                    name: "m".to_string(),
                    dims: Dims {
                        packed: vec![(41, 0)],
                        unpacked: vec![]
                    }
                }
            ]
        );
    }

    #[test]
    fn test_ignore_struct() {
        let verilog = str2tmpfile(
            "
        typedef struct {
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
                dims: Dims {
                    packed: vec![],
                    unpacked: vec![]
                }
            }]
        );
    }

    #[test]
    #[should_panic(expected = "Unsupported type: struct")]
    fn test_panic_on_unsupported() {
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

        extract_ports(&cfg, false);
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
