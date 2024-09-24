// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use slang_rs::*;
    use std::collections::HashMap;

    #[test]
    fn test_extract_ports() {
        let verilog = "
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
        endmodule
        ";
        let mut parameters = HashMap::new();
        parameters.insert("O".to_string(), "42".to_string());
        let definitions = extract_ports(verilog, true, &parameters);
        println!("{:?}", definitions);
        assert_eq!(
            definitions["foo"],
            vec![
                Port {
                    dir: PortDir::Input,
                    name: "a".to_string(),
                    msb: 0,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Output,
                    name: "b".to_string(),
                    msb: 1,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Output,
                    name: "c".to_string(),
                    msb: 2,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Input,
                    name: "d".to_string(),
                    msb: 3,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Output,
                    name: "e".to_string(),
                    msb: 4,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Output,
                    name: "f".to_string(),
                    msb: 5,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Output,
                    name: "g".to_string(),
                    msb: 6,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Input,
                    name: "h".to_string(),
                    msb: 7,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Output,
                    name: "i".to_string(),
                    msb: 8,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Input,
                    name: "j".to_string(),
                    msb: 9,
                    lsb: 0
                },
                Port {
                    dir: PortDir::Output,
                    name: "k".to_string(),
                    msb: 10,
                    lsb: 0
                },
                Port {
                    dir: PortDir::InOut,
                    name: "l".to_string(),
                    msb: 0,
                    lsb: 11
                },
                Port {
                    dir: PortDir::Output,
                    name: "m".to_string(),
                    msb: 41,
                    lsb: 0
                }
            ]
        );
    }
}
