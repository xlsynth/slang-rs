// SPDX-License-Identifier: Apache-2.0

//! Owned values evaluated by Slang, without a textual serialization step.

use num_bigint::{BigInt, BigUint};
use std::fmt;

/// A value cannot be represented as the requested integer type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerConversionError {
    /// The constant is not an integer; strings and reals are not coerced.
    NotInteger,
    /// At least one bit is X or Z, preventing a numeric interpretation.
    UnknownBits,
    /// The mathematical value does not fit the target type. This includes
    /// negative values converted to an unsigned type.
    OutOfRange { target: &'static str },
}

impl fmt::Display for IntegerConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInteger => f.write_str("Value is not an integer"),
            Self::UnknownBits => f.write_str("Integer contains X or Z bits"),
            Self::OutOfRange { target } => write!(f, "Integer is out of range for {target}"),
        }
    }
}

impl std::error::Error for IntegerConversionError {}

macro_rules! integer_conversions {
    ($source:ty) => {
        $crate::values::integer_conversions! {
            $source; i8, i16, i32, i64, i128, isize,
            u8, u16, u32, u64, u128, usize,
        }
    };
    ($source:ty; $($target:ty),+ $(,)?) => {
        $(
            #[doc = concat!("Convert to `", stringify!($target), "`, respecting SystemVerilog signedness.")]
            ///
            /// The mathematical value must fit; a wider declared source width
            /// is allowed. No bits are truncated and no non-integer value is coerced.
            ///
            /// # Errors
            ///
            /// Returns an [`IntegerConversionError`](crate::IntegerConversionError)
            /// for a non-integer value, any X/Z bits, or an out-of-range value.
            impl TryFrom<&$source> for $target {
                type Error = $crate::IntegerConversionError;

                fn try_from(value: &$source) -> Result<Self, Self::Error> {
                    Self::try_from(num_bigint::BigInt::try_from(value)?).map_err(|_| {
                        $crate::IntegerConversionError::OutOfRange {
                            target: stringify!($target),
                        }
                    })
                }
            }
        )+
    };
}

pub(crate) use integer_conversions;

/// A SystemVerilog integer, including its declared width and four-state bits.
///
/// `bits` contains the known 0/1 bits. Positions set in `x_mask` or `z_mask`
/// are zero in `bits`; each mask uses bit zero as the least significant bit.
/// Use `i32::try_from(&value)` or `let n: i32 = (&value).try_into()?` for
/// checked conversions to Rust integer types. The same traits support
/// [`BigInt`] for arbitrary-width values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegerValue {
    pub width: usize,
    pub signed: bool,
    pub bits: BigUint,
    pub x_mask: BigUint,
    pub z_mask: BigUint,
}

impl IntegerValue {
    /// Whether any bit is X or Z.
    pub fn has_unknown(&self) -> bool {
        self.x_mask.bits() != 0 || self.z_mask.bits() != 0
    }
}

/// Convert known bits to a mathematical integer, respecting signedness.
///
/// # Errors
///
/// Returns an error when X or Z bits prevent a numeric interpretation.
impl TryFrom<&IntegerValue> for BigInt {
    type Error = IntegerConversionError;

    fn try_from(integer: &IntegerValue) -> Result<Self, Self::Error> {
        if integer.has_unknown() {
            return Err(IntegerConversionError::UnknownBits);
        }
        let value = BigInt::from(integer.bits.clone());
        if integer.signed && integer.width > 0 && integer.bits.bit((integer.width - 1) as u64) {
            Ok(value - (BigInt::from(1u8) << integer.width))
        } else {
            Ok(value)
        }
    }
}

integer_conversions!(IntegerValue);

/// A compile-time value evaluated by Slang.
///
/// Strings contain SystemVerilog bytes and need not be UTF-8. `Invalid` is
/// Slang's absent/indeterminate value, also used for an unspecified map default.
/// Integer constants support checked conversions such as `i32::try_from(&value)`.
/// Use [`BigInt::try_from`] for arbitrary-width integers.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstantValue {
    Invalid,
    Integer(IntegerValue),
    Real(f64),
    ShortReal(f32),
    String(Vec<u8>),
    Elements(Vec<ConstantValue>),
    Map {
        entries: Vec<(ConstantValue, ConstantValue)>,
        default: Box<ConstantValue>,
    },
    Queue {
        elements: Vec<ConstantValue>,
        max_bound: u32,
    },
    Union {
        value: Box<ConstantValue>,
        active_member: Option<u32>,
    },
    Null,
    Unbounded,
}

impl ConstantValue {
    pub fn as_integer(&self) -> Option<&IntegerValue> {
        match self {
            Self::Integer(value) => Some(value),
            _ => None,
        }
    }
}

/// Convert an integer constant to its mathematical value, respecting its
/// SystemVerilog signedness.
///
/// # Errors
///
/// Returns an error for non-integer values or integers containing X/Z bits.
impl TryFrom<&ConstantValue> for BigInt {
    type Error = IntegerConversionError;

    fn try_from(value: &ConstantValue) -> Result<Self, Self::Error> {
        Self::try_from(
            value
                .as_integer()
                .ok_or(IntegerConversionError::NotInteger)?,
        )
    }
}

integer_conversions!(ConstantValue);
