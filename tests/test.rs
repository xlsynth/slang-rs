// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use slang_rs::*;

    #[test]
    fn test_basic() {
        let verilog = "module foo; endmodule";
        let result = run_slang(verilog);
        for member in result.unwrap()["design"]["members"].as_array().unwrap() {
            if member["kind"] == "Instance" {
                assert_eq!(member["name"], "foo");
                return;
            }
        }
        panic!("Instance not found");
    }
}
