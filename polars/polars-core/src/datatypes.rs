//! # Data types supported by Polars.
//!
//! At the moment Polars doesn't include all data types available by Arrow. The goal is to
//! incrementally support more data types and prioritize these by usability.
//!
//! [See the AnyValue variants](enum.AnyValue.html#variants) for the data types that
//! are currently supported.
//!
#[cfg(feature = "dtype-categorical")]
use crate::chunked_array::categorical::RevMapping;
pub use crate::chunked_array::logical::*;
#[cfg(feature = "object")]
use crate::chunked_array::object::PolarsObjectSafe;
use crate::chunked_array::ops::sort::PlIsNan;
use crate::prelude::*;
use crate::utils::Wrap;
use ahash::RandomState;
use arrow::compute::arithmetics::basic::NativeArithmetics;
use arrow::compute::comparison::Simd8;
use arrow::datatypes::IntegerType;
pub use arrow::datatypes::{DataType as ArrowDataType, TimeUnit as ArrowTimeUnit};
use arrow::error::ArrowError;
use arrow::types::simd::Simd;
use arrow::types::NativeType;
use num::{Bounded, FromPrimitive, Num, NumCast, Zero};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub};

pub struct Utf8Type {}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ListType {}

pub struct CategoricalType {}

pub trait PolarsDataType: Send + Sync {
    fn get_dtype() -> DataType
    where
        Self: Sized;
}

macro_rules! impl_polars_datatype {
    ($ca:ident, $variant:ident, $physical:ty) => {
        pub struct $ca {}

        impl PolarsDataType for $ca {
            fn get_dtype() -> DataType {
                DataType::$variant
            }
        }
    };
}

impl_polars_datatype!(UInt8Type, UInt8, u8);
impl_polars_datatype!(UInt16Type, UInt16, u16);
impl_polars_datatype!(UInt32Type, UInt32, u32);
impl_polars_datatype!(UInt64Type, UInt64, u64);
impl_polars_datatype!(Int8Type, Int8, i8);
impl_polars_datatype!(Int16Type, Int16, i16);
impl_polars_datatype!(Int32Type, Int32, i32);
impl_polars_datatype!(Int64Type, Int64, i64);
impl_polars_datatype!(Float32Type, Float32, f32);
impl_polars_datatype!(Float64Type, Float64, f64);
impl_polars_datatype!(DateType, Date, i32);
impl_polars_datatype!(DatetimeType, Unknown, i64);
impl_polars_datatype!(DurationType, Unknown, i64);
impl_polars_datatype!(TimeType, Time, i64);

impl PolarsDataType for Utf8Type {
    fn get_dtype() -> DataType {
        DataType::Utf8
    }
}

pub struct BooleanType {}

impl PolarsDataType for BooleanType {
    fn get_dtype() -> DataType {
        DataType::Boolean
    }
}

impl PolarsDataType for ListType {
    fn get_dtype() -> DataType {
        // null as we cannot no anything without self.
        DataType::List(Box::new(DataType::Null))
    }
}

impl PolarsDataType for CategoricalType {
    fn get_dtype() -> DataType {
        DataType::Categorical
    }
}

#[cfg(feature = "object")]
#[cfg_attr(docsrs, doc(cfg(feature = "object")))]
pub struct ObjectType<T>(T);
#[cfg(feature = "object")]
pub type ObjectChunked<T> = ChunkedArray<ObjectType<T>>;

#[cfg(feature = "object")]
#[cfg_attr(docsrs, doc(cfg(feature = "object")))]
impl<T: PolarsObject> PolarsDataType for ObjectType<T> {
    fn get_dtype() -> DataType {
        DataType::Object(T::type_name())
    }
}

/// Any type that is not nested
pub trait PolarsSingleType: PolarsDataType {}

impl<T> PolarsSingleType for T where T: NativeType + PolarsDataType {}

impl PolarsSingleType for Utf8Type {}

pub type ListChunked = ChunkedArray<ListType>;
pub type BooleanChunked = ChunkedArray<BooleanType>;
pub type UInt8Chunked = ChunkedArray<UInt8Type>;
pub type UInt16Chunked = ChunkedArray<UInt16Type>;
pub type UInt32Chunked = ChunkedArray<UInt32Type>;
pub type UInt64Chunked = ChunkedArray<UInt64Type>;
pub type Int8Chunked = ChunkedArray<Int8Type>;
pub type Int16Chunked = ChunkedArray<Int16Type>;
pub type Int32Chunked = ChunkedArray<Int32Type>;
pub type Int64Chunked = ChunkedArray<Int64Type>;
pub type Float32Chunked = ChunkedArray<Float32Type>;
pub type Float64Chunked = ChunkedArray<Float64Type>;
pub type Utf8Chunked = ChunkedArray<Utf8Type>;
pub type CategoricalChunked = ChunkedArray<CategoricalType>;

pub trait NumericNative:
    PartialOrd
    + NativeType
    + Num
    + NumCast
    + Zero
    + Simd
    + Simd8
    + std::iter::Sum<Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
    + AddAssign
    + Bounded
    + FromPrimitive
    + PlIsNan
    + NativeArithmetics
{
}
impl NumericNative for i8 {}
impl NumericNative for i16 {}
impl NumericNative for i32 {}
impl NumericNative for i64 {}
impl NumericNative for u8 {}
impl NumericNative for u16 {}
impl NumericNative for u32 {}
impl NumericNative for u64 {}
impl NumericNative for f32 {}
impl NumericNative for f64 {}

pub trait PolarsNumericType: Send + Sync + PolarsDataType + 'static {
    type Native: NumericNative;
}
impl PolarsNumericType for UInt8Type {
    type Native = u8;
}
impl PolarsNumericType for UInt16Type {
    type Native = u16;
}
impl PolarsNumericType for UInt32Type {
    type Native = u32;
}
impl PolarsNumericType for UInt64Type {
    type Native = u64;
}
impl PolarsNumericType for Int8Type {
    type Native = i8;
}
impl PolarsNumericType for Int16Type {
    type Native = i16;
}
impl PolarsNumericType for Int32Type {
    type Native = i32;
}
impl PolarsNumericType for Int64Type {
    type Native = i64;
}
impl PolarsNumericType for Float32Type {
    type Native = f32;
}
impl PolarsNumericType for Float64Type {
    type Native = f64;
}

pub trait PolarsIntegerType: PolarsNumericType {}
impl PolarsIntegerType for UInt8Type {}
impl PolarsIntegerType for UInt16Type {}
impl PolarsIntegerType for UInt32Type {}
impl PolarsIntegerType for UInt64Type {}
impl PolarsIntegerType for Int8Type {}
impl PolarsIntegerType for Int16Type {}
impl PolarsIntegerType for Int32Type {}
impl PolarsIntegerType for Int64Type {}

pub trait PolarsFloatType: PolarsNumericType {}
impl PolarsFloatType for Float32Type {}
impl PolarsFloatType for Float64Type {}

#[derive(Debug, Clone)]
pub enum AnyValue<'a> {
    Null,
    /// A binary true or false.
    Boolean(bool),
    /// A UTF8 encoded string type.
    Utf8(&'a str),
    /// An unsigned 8-bit integer number.
    UInt8(u8),
    /// An unsigned 16-bit integer number.
    UInt16(u16),
    /// An unsigned 32-bit integer number.
    UInt32(u32),
    /// An unsigned 64-bit integer number.
    UInt64(u64),
    /// An 8-bit integer number.
    Int8(i8),
    /// A 16-bit integer number.
    Int16(i16),
    /// A 32-bit integer number.
    Int32(i32),
    /// A 64-bit integer number.
    Int64(i64),
    /// A 32-bit floating point number.
    Float32(f32),
    /// A 64-bit floating point number.
    Float64(f64),
    /// A 32-bit date representing the elapsed time since UNIX epoch (1970-01-01)
    /// in days (32 bits).
    #[cfg(feature = "dtype-date")]
    Date(i32),
    /// A 64-bit date representing the elapsed time since UNIX epoch (1970-01-01)
    /// in nanoseconds (64 bits).
    #[cfg(feature = "dtype-datetime")]
    Datetime(i64, TimeUnit, &'a Option<TimeZone>),
    // A 64-bit integer representing difference between date-times in nanoseconds or milliseconds
    #[cfg(feature = "dtype-duration")]
    Duration(i64, TimeUnit),
    /// A 64-bit time representing the elapsed time since midnight in nanoseconds
    #[cfg(feature = "dtype-time")]
    Time(i64),
    #[cfg(feature = "dtype-categorical")]
    Categorical(u32, &'a RevMapping),
    /// Nested type, contains arrays that are filled with one of the datetypes.
    List(Series),
    #[cfg(feature = "object")]
    /// Can be used to fmt and implements Any, so can be downcasted to the proper value type.
    Object(&'a dyn PolarsObjectSafe),
}

impl<'a> Hash for AnyValue<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use AnyValue::*;
        match self {
            Null => state.write_u64(u64::MAX / 2 + 135123),
            Int8(v) => state.write_i8(*v),
            Int16(v) => state.write_i16(*v),
            Int32(v) => state.write_i32(*v),
            Int64(v) => state.write_i64(*v),
            UInt8(v) => state.write_u8(*v),
            UInt16(v) => state.write_u16(*v),
            UInt32(v) => state.write_u32(*v),
            UInt64(v) => state.write_u64(*v),
            Utf8(s) => state.write(s.as_bytes()),
            Boolean(v) => state.write_u8(*v as u8),
            List(v) => Hash::hash(&Wrap(v.clone()), state),
            _ => unimplemented!(),
        }
    }
}

impl<'a> Eq for AnyValue<'a> {}

impl From<f64> for AnyValue<'_> {
    fn from(a: f64) -> Self {
        AnyValue::Float64(a)
    }
}
impl From<f32> for AnyValue<'_> {
    fn from(a: f32) -> Self {
        AnyValue::Float32(a)
    }
}
impl From<u32> for AnyValue<'_> {
    fn from(a: u32) -> Self {
        AnyValue::UInt32(a)
    }
}
impl From<u64> for AnyValue<'_> {
    fn from(a: u64) -> Self {
        AnyValue::UInt64(a)
    }
}
impl From<i64> for AnyValue<'_> {
    fn from(a: i64) -> Self {
        AnyValue::Int64(a)
    }
}
impl From<i32> for AnyValue<'_> {
    fn from(a: i32) -> Self {
        AnyValue::Int32(a)
    }
}
impl From<i16> for AnyValue<'_> {
    fn from(a: i16) -> Self {
        AnyValue::Int16(a)
    }
}
impl From<u16> for AnyValue<'_> {
    fn from(a: u16) -> Self {
        AnyValue::UInt16(a)
    }
}

impl From<i8> for AnyValue<'_> {
    fn from(a: i8) -> Self {
        AnyValue::Int8(a)
    }
}
impl From<u8> for AnyValue<'_> {
    fn from(a: u8) -> Self {
        AnyValue::UInt8(a)
    }
}

impl<'a, T> From<Option<T>> for AnyValue<'a>
where
    T: Into<AnyValue<'a>>,
{
    fn from(a: Option<T>) -> Self {
        match a {
            None => AnyValue::Null,
            Some(v) => v.into(),
        }
    }
}

impl<'a> AnyValue<'a> {
    #[cfg(any(feature = "dtype-date", feature = "dtype-datetime"))]
    pub(crate) fn into_date(self) -> Self {
        match self {
            #[cfg(feature = "dtype-date")]
            AnyValue::Int32(v) => AnyValue::Date(v),
            AnyValue::Null => AnyValue::Null,
            dt => panic!("cannot create date from other type. dtype: {}", dt),
        }
    }
    pub(crate) fn into_datetime(self, tu: TimeUnit, tz: &'a Option<TimeZone>) -> Self {
        match self {
            #[cfg(feature = "dtype-datetime")]
            AnyValue::Int64(v) => AnyValue::Datetime(v, tu, tz),
            AnyValue::Null => AnyValue::Null,
            dt => panic!("cannot create date from other type. dtype: {}", dt),
        }
    }

    pub(crate) fn into_duration(self, tu: TimeUnit) -> Self {
        match self {
            #[cfg(feature = "dtype-duration")]
            AnyValue::Int64(v) => AnyValue::Duration(v, tu),
            AnyValue::Null => AnyValue::Null,
            dt => panic!("cannot create date from other type. dtype: {}", dt),
        }
    }

    #[cfg(feature = "dtype-time")]
    pub(crate) fn into_time(self) -> Self {
        match self {
            AnyValue::Int64(v) => AnyValue::Time(v),
            AnyValue::Null => AnyValue::Null,
            dt => panic!("cannot create date from other type. dtype: {}", dt),
        }
    }

    #[must_use]
    pub fn add<'b>(&self, rhs: &AnyValue<'b>) -> Self {
        use AnyValue::*;
        match (self, rhs) {
            (Null, _) => Null,
            (_, Null) => Null,
            (Int32(l), Int32(r)) => Int32(l + r),
            (Int64(l), Int64(r)) => Int64(l + r),
            (UInt32(l), UInt32(r)) => UInt32(l + r),
            (UInt64(l), UInt64(r)) => UInt64(l + r),
            (Float32(l), Float32(r)) => Float32(l + r),
            (Float64(l), Float64(r)) => Float64(l + r),
            _ => todo!(),
        }
    }

    /// Try to coerce to an AnyValue with static lifetime.
    /// This can be done if it does not borrow any values.
    pub fn to_static(&self) -> Result<AnyValue<'static>> {
        use AnyValue::*;
        let av = match self {
            Null => AnyValue::Null,
            Int8(v) => AnyValue::Int8(*v),
            Int16(v) => AnyValue::Int16(*v),
            Int32(v) => AnyValue::Int32(*v),
            Int64(v) => AnyValue::Int64(*v),
            UInt8(v) => AnyValue::UInt8(*v),
            UInt16(v) => AnyValue::UInt16(*v),
            UInt32(v) => AnyValue::UInt32(*v),
            UInt64(v) => AnyValue::UInt64(*v),
            Boolean(v) => AnyValue::Boolean(*v),
            Float32(v) => AnyValue::Float32(*v),
            Float64(v) => AnyValue::Float64(*v),
            #[cfg(feature = "dtype-date")]
            Date(v) => AnyValue::Date(*v),
            #[cfg(feature = "dtype-time")]
            Time(v) => AnyValue::Time(*v),
            List(v) => AnyValue::List(v.clone()),
            dt => {
                return Err(PolarsError::ComputeError(
                    format!("cannot get static AnyValue from {}", dt).into(),
                ))
            }
        };
        Ok(av)
    }
}

impl<'a> From<AnyValue<'a>> for Option<i64> {
    fn from(val: AnyValue<'a>) -> Self {
        use AnyValue::*;
        match val {
            Null => None,
            Int32(v) => Some(v as i64),
            Int64(v) => Some(v as i64),
            UInt32(v) => Some(v as i64),
            _ => todo!(),
        }
    }
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DataType::Null => "null",
            DataType::Boolean => "bool",
            DataType::UInt8 => "u8",
            DataType::UInt16 => "u16",
            DataType::UInt32 => "u32",
            DataType::UInt64 => "u64",
            DataType::Int8 => "i8",
            DataType::Int16 => "i16",
            DataType::Int32 => "i32",
            DataType::Int64 => "i64",
            DataType::Float32 => "f32",
            DataType::Float64 => "f64",
            DataType::Utf8 => "str",
            DataType::Date => "date",
            DataType::Datetime(tu, tz) => {
                let s = match tz {
                    None => format!("datetime[{}]", tu),
                    Some(tz) => format!("datetime[{}, {}]", tu, tz),
                };
                return f.write_str(&s);
            }
            DataType::Duration(tu) => return write!(f, "duration[{}]", tu),
            DataType::Time => "time",
            DataType::List(tp) => return write!(f, "list [{}]", tp),
            #[cfg(feature = "object")]
            DataType::Object(s) => s,
            DataType::Categorical => "cat",
            DataType::Unknown => unreachable!(),
        };
        f.write_str(s)
    }
}

impl PartialEq for AnyValue<'_> {
    // Everything of Any is slow. Don't use.
    fn eq(&self, other: &Self) -> bool {
        use AnyValue::*;
        match (self, other) {
            (Utf8(l), Utf8(r)) => l == r,
            (UInt8(l), UInt8(r)) => l == r,
            (UInt16(l), UInt16(r)) => l == r,
            (UInt32(l), UInt32(r)) => l == r,
            (UInt64(l), UInt64(r)) => l == r,
            (Int8(l), Int8(r)) => l == r,
            (Int16(l), Int16(r)) => l == r,
            (Int32(l), Int32(r)) => l == r,
            (Int64(l), Int64(r)) => l == r,
            (Float32(l), Float32(r)) => l == r,
            (Float64(l), Float64(r)) => l == r,
            #[cfg(feature = "dtype-time")]
            (Time(l), Time(r)) => l == r,
            #[cfg(all(feature = "dtype-datetime", feature = "dtype-date"))]
            (Date(l), Date(r)) => l == r,
            #[cfg(all(feature = "dtype-datetime", feature = "dtype-date"))]
            (Datetime(l, tul, tzl), Datetime(r, tur, tzr)) => l == r && tul == tur && tzl == tzr,
            (Boolean(l), Boolean(r)) => l == r,
            (List(_), List(_)) => panic!("eq between list series not supported"),
            #[cfg(feature = "object")]
            (Object(_), Object(_)) => panic!("eq between object not supported"),
            // should it?
            (Null, Null) => true,
            #[cfg(feature = "dtype-categorical")]
            (Categorical(idx_l, rev_l), Categorical(idx_r, rev_r)) => match (rev_l, rev_r) {
                (RevMapping::Global(_, _, id_l), RevMapping::Global(_, _, id_r)) => {
                    id_l == id_r && idx_l == idx_r
                }
                (RevMapping::Local(arr_l), RevMapping::Local(arr_r)) => {
                    std::ptr::eq(arr_l, arr_r) && idx_l == idx_r
                }
                _ => false,
            },
            #[cfg(feature = "dtype-duration")]
            (Duration(l, tu_l), Duration(r, tu_r)) => l == r && tu_l == tu_r,
            _ => false,
        }
    }
}

impl PartialOrd for AnyValue<'_> {
    /// Only implemented for the same types and physical types!
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use AnyValue::*;
        match (self, other) {
            (UInt8(l), UInt8(r)) => l.partial_cmp(r),
            (UInt16(l), UInt16(r)) => l.partial_cmp(r),
            (UInt32(l), UInt32(r)) => l.partial_cmp(r),
            (UInt64(l), UInt64(r)) => l.partial_cmp(r),
            (Int8(l), Int8(r)) => l.partial_cmp(r),
            (Int16(l), Int16(r)) => l.partial_cmp(r),
            (Int32(l), Int32(r)) => l.partial_cmp(r),
            (Int64(l), Int64(r)) => l.partial_cmp(r),
            (Float32(l), Float32(r)) => l.partial_cmp(r),
            (Float64(l), Float64(r)) => l.partial_cmp(r),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    Nanoseconds,
    Milliseconds,
}

impl From<&ArrowTimeUnit> for TimeUnit {
    fn from(tu: &ArrowTimeUnit) -> Self {
        match tu {
            ArrowTimeUnit::Millisecond => TimeUnit::Milliseconds,
            ArrowTimeUnit::Nanosecond => TimeUnit::Nanoseconds,
            // will be cast
            ArrowTimeUnit::Microsecond => TimeUnit::Nanoseconds,
            // will be cast
            ArrowTimeUnit::Second => TimeUnit::Milliseconds,
        }
    }
}

impl Display for TimeUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeUnit::Nanoseconds => {
                write!(f, "ns")
            }
            TimeUnit::Milliseconds => {
                write!(f, "ms")
            }
        }
    }
}

impl TimeUnit {
    pub fn to_arrow(self) -> ArrowTimeUnit {
        match self {
            TimeUnit::Nanoseconds => ArrowTimeUnit::Nanosecond,
            TimeUnit::Milliseconds => ArrowTimeUnit::Millisecond,
        }
    }
}

pub type TimeZone = String;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum DataType {
    Boolean,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    /// String data
    Utf8,
    /// A 32-bit date representing the elapsed time since UNIX epoch (1970-01-01)
    /// in days (32 bits).
    Date,
    /// A 64-bit date representing the elapsed time since UNIX epoch (1970-01-01)
    /// in milliseconds (64 bits).
    Datetime(TimeUnit, Option<TimeZone>),
    // 64-bit integer representing difference between times in milliseconds or nanoseconds
    Duration(TimeUnit),
    /// A 64-bit time representing the elapsed time since midnight in nanoseconds
    Time,
    List(Box<DataType>),
    #[cfg(feature = "object")]
    /// A generic type that can be used in a `Series`
    /// &'static str can be used to determine/set inner type
    Object(&'static str),
    Null,
    Categorical,
    // some logical types we cannot know statically, e.g. Datetime
    Unknown,
}

impl DataType {
    pub fn inner_dtype(&self) -> Option<&DataType> {
        if let DataType::List(inner) = self {
            Some(&*inner)
        } else {
            None
        }
    }

    /// Convert to the physical data type
    #[must_use]
    pub fn to_physical(&self) -> DataType {
        use DataType::*;
        match self {
            Date => Int32,
            Datetime(_, _) => Int64,
            Duration(_) => Int64,
            Time => Int64,
            Categorical => UInt32,
            _ => self.clone(),
        }
    }

    /// Check if this dtype is a logical type
    pub fn is_logical(&self) -> bool {
        self != &self.to_physical()
    }

    /// Convert to an Arrow data type.
    pub fn to_arrow(&self) -> ArrowDataType {
        use DataType::*;
        match self {
            Boolean => ArrowDataType::Boolean,
            UInt8 => ArrowDataType::UInt8,
            UInt16 => ArrowDataType::UInt16,
            UInt32 => ArrowDataType::UInt32,
            UInt64 => ArrowDataType::UInt64,
            Int8 => ArrowDataType::Int8,
            Int16 => ArrowDataType::Int16,
            Int32 => ArrowDataType::Int32,
            Int64 => ArrowDataType::Int64,
            Float32 => ArrowDataType::Float32,
            Float64 => ArrowDataType::Float64,
            Utf8 => ArrowDataType::LargeUtf8,
            Date => ArrowDataType::Date32,
            Datetime(unit, tz) => ArrowDataType::Timestamp(unit.to_arrow(), tz.clone()),
            Duration(unit) => ArrowDataType::Duration(unit.to_arrow()),
            Time => ArrowDataType::Time64(ArrowTimeUnit::Nanosecond),
            List(dt) => ArrowDataType::LargeList(Box::new(arrow::datatypes::Field::new(
                "",
                dt.to_arrow(),
                true,
            ))),
            Null => ArrowDataType::Null,
            #[cfg(feature = "object")]
            Object(_) => panic!("cannot convert object to arrow"),
            Categorical => ArrowDataType::UInt32,
            Unknown => unreachable!(),
        }
    }
}

impl PartialEq<ArrowDataType> for DataType {
    fn eq(&self, other: &ArrowDataType) -> bool {
        let dt: DataType = other.into();
        self == &dt
    }
}

/// Characterizes the name and the [`DataType`] of a column.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Field {
    name: String,
    data_type: DataType,
}

impl Field {
    /// Creates a new `Field`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use polars_core::prelude::*;
    /// let f1 = Field::new("Fruit name", DataType::Utf8);
    /// let f2 = Field::new("Lawful", DataType::Boolean);
    /// let f2 = Field::new("Departure", DataType::Time);
    /// ```
    pub fn new(name: &str, data_type: DataType) -> Self {
        Field {
            name: name.to_string(),
            data_type,
        }
    }

    /// Returns a reference to the `Field` name.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use polars_core::prelude::*;
    /// let f = Field::new("Year", DataType::Int32);
    ///
    /// assert_eq!(f.name(), "Year");
    /// ```
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Returns a reference to the `Field` datatype.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use polars_core::prelude::*;
    /// let f = Field::new("Birthday", DataType::Date);
    ///
    /// assert_eq!(f.data_type(), &DataType::Date);
    /// ```
    pub fn data_type(&self) -> &DataType {
        &self.data_type
    }

    /// Sets the `Field` datatype.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use polars_core::prelude::*;
    /// let mut f = Field::new("Temperature", DataType::Int32);
    /// f.coerce(DataType::Float32);
    ///
    /// assert_eq!(f, Field::new("Temperature", DataType::Float32));
    /// ```
    pub fn coerce(&mut self, dtype: DataType) {
        self.data_type = dtype;
    }

    /// Sets the `Field` name.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use polars_core::prelude::*;
    /// let mut f = Field::new("Atomic number", DataType::UInt32);
    /// f.set_name("Proton".to_owned());
    ///
    /// assert_eq!(f, Field::new("Proton", DataType::UInt32));
    /// ```
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Converts the `Field` to an `arrow::datatypes::Field`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use polars_core::prelude::*;
    /// let f = Field::new("Value", DataType::Int64);
    /// let af = arrow::datatypes::Field::new("Value", arrow::datatypes::DataType::Int64, true);
    ///
    /// assert_eq!(f.to_arrow(), af);
    /// ```
    pub fn to_arrow(&self) -> ArrowField {
        ArrowField::new(&self.name, self.data_type.to_arrow(), true)
    }
}

#[cfg(feature = "private")]
pub trait IndexOfSchema {
    fn index_of(&self, name: &str) -> arrow::error::Result<usize>;
}

impl IndexOfSchema for ArrowSchema {
    fn index_of(&self, name: &str) -> arrow::error::Result<usize> {
        self.fields
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| {
                let valid_fields: Vec<String> =
                    self.fields.iter().map(|f| f.name.clone()).collect();
                ArrowError::InvalidArgumentError(format!(
                    "Unable to get field named \"{}\". Valid fields: {:?}",
                    name, valid_fields
                ))
            })
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Default)]
pub struct Schema {
    fields: Vec<Field>,
}

impl Schema {
    pub fn rename<I, J, T, S>(&self, old_names: I, new_names: J) -> Result<Schema>
    where
        I: IntoIterator<Item = T>,
        J: IntoIterator<Item = S>,
        T: AsRef<str>,
        S: AsRef<str>,
    {
        let idx = old_names
            .into_iter()
            .map(|name| self.index_of(name.as_ref()))
            .collect::<Result<Vec<_>>>()?;
        let mut new_fields = self.fields.clone();

        for (i, name) in idx.into_iter().zip(new_names) {
            let dt = new_fields[i].data_type.clone();
            new_fields[i] = Field::new(name.as_ref(), dt)
        }
        Ok(Self::new(new_fields))
    }

    pub fn new(fields: Vec<Field>) -> Self {
        Schema { fields }
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns an immutable reference of the vector of `Field` instances
    pub fn fields(&self) -> &Vec<Field> {
        &self.fields
    }

    /// Returns a mutable reference of the vector of `Field` instances
    pub fn fields_mut(&mut self) -> &mut Vec<Field> {
        &mut self.fields
    }

    /// Returns an immutable reference of a specific `Field` instance selected using an
    /// offset within the internal `fields` vector
    pub fn field(&self, i: usize) -> Option<&Field> {
        self.fields.get(i)
    }

    /// Returns an immutable reference of a specific `Field` instance selected by name
    pub fn field_with_name(&self, name: &str) -> Result<&Field> {
        Ok(&self.fields[self.index_of(name)?])
    }

    /// Find the index of the column with the given name
    pub fn index_of(&self, name: &str) -> Result<usize> {
        self.fields
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| {
                let valid_fields: Vec<String> =
                    self.fields.iter().map(|f| f.name.clone()).collect();
                PolarsError::NotFound(format!(
                    "Unable to get field named \"{}\". Valid fields: {:?}",
                    name, valid_fields
                ))
            })
    }

    pub fn to_arrow(&self) -> ArrowSchema {
        let fields: Vec<ArrowField> = self
            .fields
            .iter()
            .map(|f| {
                match f.data_type() {
                    // we must call this item, because the arrow crate names this item when creating a
                    // schema from record batches
                    DataType::List(dt) => ArrowField::new(
                        f.name(),
                        ArrowDataType::LargeList(Box::new(ArrowField::new(
                            "item",
                            dt.to_arrow(),
                            true,
                        ))),
                        true,
                    ),
                    DataType::Categorical => ArrowField::new(
                        f.name(),
                        ArrowDataType::Dictionary(
                            IntegerType::UInt32,
                            Box::new(ArrowDataType::LargeUtf8),
                            false,
                        ),
                        true,
                    ),
                    _ => f.to_arrow(),
                }
            })
            .collect();
        ArrowSchema::from(fields)
    }

    pub fn try_merge(schemas: &[Self]) -> Result<Self> {
        let mut merged = Self::default();

        for schema in schemas {
            // merge fields
            for field in &schema.fields {
                let mut new_field = true;
                for merged_field in &mut merged.fields {
                    if field.name != merged_field.name {
                        continue;
                    }
                    new_field = false;
                }
                // found a new field, add to field list
                if new_field {
                    merged.fields.push(field.clone());
                }
            }
        }

        Ok(merged)
    }

    pub fn column_with_name(&self, name: &str) -> Option<(usize, &Field)> {
        self.fields
            .iter()
            .enumerate()
            .find(|&(_, c)| c.name == name)
    }
}

pub type SchemaRef = Arc<Schema>;

impl From<&ArrowDataType> for DataType {
    fn from(dt: &ArrowDataType) -> Self {
        match dt {
            ArrowDataType::Null => DataType::Null,
            ArrowDataType::UInt8 => DataType::UInt8,
            ArrowDataType::UInt16 => DataType::UInt16,
            ArrowDataType::UInt32 => DataType::UInt32,
            ArrowDataType::UInt64 => DataType::UInt64,
            ArrowDataType::Int8 => DataType::Int8,
            ArrowDataType::Int16 => DataType::Int16,
            ArrowDataType::Int32 => DataType::Int32,
            ArrowDataType::Int64 => DataType::Int64,
            ArrowDataType::Boolean => DataType::Boolean,
            ArrowDataType::Float32 => DataType::Float32,
            ArrowDataType::Float64 => DataType::Float64,
            ArrowDataType::LargeList(f) => DataType::List(Box::new(f.data_type().into())),
            ArrowDataType::List(f) => DataType::List(Box::new(f.data_type().into())),
            ArrowDataType::Date32 => DataType::Date,
            ArrowDataType::Timestamp(tu, tz) => DataType::Datetime(tu.into(), tz.clone()),
            ArrowDataType::Duration(tu) => DataType::Duration(tu.into()),
            ArrowDataType::Date64 => DataType::Datetime(TimeUnit::Milliseconds, None),
            ArrowDataType::LargeUtf8 => DataType::Utf8,
            ArrowDataType::Utf8 => DataType::Utf8,
            ArrowDataType::Time64(_) | ArrowDataType::Time32(_) => DataType::Time,
            ArrowDataType::Dictionary(_, _, _) => DataType::Categorical,
            ArrowDataType::Extension(name, _, _) if name == "POLARS_EXTENSION_TYPE" => {
                #[cfg(feature = "object")]
                {
                    DataType::Object("extension")
                }
                #[cfg(not(feature = "object"))]
                {
                    panic!("activate the 'object' feature to be able to load POLARS_EXTENSION_TYPE")
                }
            }
            dt => panic!("Arrow datatype {:?} not supported by Polars", dt),
        }
    }
}

impl From<&ArrowField> for Field {
    fn from(f: &ArrowField) -> Self {
        Field::new(&f.name, f.data_type().into())
    }
}
impl From<&ArrowSchema> for Schema {
    fn from(a_schema: &ArrowSchema) -> Self {
        Schema::new(
            a_schema
                .fields
                .iter()
                .map(|arrow_f| arrow_f.into())
                .collect(),
        )
    }
}
impl From<ArrowSchema> for Schema {
    fn from(a_schema: ArrowSchema) -> Self {
        (&a_schema).into()
    }
}

#[cfg(feature = "private")]
pub type PlHashMap<K, V> = hashbrown::HashMap<K, V, RandomState>;
#[cfg(feature = "private")]
pub type PlHashSet<V> = hashbrown::HashSet<V, RandomState>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_arrow_dtypes_to_polars() {
        let dtypes = [
            (
                ArrowDataType::Duration(ArrowTimeUnit::Nanosecond),
                DataType::Duration(TimeUnit::Nanoseconds),
            ),
            (
                ArrowDataType::Duration(ArrowTimeUnit::Millisecond),
                DataType::Duration(TimeUnit::Milliseconds),
            ),
            (
                ArrowDataType::Date64,
                DataType::Datetime(TimeUnit::Milliseconds, None),
            ),
            (
                ArrowDataType::Timestamp(ArrowTimeUnit::Nanosecond, None),
                DataType::Datetime(TimeUnit::Nanoseconds, None),
            ),
            (
                ArrowDataType::Timestamp(ArrowTimeUnit::Microsecond, None),
                DataType::Datetime(TimeUnit::Nanoseconds, None),
            ),
            (
                ArrowDataType::Timestamp(ArrowTimeUnit::Millisecond, None),
                DataType::Datetime(TimeUnit::Milliseconds, None),
            ),
            (
                ArrowDataType::Timestamp(ArrowTimeUnit::Second, None),
                DataType::Datetime(TimeUnit::Milliseconds, None),
            ),
            (
                ArrowDataType::Timestamp(ArrowTimeUnit::Second, Some("".to_string())),
                DataType::Datetime(TimeUnit::Milliseconds, Some("".to_string())),
            ),
            (ArrowDataType::LargeUtf8, DataType::Utf8),
            (ArrowDataType::Utf8, DataType::Utf8),
            (
                ArrowDataType::Time64(ArrowTimeUnit::Nanosecond),
                DataType::Time,
            ),
            (
                ArrowDataType::Time64(ArrowTimeUnit::Millisecond),
                DataType::Time,
            ),
            (
                ArrowDataType::Time64(ArrowTimeUnit::Microsecond),
                DataType::Time,
            ),
            (ArrowDataType::Time64(ArrowTimeUnit::Second), DataType::Time),
            (
                ArrowDataType::Time32(ArrowTimeUnit::Nanosecond),
                DataType::Time,
            ),
            (
                ArrowDataType::Time32(ArrowTimeUnit::Millisecond),
                DataType::Time,
            ),
            (
                ArrowDataType::Time32(ArrowTimeUnit::Microsecond),
                DataType::Time,
            ),
            (ArrowDataType::Time32(ArrowTimeUnit::Second), DataType::Time),
            (
                ArrowDataType::List(Box::new(ArrowField::new(
                    "item",
                    ArrowDataType::Float64,
                    true,
                ))),
                DataType::List(DataType::Float64.into()),
            ),
            (
                ArrowDataType::LargeList(Box::new(ArrowField::new(
                    "item",
                    ArrowDataType::Float64,
                    true,
                ))),
                DataType::List(DataType::Float64.into()),
            ),
            (
                ArrowDataType::Dictionary(IntegerType::UInt32, ArrowDataType::Utf8.into(), false),
                DataType::Categorical,
            ),
            (
                ArrowDataType::Dictionary(
                    IntegerType::UInt32,
                    ArrowDataType::LargeUtf8.into(),
                    false,
                ),
                DataType::Categorical,
            ),
            (
                ArrowDataType::Dictionary(
                    IntegerType::UInt64,
                    ArrowDataType::LargeUtf8.into(),
                    false,
                ),
                DataType::Categorical,
            ),
        ];

        for (dt_a, dt_p) in dtypes {
            let dt: DataType = (&dt_a).into();

            assert_eq!(dt_p, dt);
        }
    }
}
