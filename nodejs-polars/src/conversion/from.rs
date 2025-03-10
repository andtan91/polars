use crate::conversion::wrap::*;
use crate::error::JsPolarsEr;
use napi::{
    JsBigint, JsBoolean, JsDate, JsNumber, JsObject, JsString, JsUnknown, Result, ValueType,
};
use polars::prelude::*;

pub trait FromJsUnknown: Sized + Send {
    fn from_js(obj: JsUnknown) -> Result<Self>;
}

impl FromJsUnknown for String {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsString = val.try_into()?;
        s.into_utf8()?.into_owned()
    }
}
impl FromJsUnknown for QuantileInterpolOptions {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s = String::from_js(val)?;
        match s.as_str() {
            "nearest" => Ok(QuantileInterpolOptions::Nearest),
            "lower" => Ok(QuantileInterpolOptions::Lower),
            "higher" => Ok(QuantileInterpolOptions::Higher),
            "midpoinp" => Ok(QuantileInterpolOptions::Midpoint),
            "linear" => Ok(QuantileInterpolOptions::Linear),
            s => panic!("quantile interpolation option {} is not supported", s),
        }
    }
}

impl FromJsUnknown for DistinctKeepStrategy {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s = String::from_js(val)?;
        match s.as_str() {
            "first" => Ok(DistinctKeepStrategy::First),
            "last" => Ok(DistinctKeepStrategy::Last),
            s => panic!("keep strategy {} is not supported", s),
        }
    }
}

impl<V> FromJsUnknown for Vec<V>
where
    V: FromJsUnknown,
{
    fn from_js(val: JsUnknown) -> Result<Self> {
        let obj: JsObject = match val.get_type()? {
            ValueType::Object => unsafe { val.cast() },
            dt => {
                return Err(JsPolarsEr::Other(format!(
                    "Invalid cast, unable to cast {} to array",
                    dt
                ))
                .into())
            }
        };
        let len = obj.get_array_length()?;
        let mut arr: Self = Vec::with_capacity(len as usize);
        for i in 0..len {
            let item: WrappedValue = obj.get_element::<JsUnknown>(i)?.into();
            let item = item.extract::<V>()?;
            arr.push(item);
        }
        Ok(arr)
    }
}

impl FromJsUnknown for AnyValue<'_> {
    fn from_js(val: JsUnknown) -> Result<Self> {
        match val.get_type()? {
            ValueType::Undefined | ValueType::Null => Ok(AnyValue::Null),
            ValueType::Boolean => bool::from_js(val).map(AnyValue::Boolean),
            ValueType::Number => f64::from_js(val).map(AnyValue::Float64),
            ValueType::String => {
                String::from_js(val).map(|s| AnyValue::Utf8(Box::leak::<'_>(s.into_boxed_str())))
            }
            ValueType::Bigint => u64::from_js(val).map(AnyValue::UInt64),
            ValueType::Object => {
                if val.is_date()? {
                    let d: JsDate = unsafe { val.cast() };
                    let d = d.value_of()?;
                    let d = d as i64;
                    Ok(AnyValue::Datetime(d, TimeUnit::Milliseconds, &None))
                } else {
                    Err(JsPolarsEr::Other("Unsupported Data type".to_owned()).into())
                }
            }
            _ => panic!("not supported"),
        }
    }
}

impl FromJsUnknown for Wrap<Utf8Chunked> {
    fn from_js(val: JsUnknown) -> Result<Self> {
        if val.is_array()? {
            let obj: JsObject = unsafe { val.cast() };
            let len = obj.get_array_length()?;
            let u_len = len as usize;
            let mut builder = Utf8ChunkedBuilder::new("", u_len, u_len * 25);
            for i in 0..len {
                let item: WrappedValue = obj.get_element::<JsUnknown>(i)?.into();
                match item.extract::<String>() {
                    Ok(val) => builder.append_value(val),
                    Err(_) => builder.append_null(),
                }
            }
            Ok(Wrap(builder.finish()))
        } else {
            Err(JsPolarsEr::Other("incorrect value type".to_owned()).into())
        }
    }
}

impl FromJsUnknown for Wrap<NullValues> {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let value_type = val.get_type()?;
        if value_type == ValueType::String {
            let jsv: JsString = unsafe { val.cast() };
            let js_string = jsv.into_utf8()?;
            let js_string = js_string.into_owned()?;
            Ok(Wrap(NullValues::AllColumns(js_string)))
        } else if value_type == ValueType::Object {
            let obj_val = unsafe { val.cast::<JsObject>() };
            if let Ok(len) = obj_val.get_array_length() {
                let cols: Vec<String> = (0..len)
                    .map(|idx| {
                        let s: JsString = obj_val.get_element_unchecked(idx).expect("array");
                        let js_string = s.into_utf8().expect("item to be of string");
                        js_string.into_owned().expect("item to be of string")
                    })
                    .collect();

                Ok(Wrap(NullValues::Columns(cols)))
            } else {
                let keys_obj = obj_val.get_property_names()?;
                if let Ok(len) = keys_obj.get_array_length() {
                    let cols: Vec<(String, String)> = (0..len)
                        .map(|idx| {
                            let key: JsString =
                                keys_obj.get_element_unchecked(idx).expect("key to exist");
                            let value: JsString =
                                obj_val.get_property(key).expect("value to exist");
                            let key_string = key.into_utf8().expect("item to be of string");
                            let value_string = value.into_utf8().expect("item to be of string");

                            let key_string = key_string.into_owned().unwrap();
                            let value_string = value_string.into_owned().unwrap();

                            (key_string, value_string)
                        })
                        .collect();
                    Ok(Wrap(NullValues::Named(cols)))
                } else {
                    Err(JsPolarsEr::Other(
                        "could not extract value from null_values argument".into(),
                    )
                    .into())
                }
            }
        } else {
            Err(
                JsPolarsEr::Other("could not extract value from null_values argument".into())
                    .into(),
            )
        }
    }
}

impl<'a> FromJsUnknown for &'a str {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsString = val.try_into()?;
        let s = s.into_utf8()?.into_owned()?;
        Ok(Box::leak::<'a>(s.into_boxed_str()))
    }
}

impl FromJsUnknown for bool {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsBoolean = val.try_into()?;
        s.try_into()
    }
}

impl FromJsUnknown for f64 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        s.try_into()
    }
}

impl FromJsUnknown for i64 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        match val.get_type()? {
            ValueType::Bigint => {
                let big: JsBigint = unsafe { val.cast() };
                big.try_into()
            }
            ValueType::Number => {
                let s: JsNumber = val.try_into()?;
                s.try_into()
            }
            dt => Err(JsPolarsEr::Other(format!("cannot cast {} to u64", dt)).into()),
        }
    }
}

impl FromJsUnknown for u64 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        match val.get_type()? {
            ValueType::Bigint => {
                let big: JsBigint = unsafe { val.cast() };
                big.try_into()
            }
            ValueType::Number => {
                let s: JsNumber = val.try_into()?;
                Ok(s.get_int64()? as u64)
            }
            dt => Err(JsPolarsEr::Other(format!("cannot cast {} to u64", dt)).into()),
        }
    }
}
impl FromJsUnknown for u32 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        s.get_uint32()
    }
}
impl FromJsUnknown for f32 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        s.get_double().map(|s| s as f32)
    }
}

impl FromJsUnknown for usize {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        Ok(s.get_uint32()? as usize)
    }
}
impl FromJsUnknown for u8 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        Ok(s.get_uint32()? as u8)
    }
}
impl FromJsUnknown for u16 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        Ok(s.get_uint32()? as u16)
    }
}
impl FromJsUnknown for i8 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        Ok(s.get_int32()? as i8)
    }
}
impl FromJsUnknown for i16 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        Ok(s.get_int32()? as i16)
    }
}

impl FromJsUnknown for i32 {
    fn from_js(val: JsUnknown) -> Result<Self> {
        let s: JsNumber = val.try_into()?;
        s.try_into()
    }
}

impl<V> FromJsUnknown for Option<V>
where
    V: FromJsUnknown,
{
    fn from_js(val: JsUnknown) -> Result<Self> {
        let v = V::from_js(val);
        match v {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }
}
