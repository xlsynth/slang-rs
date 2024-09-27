// SPDX-License-Identifier: Apache-2.0

use pest::Parser;
use pest_derive::Parser;
use std::error::Error;

#[derive(Parser)]
#[grammar = "extract/grammar.pest"]
struct DataTypeParser;

#[derive(Debug, PartialEq)]
pub enum Type {
    Logic {
        packed_dimensions: Vec<Range>,
    },
    Struct {
        name: String,
        fields: Vec<Field>,
        packed_dimensions: Vec<Range>,
    },
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, PartialEq)]
pub struct Range {
    pub msb: usize,
    pub lsb: usize,
}

pub fn parse_type_definition(input: &str) -> Result<Type, Box<dyn Error>> {
    let mut parse_tree = DataTypeParser::parse(Rule::top, input)?;
    let ty = parse_tree.next().unwrap().into_inner().next().unwrap();
    Ok(build_field_type(ty))
}

fn build_field_type(pair: pest::iterators::Pair<Rule>) -> Type {
    let inner_pair = pair.into_inner().next().unwrap();
    match inner_pair.as_rule() {
        Rule::logic_type => build_logic_type(inner_pair),
        Rule::struct_type => build_struct_type(inner_pair),
        _ => unreachable!(),
    }
}

fn build_logic_type(pair: pest::iterators::Pair<Rule>) -> Type {
    let inner = pair.into_inner();
    let mut packed_dimensions = Vec::new();

    for inner_pair in inner {
        if inner_pair.as_rule() == Rule::packed_dimensions {
            for dim_pair in inner_pair.into_inner() {
                let range = build_range(dim_pair.into_inner().next().unwrap());
                packed_dimensions.push(range);
            }
        }
    }

    Type::Logic { packed_dimensions }
}

fn build_struct_type(pair: pest::iterators::Pair<Rule>) -> Type {
    let inner = pair.into_inner();
    let mut fields = Vec::new();
    let mut packed_dimensions = Vec::new();
    let mut name = String::new();

    for inner_pair in inner {
        match inner_pair.as_rule() {
            Rule::field_list => {
                for field_pair in inner_pair.into_inner() {
                    if field_pair.as_rule() == Rule::field {
                        let field = build_field(field_pair);
                        fields.push(field);
                    }
                }
            }
            Rule::identifier => {
                name = inner_pair.as_str().to_string();
            }
            Rule::packed_dimensions => {
                for dim_pair in inner_pair.into_inner() {
                    let range = build_range(dim_pair.into_inner().next().unwrap());
                    packed_dimensions.push(range);
                }
            }
            _ => {}
        }
    }

    Type::Struct {
        name,
        fields,
        packed_dimensions,
    }
}

fn build_field(pair: pest::iterators::Pair<Rule>) -> Field {
    let mut inner = pair.into_inner();
    let field_type_pair = inner.next().unwrap();
    let ty = build_field_type(field_type_pair);
    let name = inner.next().unwrap().as_str();
    Field {
        name: String::from(name).clone(),
        ty,
    }
}

fn build_range(pair: pest::iterators::Pair<Rule>) -> Range {
    let mut inner = pair.into_inner();
    let msb = inner.next().unwrap().as_str().parse::<usize>().unwrap();
    let lsb = inner.next().unwrap().as_str().parse::<usize>().unwrap();
    Range { msb, lsb }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_logic() {
        let type_def = parse_type_definition("logic").unwrap();
        assert_eq!(
            type_def,
            Type::Logic {
                packed_dimensions: vec![]
            }
        );
    }

    #[test]
    fn test_logic_bus() {
        let type_def = parse_type_definition("logic[7:0]").unwrap();
        assert_eq!(
            type_def,
            Type::Logic {
                packed_dimensions: vec![Range { msb: 7, lsb: 0 }]
            }
        );
    }

    #[test]
    fn test_logic_bus_2d() {
        let type_def = parse_type_definition("logic[15:0][7:0]").unwrap();
        assert_eq!(
            type_def,
            Type::Logic {
                packed_dimensions: vec![Range { msb: 15, lsb: 0 }, Range { msb: 7, lsb: 0 }]
            }
        );
    }

    #[test]
    fn test_struct_packed_logic_a_bus_t() {
        let type_def = parse_type_definition("struct packed{logic a}bus_t").unwrap();
        assert_eq!(
            type_def,
            Type::Struct {
                name: "bus_t".to_string(),
                fields: vec![Field {
                    name: "a".to_string(),
                    ty: Type::Logic {
                        packed_dimensions: vec![]
                    }
                }],
                packed_dimensions: vec![]
            }
        );
    }

    #[test]
    fn test_struct_packed_with_logic_fields_and_packed_dimensions() {
        let type_def =
            parse_type_definition("struct packed{logic[7:0] a; logic b;}bus_t[3:0]").unwrap();
        assert_eq!(
            type_def,
            Type::Struct {
                name: "bus_t".to_string(),
                fields: vec![
                    Field {
                        name: "a".to_string(),
                        ty: Type::Logic {
                            packed_dimensions: vec![Range { msb: 7, lsb: 0 }]
                        }
                    },
                    Field {
                        name: "b".to_string(),
                        ty: Type::Logic {
                            packed_dimensions: vec![]
                        }
                    }
                ],
                packed_dimensions: vec![Range { msb: 3, lsb: 0 }]
            }
        );
    }

    #[test]
    fn test_nested_structs() {
        let type_def =
            parse_type_definition("struct packed{struct packed{logic a}inner_t inner}outer_t")
                .unwrap();
        assert_eq!(
            type_def,
            Type::Struct {
                name: "outer_t".to_string(),
                fields: vec![Field {
                    name: "inner".to_string(),
                    ty: Type::Struct {
                        name: "inner_t".to_string(),
                        fields: vec![Field {
                            name: "a".to_string(),
                            ty: Type::Logic {
                                packed_dimensions: vec![]
                            }
                        }],
                        packed_dimensions: vec![]
                    }
                }],
                packed_dimensions: vec![]
            }
        );
    }

    #[test]
    fn test_complex_nested_structs_with_packed_dimensions() {
        let type_def = parse_type_definition("struct packed{struct packed{logic a; logic[3:0][2:0] b;}inner_t inner; logic[13:0] c;}mega_t[7:0][15:0]").unwrap();
        assert_eq!(
            type_def,
            Type::Struct {
                name: "mega_t".to_string(),
                fields: vec![
                    Field {
                        name: "inner".to_string(),
                        ty: Type::Struct {
                            name: "inner_t".to_string(),
                            fields: vec![
                                Field {
                                    name: "a".to_string(),
                                    ty: Type::Logic {
                                        packed_dimensions: vec![]
                                    }
                                },
                                Field {
                                    name: "b".to_string(),
                                    ty: Type::Logic {
                                        packed_dimensions: vec![
                                            Range { msb: 3, lsb: 0 },
                                            Range { msb: 2, lsb: 0 }
                                        ]
                                    }
                                }
                            ],
                            packed_dimensions: vec![]
                        }
                    },
                    Field {
                        name: "c".to_string(),
                        ty: Type::Logic {
                            packed_dimensions: vec![Range { msb: 13, lsb: 0 }]
                        }
                    }
                ],
                packed_dimensions: vec![Range { msb: 7, lsb: 0 }, Range { msb: 15, lsb: 0 }]
            }
        );
    }
}
