// SPDX-License-Identifier: Apache-2.0

//! Owned type descriptions extracted from Slang semantic types.

use crate::IntegerValue;

/// The native SystemVerilog primitive underlying an integral type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegralKind {
    Bit,
    Logic,
    Reg,
    Byte,
    ShortInt,
    Int,
    LongInt,
    Integer,
    Time,
}

#[derive(Debug, PartialEq)]
pub enum Type {
    Integral {
        kind: IntegralKind,
        signed: bool,
        four_state: bool,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
        bit_width: Option<usize>,
    },
    Struct {
        name: Option<String>,
        fields: Vec<Field>,
        signed: bool,
        four_state: bool,
        is_packed: bool,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
        bit_width: Option<usize>,
    },
    Union {
        name: Option<String>,
        fields: Vec<Field>,
        signed: bool,
        four_state: bool,
        is_packed: bool,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
        bit_width: Option<usize>,
    },
    Enum {
        name: Option<String>,
        variants: Vec<Variant>,
        signed: bool,
        four_state: bool,
        base_type: Box<Type>,
        packed_dimensions: Vec<Range>,
        unpacked_dimensions: Vec<Range>,
        bit_width: Option<usize>,
    },
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, PartialEq)]
pub struct Range {
    /// The left bound as declared, which may be smaller than the right bound.
    pub left: i32,
    /// The right bound as declared.
    pub right: i32,
}

#[derive(Debug, PartialEq)]
pub struct Variant {
    pub name: String,
    pub value: IntegerValue,
}

impl Type {
    /// The numeric width resolved by Slang, including packed dimensions.
    ///
    /// Returns an error for unpacked arrays, structs and unions.
    pub fn width(&self) -> Result<usize, &'static str> {
        let width = match self {
            Self::Integral { bit_width, .. }
            | Self::Struct { bit_width, .. }
            | Self::Union { bit_width, .. }
            | Self::Enum { bit_width, .. } => *bit_width,
        };
        width.ok_or("Type is not numeric")
    }
}
