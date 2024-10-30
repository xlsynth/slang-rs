// SPDX-License-Identifier: Apache-2.0

use num_bigint::{BigInt, BigUint, Sign};
use num_traits::Zero;

use pest::Parser;
use pest_derive::Parser;
use std::error::Error;

#[derive(Parser)]
#[grammar = "extract/grammar.pest"]
struct DataTypeParser;

#[derive(Debug, PartialEq)]
pub enum Type {
    Logic {
        signed: bool,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
    },
    Struct {
        name: String,
        fields: Vec<Field>,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
    },
    Union {
        name: String,
        fields: Vec<Field>,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
    },
    Enum {
        name: String,
        variants: Vec<Variant>,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
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

#[derive(Debug, PartialEq)]
pub struct Variant {
    pub name: String,
    pub width: usize,
    pub value: BigInt,
}

impl Type {
    fn unpacked_dimensions(&self) -> &Vec<Range> {
        match self {
            Type::Logic {
                unpacked_dimensions,
                ..
            } => unpacked_dimensions,
            Type::Struct {
                unpacked_dimensions,
                ..
            } => unpacked_dimensions,
            Type::Union {
                unpacked_dimensions,
                ..
            } => unpacked_dimensions,
            Type::Enum {
                unpacked_dimensions,
                ..
            } => unpacked_dimensions,
        }
    }

    fn packed_dimensions(&self) -> &Vec<Range> {
        match self {
            Type::Logic {
                packed_dimensions, ..
            } => packed_dimensions,
            Type::Struct {
                packed_dimensions, ..
            } => packed_dimensions,
            Type::Enum {
                packed_dimensions, ..
            } => packed_dimensions,
            Type::Union {
                packed_dimensions, ..
            } => packed_dimensions,
        }
    }

    fn number_of_elements(&self) -> Result<usize, &str> {
        if !self.unpacked_dimensions().is_empty() {
            return Err("Unpacked dimensions are not supported in width calculations");
        }

        Ok(self
            .packed_dimensions()
            .iter()
            .map(|Range { msb, lsb }| msb - lsb + 1)
            .product())
    }

    pub fn width(&self) -> Result<usize, &str> {
        match self {
            Type::Logic { signed: _, .. } => self.number_of_elements(),
            Type::Struct { fields, .. } => {
                let mut width = 0;
                for field in fields {
                    width += field.ty.width()?;
                }
                Ok(width * self.number_of_elements()?)
            }
            Type::Union { fields, .. } => {
                let mut width = 0;
                for field in fields {
                    width = width.max(field.ty.width()?);
                }
                Ok(width * self.number_of_elements()?)
            }
            Type::Enum { variants, .. } => {
                if variants.is_empty() {
                    return Err("Enum with no variants");
                }

                let width = variants[0].width;
                for variant in variants[1..].iter() {
                    if width != variant.width {
                        return Err("Enum with different-sized variants");
                    }
                }
                Ok(width * self.number_of_elements()?)
            }
        }
    }
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
        Rule::struct_type => build_struct_or_union_type(inner_pair, false),
        Rule::union_type => build_struct_or_union_type(inner_pair, true),
        Rule::enum_type => build_enum_type(inner_pair),
        _ => unreachable!(),
    }
}

fn build_logic_type(pair: pest::iterators::Pair<Rule>) -> Type {
    let inner = pair.into_inner();

    let mut signed = false;
    let mut packed_dimensions = Vec::new();
    let mut unpacked_dimensions = Vec::new();

    for inner_pair in inner {
        match inner_pair.as_rule() {
            Rule::dimensions => {
                let inner_inner = inner_pair.into_inner();
                for inner_inner_pair in inner_inner {
                    match inner_inner_pair.as_rule() {
                        Rule::packed_dimensions => {
                            for dim_pair in inner_inner_pair.into_inner() {
                                let range = build_range(dim_pair.into_inner().next().unwrap());
                                packed_dimensions.push(range);
                            }
                        }
                        Rule::unpacked_dimensions => {
                            for dim_pair in inner_inner_pair.into_inner() {
                                let range = build_range(dim_pair.into_inner().next().unwrap());
                                unpacked_dimensions.push(range);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Rule::signed_modifier => {
                signed = true;
            }
            _ => {}
        }
    }

    Type::Logic {
        signed,
        packed_dimensions,
        unpacked_dimensions,
    }
}

fn build_struct_or_union_type(pair: pest::iterators::Pair<Rule>, is_union: bool) -> Type {
    let inner = pair.into_inner();
    let mut fields = Vec::new();
    let mut packed_dimensions = Vec::new();
    let mut unpacked_dimensions = Vec::new();
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
            Rule::full_identifier => {
                name = inner_pair.as_str().to_string();
            }
            Rule::dimensions => {
                let inner_inner = inner_pair.into_inner();
                for inner_inner_pair in inner_inner {
                    match inner_inner_pair.as_rule() {
                        Rule::packed_dimensions => {
                            for dim_pair in inner_inner_pair.into_inner() {
                                let range = build_range(dim_pair.into_inner().next().unwrap());
                                packed_dimensions.push(range);
                            }
                        }
                        Rule::unpacked_dimensions => {
                            for dim_pair in inner_inner_pair.into_inner() {
                                let range = build_range(dim_pair.into_inner().next().unwrap());
                                unpacked_dimensions.push(range);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if is_union {
        Type::Union {
            name,
            fields,
            packed_dimensions,
            unpacked_dimensions,
        }
    } else {
        Type::Struct {
            name,
            fields,
            packed_dimensions,
            unpacked_dimensions,
        }
    }
}

fn build_enum_type(pair: pest::iterators::Pair<Rule>) -> Type {
    let inner = pair.into_inner();

    let mut variants = Vec::new();
    let mut packed_dimensions = Vec::new();
    let mut unpacked_dimensions = Vec::new();
    let mut name = String::new();

    for inner_pair in inner {
        match inner_pair.as_rule() {
            Rule::variant_list => {
                for variant_pair in inner_pair.into_inner() {
                    if variant_pair.as_rule() == Rule::variant {
                        let variant = build_variant(variant_pair);
                        variants.push(variant);
                    }
                }
            }
            Rule::full_identifier => {
                name = inner_pair.as_str().to_string();
            }
            Rule::dimensions => {
                let inner_inner = inner_pair.into_inner();
                for inner_inner_pair in inner_inner {
                    match inner_inner_pair.as_rule() {
                        Rule::packed_dimensions => {
                            for dim_pair in inner_inner_pair.into_inner() {
                                let range = build_range(dim_pair.into_inner().next().unwrap());
                                packed_dimensions.push(range);
                            }
                        }
                        Rule::unpacked_dimensions => {
                            for dim_pair in inner_inner_pair.into_inner() {
                                let range = build_range(dim_pair.into_inner().next().unwrap());
                                unpacked_dimensions.push(range);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Type::Enum {
        name,
        variants,
        packed_dimensions,
        unpacked_dimensions,
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

fn build_variant(pair: pest::iterators::Pair<Rule>) -> Variant {
    let mut inner = pair.into_inner();

    let name = inner.next().unwrap().as_str().to_string();

    let mut inner_inner = inner.next().unwrap().into_inner();

    let mut width = inner_inner.next().unwrap();
    let mut negative = false;
    if width.as_rule() == Rule::negative_sign {
        negative = true;
        width = inner_inner.next().unwrap();
    }

    let width = width.as_str().parse::<usize>().unwrap();

    let magnitude_str = inner_inner.next().unwrap().as_str();
    let magnitude = BigUint::parse_bytes(magnitude_str.as_bytes(), 10).unwrap();

    let sign = if magnitude.is_zero() {
        Sign::NoSign
    } else if negative {
        Sign::Minus
    } else {
        Sign::Plus
    };

    Variant {
        name,
        width,
        value: BigInt::from_biguint(sign, magnitude),
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
    fn test_enum() {
        let type_def = parse_type_definition("enum{a=42'd1,b=-42'sd2,c=42'd3}my_enum").unwrap();
        assert_eq!(
            type_def,
            Type::Enum {
                name: "my_enum".to_string(),
                variants: vec![
                    Variant {
                        name: "a".to_string(),
                        width: 42,
                        value: BigInt::from(1),
                    },
                    Variant {
                        name: "b".to_string(),
                        width: 42,
                        value: BigInt::from(-2),
                    },
                    Variant {
                        name: "c".to_string(),
                        width: 42,
                        value: BigInt::from(3),
                    },
                ],
                packed_dimensions: vec![],
                unpacked_dimensions: vec![],
            }
        );
    }
}
