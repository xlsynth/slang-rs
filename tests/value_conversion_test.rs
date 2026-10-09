// SPDX-License-Identifier: Apache-2.0

use num_bigint::{BigInt, BigUint, Sign};
use slang_rs::{
    Compilation, ConstantValue, IntegerConversionError, IntegerValue, SlangConfig, Source,
};

fn integer(width: usize, signed: bool, value: impl Into<BigInt>) -> IntegerValue {
    let value = value.into();
    let bits = if value.sign() == Sign::Minus {
        (value + (BigInt::from(1u8) << width)).to_biguint().unwrap()
    } else {
        value.to_biguint().unwrap()
    };
    IntegerValue {
        width,
        signed,
        bits,
        x_mask: BigUint::from(0u8),
        z_mask: BigUint::from(0u8),
    }
}

macro_rules! check_integer {
    ($integer:expr, $ty:ty, $expected:expr) => {{
        let integer = $integer;
        let expected = $expected;
        assert_eq!(<$ty>::try_from(&integer), expected);
        let converted: Result<$ty, IntegerConversionError> = (&integer).try_into();
        assert_eq!(converted, expected);
        let value = ConstantValue::Integer(integer);
        assert_eq!(<$ty>::try_from(&value), expected);
        let converted: Result<$ty, IntegerConversionError> = (&value).try_into();
        assert_eq!(converted, expected);
    }};
}

#[test]
fn signed_conversions_check_mathematical_value_and_both_boundaries() {
    macro_rules! check_signed {
        ($ty:ty) => {{
            let out_of_range = Err(IntegerConversionError::OutOfRange {
                target: stringify!($ty),
            });
            check_integer!(
                integer(<$ty>::BITS as usize, true, <$ty>::MIN),
                $ty,
                Ok(<$ty>::MIN)
            );
            check_integer!(
                integer(<$ty>::BITS as usize, true, <$ty>::MAX),
                $ty,
                Ok(<$ty>::MAX)
            );
            check_integer!(integer(256, true, -3), $ty, Ok(-3));
            check_integer!(integer(256, false, 0), $ty, Ok(0));
            check_integer!(integer(256, false, <$ty>::MAX), $ty, Ok(<$ty>::MAX));
            check_integer!(
                integer(256, true, BigInt::from(<$ty>::MIN) - 1),
                $ty,
                out_of_range
            );
            check_integer!(
                integer(256, true, BigInt::from(<$ty>::MAX) + 1),
                $ty,
                out_of_range
            );
            check_integer!(
                integer(
                    <$ty>::BITS as usize,
                    false,
                    (BigInt::from(1u8) << <$ty>::BITS) - 1
                ),
                $ty,
                out_of_range
            );
        }};
    }
    check_signed!(i8);
    check_signed!(i16);
    check_signed!(i32);
    check_signed!(i64);
    check_signed!(i128);
    check_signed!(isize);
}

#[test]
fn unsigned_conversions_reject_negative_values_and_overflow() {
    macro_rules! check_unsigned {
        ($ty:ty) => {{
            let out_of_range = Err(IntegerConversionError::OutOfRange {
                target: stringify!($ty),
            });
            check_integer!(
                integer(<$ty>::BITS as usize, false, <$ty>::MAX),
                $ty,
                Ok(<$ty>::MAX)
            );
            check_integer!(integer(256, true, <$ty>::MAX), $ty, Ok(<$ty>::MAX));
            check_integer!(integer(256, false, 0), $ty, Ok(0));
            check_integer!(integer(256, true, -1), $ty, out_of_range);
            check_integer!(
                integer(256, false, BigInt::from(<$ty>::MAX) + 1),
                $ty,
                out_of_range
            );
        }};
    }
    check_unsigned!(u8);
    check_unsigned!(u16);
    check_unsigned!(u32);
    check_unsigned!(u64);
    check_unsigned!(u128);
    check_unsigned!(usize);
}

#[test]
fn unknown_bits_are_rejected_even_above_the_destination_width() {
    for unknown_is_x in [true, false] {
        let mut value = integer(256, false, 3);
        let mask = BigUint::from(1u8) << 200usize;
        if unknown_is_x {
            value.x_mask = mask;
        } else {
            value.z_mask = mask;
        }
        macro_rules! check_unknown {
            ($($ty:ty),+ $(,)?) => {
                $(check_integer!(value.clone(), $ty, Err(IntegerConversionError::UnknownBits));)+
            };
        }
        check_unknown!(
            i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize,
        );
        check_integer!(value, BigInt, Err(IntegerConversionError::UnknownBits));
    }
}

#[test]
fn noninteger_values_are_never_coerced_or_unwrapped() {
    let nested_integer = ConstantValue::Integer(integer(32, true, 42));
    for value in [
        ConstantValue::Invalid,
        ConstantValue::Real(42.0),
        ConstantValue::ShortReal(42.0),
        ConstantValue::String(b"42".to_vec()),
        ConstantValue::Elements(vec![nested_integer.clone()]),
        ConstantValue::Map {
            entries: vec![(nested_integer.clone(), nested_integer.clone())],
            default: Box::new(nested_integer.clone()),
        },
        ConstantValue::Queue {
            elements: vec![nested_integer.clone()],
            max_bound: 1,
        },
        ConstantValue::Union {
            value: Box::new(nested_integer),
            active_member: Some(0),
        },
        ConstantValue::Null,
        ConstantValue::Unbounded,
    ] {
        macro_rules! check_noninteger {
            ($($ty:ty),+ $(,)?) => {
                $(
                    assert_eq!(<$ty>::try_from(&value), Err(IntegerConversionError::NotInteger));
                    let converted: Result<$ty, IntegerConversionError> = (&value).try_into();
                    assert_eq!(converted, Err(IntegerConversionError::NotInteger));
                )+
            };
        }
        check_noninteger!(
            i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, BigInt,
        );
    }
}

#[test]
fn arbitrary_precision_conversion_remains_available() {
    for value in [
        BigInt::from(1u8) << 200usize,
        -(BigInt::from(1u8) << 200usize),
    ] {
        check_integer!(integer(256, true, value.clone()), BigInt, Ok(value));
    }
}

#[test]
fn native_package_parameters_offer_checked_conversions_directly() {
    let source = r#"package numeric;
      localparam int VALUE = 22;
      localparam logic signed [255:0] NEGATIVE = -256'sd3;
      localparam logic [63:0] MAX_U64 = 64'hffff_ffff_ffff_ffff;
      localparam logic [64:0] ABOVE_U64 = 65'h1_0000_0000_0000_0000;
      localparam logic [255:0] HIGH_X = {1'bx, 255'd3};
      localparam logic [255:0] HIGH_Z = {1'bz, 255'd3};
      localparam real REAL_VALUE = 22.0;
      localparam string TEXT = "22";
    endpackage"#;
    let packages = Compilation::new(&SlangConfig {
        sources: &[Source::text("numeric.sv", source)],
        ..Default::default()
    })
    .unwrap()
    .packages()
    .unwrap();
    let parameters = &packages["numeric"].parameters;
    assert_eq!(i32::try_from(&parameters["VALUE"]), Ok(22));
    let value: i32 = (&parameters["VALUE"]).try_into().unwrap();
    assert_eq!(value, 22);
    assert_eq!(i8::try_from(&parameters["NEGATIVE"]), Ok(-3));
    assert_eq!(u64::try_from(&parameters["MAX_U64"]), Ok(u64::MAX));
    assert_eq!(
        i64::try_from(&parameters["MAX_U64"]),
        Err(IntegerConversionError::OutOfRange { target: "i64" })
    );
    assert_eq!(
        u64::try_from(&parameters["ABOVE_U64"]),
        Err(IntegerConversionError::OutOfRange { target: "u64" })
    );
    assert_eq!(u128::try_from(&parameters["ABOVE_U64"]), Ok(1u128 << 64));
    for name in ["HIGH_X", "HIGH_Z"] {
        assert_eq!(
            u64::try_from(&parameters[name]),
            Err(IntegerConversionError::UnknownBits)
        );
    }
    for name in ["REAL_VALUE", "TEXT"] {
        assert_eq!(
            i32::try_from(&parameters[name]),
            Err(IntegerConversionError::NotInteger)
        );
    }
}
