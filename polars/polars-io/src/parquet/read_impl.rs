use crate::aggregations::{apply_aggregations, ScanAggregation};
use crate::mmap::{MmapBytesReader, ReaderBytes};
use crate::parquet::predicates::collect_statistics;
use crate::predicates::{apply_predicate, arrow_schema_to_empty_df, PhysicalIoExpr};
use crate::utils::apply_projection;
use arrow::io::parquet::read;
use arrow::io::parquet::read::{to_deserializer, FileMetaData};
use polars_core::prelude::*;
use polars_core::utils::accumulate_dataframes_vertical;
use polars_core::POOL;
use rayon::prelude::*;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::io::Cursor;
use std::ops::Deref;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn read_parquet<R: MmapBytesReader>(
    reader: R,
    limit: usize,
    projection: Option<&[usize]>,
    schema: &ArrowSchema,
    metadata: Option<FileMetaData>,
    predicate: Option<Arc<dyn PhysicalIoExpr>>,
    aggregate: Option<&[ScanAggregation]>,
    parallel: bool,
) -> Result<DataFrame> {
    let reader = ReaderBytes::from(&reader);
    let bytes = reader.deref();
    let mut reader = Cursor::new(bytes);

    let file_metadata = metadata
        .map(Ok)
        .unwrap_or_else(|| read::read_metadata(&mut reader))?;
    let row_group_len = file_metadata.row_groups.len();

    let projection = projection
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned((0usize..schema.fields.len()).collect::<Vec<_>>()));

    let mut dfs = Vec::with_capacity(row_group_len);

    let mut remaining_rows = limit;

    for rg in 0..row_group_len {
        let md = &file_metadata.row_groups[rg];
        if let Some(pred) = &predicate {
            if let Some(pred) = pred.as_stats_evaluator() {
                if let Some(stats) = collect_statistics(md.columns(), schema)? {
                    let should_read = pred.should_read(&stats);
                    // a parquet file may not have statistics of all columns
                    if matches!(should_read, Ok(false)) {
                        continue;
                    } else if !matches!(should_read, Err(PolarsError::NotFound(_))) {
                        let _ = should_read?;
                    }
                }
            }
        }

        // test we don't read the parquet file if this env var is set
        #[cfg(debug_assertions)]
        {
            assert!(std::env::var("POLARS_PANIC_IF_PARQUET_PARSED").is_err())
        }

        let chunk_size = md.num_rows() as usize;
        let columns = if parallel {
            POOL.install(|| {
                projection
                    .par_iter()
                    .map(|column_i| {
                        let mut reader = Cursor::new(bytes);
                        let field = &schema.fields[*column_i];
                        let columns = read::read_columns(&mut reader, md.columns(), &field.name)?;
                        let mut iter = to_deserializer(
                            columns,
                            field.clone(),
                            remaining_rows,
                            Some(chunk_size),
                        )?;

                        Series::try_from((field.name.as_str(), iter.next().unwrap()?))
                    })
                    .collect::<Result<Vec<_>>>()
            })?
        } else {
            projection
                .iter()
                .map(|column_i| {
                    let field = &schema.fields[*column_i];
                    let columns = read::read_columns(&mut reader, md.columns(), &field.name)?;
                    let mut iter =
                        to_deserializer(columns, field.clone(), remaining_rows, Some(chunk_size))?;

                    Series::try_from((field.name.as_str(), iter.next().unwrap()?))
                })
                .collect::<Result<Vec<_>>>()?
        };

        remaining_rows = file_metadata.row_groups[rg].num_rows() as usize;

        let mut df = DataFrame::new_no_checks(columns);

        apply_predicate(&mut df, predicate.as_deref())?;
        apply_aggregations(&mut df, aggregate)?;

        dfs.push(df)
    }

    if dfs.is_empty() {
        let schema = if let Cow::Borrowed(_) = projection {
            Cow::Owned(apply_projection(schema, &projection))
        } else {
            Cow::Borrowed(schema)
        };
        Ok(arrow_schema_to_empty_df(&schema))
    } else {
        let mut df = accumulate_dataframes_vertical(dfs.into_iter())?;
        apply_aggregations(&mut df, aggregate)?;
        Ok(df.slice(0, limit))
    }
}
