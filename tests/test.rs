// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use slang_rs::*;

    #[test]
    fn test_extract_ports() {
        let verilog = "
        `define M 8
        module foo #(
            parameter N=11
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
            inout wire [0:N] l
        );
            bar bar_inst(.*);
        endmodule
        ";
        let definitions = extract_ports(verilog, true);
        println!("{:?}", definitions);
        assert_eq!(
            definitions["foo"],
            vec![
                IO::Input { msb: 0, lsb: 0 },
                IO::Output { msb: 1, lsb: 0 },
                IO::Output { msb: 2, lsb: 0 },
                IO::Input { msb: 3, lsb: 0 },
                IO::Output { msb: 4, lsb: 0 },
                IO::Output { msb: 5, lsb: 0 },
                IO::Output { msb: 6, lsb: 0 },
                IO::Input { msb: 7, lsb: 0 },
                IO::Output { msb: 8, lsb: 0 },
                IO::Input { msb: 9, lsb: 0 },
                IO::Output { msb: 10, lsb: 0 },
                IO::InOut { msb: 0, lsb: 11 }
            ]
        );
    }
}
