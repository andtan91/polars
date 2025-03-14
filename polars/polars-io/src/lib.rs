#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "private")]
pub mod aggregations;
#[cfg(not(feature = "private"))]
pub(crate) mod aggregations;

#[cfg(feature = "avro")]
#[cfg_attr(docsrs, doc(cfg(feature = "avro")))]
pub mod avro;
#[cfg(feature = "csv-file")]
#[cfg_attr(docsrs, doc(cfg(feature = "csv-file")))]
pub mod csv;
#[cfg(feature = "csv-file")]
#[cfg_attr(docsrs, doc(cfg(feature = "csv-file")))]
pub mod csv_core;
pub mod export;
#[cfg(feature = "ipc")]
#[cfg_attr(docsrs, doc(cfg(feature = "ipc")))]
pub mod ipc;
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
#[cfg(any(feature = "csv-file", feature = "parquet"))]
pub mod mmap;
mod options;
#[cfg(feature = "parquet")]
#[cfg_attr(docsrs, doc(cfg(feature = "feature")))]
pub mod parquet;
#[cfg(feature = "private")]
pub mod predicates;
#[cfg(not(feature = "private"))]
pub(crate) mod predicates;
pub mod prelude;
#[cfg(all(test, feature = "csv-file"))]
mod tests;
pub(crate) mod utils;

pub use options::*;

use arrow::error::Result as ArrowResult;

#[cfg(any(
    feature = "ipc",
    feature = "parquet",
    feature = "json",
    feature = "avro"
))]
use crate::aggregations::{apply_aggregations, ScanAggregation};
#[cfg(any(
    feature = "ipc",
    feature = "parquet",
    feature = "json",
    feature = "avro"
))]
use crate::predicates::PhysicalIoExpr;
use polars_core::frame::ArrowChunk;
use polars_core::prelude::*;
use std::io::{Read, Seek, Write};

pub trait SerReader<R>
where
    R: Read + Seek,
{
    fn new(reader: R) -> Self;

    /// Rechunk to a single chunk after Reading file.
    #[must_use]
    fn set_rechunk(self, _rechunk: bool) -> Self
    where
        Self: std::marker::Sized,
    {
        self
    }

    /// Take the SerReader and return a parsed DataFrame.
    fn finish(self) -> Result<DataFrame>;
}

pub trait SerWriter<W>
where
    W: Write,
{
    fn new(writer: W) -> Self;
    fn finish(self, df: &mut DataFrame) -> Result<()>;
}

pub trait ArrowReader {
    fn next_record_batch(&mut self) -> ArrowResult<Option<ArrowChunk>>;
}

#[cfg(any(
    feature = "ipc",
    feature = "parquet",
    feature = "json",
    feature = "avro"
))]
pub(crate) fn finish_reader<R: ArrowReader>(
    mut reader: R,
    rechunk: bool,
    n_rows: Option<usize>,
    predicate: Option<Arc<dyn PhysicalIoExpr>>,
    aggregate: Option<&[ScanAggregation]>,
    arrow_schema: &ArrowSchema,
) -> Result<DataFrame> {
    use polars_core::utils::accumulate_dataframes_vertical;

    let mut num_rows = 0;
    let mut parsed_dfs = Vec::with_capacity(1024);

    while let Some(batch) = reader.next_record_batch()? {
        num_rows += batch.len();

        let mut df = DataFrame::try_from((batch, arrow_schema.fields.as_slice()))?;

        if let Some(predicate) = &predicate {
            let s = predicate.evaluate(&df)?;
            let mask = s.bool().expect("filter predicates was not of type boolean");
            df = df.filter(mask)?;
        }

        apply_aggregations(&mut df, aggregate)?;

        parsed_dfs.push(df);
        if let Some(n) = n_rows {
            if num_rows >= n {
                break;
            }
        }
    }
    let mut df = accumulate_dataframes_vertical(parsed_dfs)?;

    // Aggregations must be applied a final time to aggregate the partitions
    apply_aggregations(&mut df, aggregate)?;

    match rechunk {
        true => Ok(df.agg_chunks()),
        false => Ok(df),
    }
}
