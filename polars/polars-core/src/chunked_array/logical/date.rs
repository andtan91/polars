use super::*;
use crate::prelude::*;

pub type DateChunked = Logical<DateType, Int32Type>;

impl From<Int32Chunked> for DateChunked {
    fn from(ca: Int32Chunked) -> Self {
        DateChunked::new_logical(ca)
    }
}

impl Int32Chunked {
    pub fn into_date(self) -> DateChunked {
        DateChunked::new_logical(self)
    }
}

impl LogicalType for DateChunked {
    fn dtype(&self) -> &DataType {
        &DataType::Date
    }

    #[cfg(feature = "dtype-date")]
    fn get_any_value(&self, i: usize) -> AnyValue<'_> {
        self.0.get_any_value(i).into_date()
    }
}
