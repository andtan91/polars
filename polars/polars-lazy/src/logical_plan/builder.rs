use crate::logical_plan::projection::rewrite_projections;
use crate::prelude::*;
use crate::utils;
use crate::utils::{combine_predicates_expr, has_expr};
use ahash::RandomState;
use polars_core::prelude::*;
use polars_io::csv::CsvEncoding;
#[cfg(feature = "csv-file")]
use polars_io::csv_core::utils::infer_file_schema;
#[cfg(feature = "ipc")]
use polars_io::ipc::IpcReader;
#[cfg(feature = "parquet")]
use polars_io::parquet::ParquetReader;
use polars_io::RowCount;
#[cfg(feature = "csv-file")]
use polars_io::{
    csv::NullValues,
    csv_core::utils::{get_reader_bytes, is_compressed},
};
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub(crate) fn prepare_projection(exprs: Vec<Expr>, schema: &Schema) -> (Vec<Expr>, Schema) {
    let exprs = rewrite_projections(exprs, schema, &[]);
    let schema = utils::expressions_to_schema(&exprs, schema, Context::Default);
    (exprs, schema)
}

pub struct LogicalPlanBuilder(LogicalPlan);

impl From<LogicalPlan> for LogicalPlanBuilder {
    fn from(lp: LogicalPlan) -> Self {
        LogicalPlanBuilder(lp)
    }
}

impl LogicalPlanBuilder {
    #[cfg(feature = "parquet")]
    #[cfg_attr(docsrs, doc(cfg(feature = "parquet")))]
    pub fn scan_parquet<P: Into<PathBuf>>(
        path: P,
        n_rows: Option<usize>,
        cache: bool,
        parallel: bool,
    ) -> Result<Self> {
        use polars_io::SerReader as _;

        let path = path.into();
        let file = std::fs::File::open(&path)?;
        let schema = Arc::new(ParquetReader::new(file).schema()?);

        Ok(LogicalPlan::ParquetScan {
            path,
            schema,
            predicate: None,
            aggregate: vec![],
            options: ParquetOptions {
                n_rows,
                with_columns: None,
                cache,
                parallel,
            },
        }
        .into())
    }

    #[cfg(feature = "ipc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ipc")))]
    pub fn scan_ipc<P: Into<PathBuf>>(path: P, options: LpScanOptions) -> Result<Self> {
        use polars_io::SerReader as _;

        let path = path.into();
        let file = std::fs::File::open(&path)?;
        let schema = Arc::new(IpcReader::new(file).schema()?);

        Ok(LogicalPlan::IpcScan {
            path,
            schema,
            predicate: None,
            aggregate: vec![],
            options,
        }
        .into())
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "csv-file")]
    pub fn scan_csv<P: Into<PathBuf>>(
        path: P,
        delimiter: u8,
        has_header: bool,
        ignore_errors: bool,
        mut skip_rows: usize,
        n_rows: Option<usize>,
        cache: bool,
        schema: Option<Arc<Schema>>,
        schema_overwrite: Option<&Schema>,
        low_memory: bool,
        comment_char: Option<u8>,
        quote_char: Option<u8>,
        null_values: Option<NullValues>,
        infer_schema_length: Option<usize>,
        rechunk: bool,
        skip_rows_after_header: usize,
        encoding: CsvEncoding,
        row_count: Option<RowCount>,
    ) -> Result<Self> {
        let path = path.into();
        let mut file = std::fs::File::open(&path)?;
        let mut magic_nr = [0u8; 2];
        file.read_exact(&mut magic_nr)?;
        if is_compressed(&magic_nr) {
            return Err(PolarsError::ComputeError(
                "cannot scan compressed csv; use read_csv for compressed data".into(),
            ));
        }
        file.seek(SeekFrom::Start(0))?;
        let reader_bytes = get_reader_bytes(&mut file).expect("could not mmap file");

        let schema = schema.unwrap_or_else(|| {
            let (schema, _) = infer_file_schema(
                &reader_bytes,
                delimiter,
                infer_schema_length,
                has_header,
                schema_overwrite,
                &mut skip_rows,
                comment_char,
                quote_char,
                null_values.as_ref(),
            )
            .expect("could not read schema");
            Arc::new(schema)
        });
        skip_rows += skip_rows_after_header;
        Ok(LogicalPlan::CsvScan {
            path,
            schema,
            options: CsvParserOptions {
                has_header,
                delimiter,
                ignore_errors,
                skip_rows,
                n_rows,
                with_columns: None,
                low_memory,
                cache,
                comment_char,
                quote_char,
                null_values,
                rechunk,
                encoding,
                row_count,
            },
            predicate: None,
            aggregate: vec![],
        }
        .into())
    }

    pub fn cache(self) -> Self {
        LogicalPlan::Cache {
            input: Box::new(self.0),
        }
        .into()
    }

    pub fn project(self, exprs: Vec<Expr>) -> Self {
        let (exprs, schema) = prepare_projection(exprs, self.0.schema());

        if exprs.is_empty() {
            self.map(
                |_| Ok(DataFrame::new_no_checks(vec![])),
                AllowedOptimizations::default(),
                Some(Arc::new(Schema::default())),
                "EMPTY PROJECTION",
            )
        } else {
            LogicalPlan::Projection {
                expr: exprs,
                input: Box::new(self.0),
                schema: Arc::new(schema),
            }
            .into()
        }
    }

    pub fn project_local(self, exprs: Vec<Expr>) -> Self {
        let (exprs, schema) = prepare_projection(exprs, self.0.schema());
        if !exprs.is_empty() {
            LogicalPlan::LocalProjection {
                expr: exprs,
                input: Box::new(self.0),
                schema: Arc::new(schema),
            }
            .into()
        } else {
            self
        }
    }

    pub fn fill_null(self, fill_value: Expr) -> Self {
        let schema = self.0.schema();
        let exprs = schema
            .fields()
            .iter()
            .map(|field| {
                let name = field.name();
                when(col(name).is_null())
                    .then(fill_value.clone())
                    .otherwise(col(name))
                    .alias(name)
            })
            .collect();
        self.project_local(exprs)
    }

    pub fn fill_nan(self, fill_value: Expr) -> Self {
        let schema = self.0.schema();
        let exprs = schema
            .fields()
            .iter()
            .map(|field| {
                let name = field.name();
                when(col(name).is_nan())
                    .then(fill_value.clone())
                    .otherwise(col(name))
                    .alias(name)
            })
            .collect();
        self.project_local(exprs)
    }

    pub fn with_columns(self, exprs: Vec<Expr>) -> Self {
        // current schema
        let schema = self.0.schema();
        let mut new_fields = schema.fields().clone();
        let (exprs, _) = prepare_projection(exprs, schema);

        for e in &exprs {
            let field = e.to_field(schema, Context::Default).unwrap();
            match schema.index_of(field.name()) {
                Ok(idx) => {
                    new_fields[idx] = field;
                }
                Err(_) => new_fields.push(field),
            }
        }

        let new_schema = Schema::new(new_fields);

        LogicalPlan::HStack {
            input: Box::new(self.0),
            exprs,
            schema: Arc::new(new_schema),
        }
        .into()
    }

    /// Apply a filter
    pub fn filter(self, predicate: Expr) -> Self {
        let predicate = if has_expr(&predicate, |e| matches!(e, Expr::Wildcard)) {
            let rewritten = rewrite_projections(vec![predicate], self.0.schema(), &[]);
            combine_predicates_expr(rewritten.into_iter())
        } else {
            predicate
        };
        LogicalPlan::Selection {
            predicate,
            input: Box::new(self.0),
        }
        .into()
    }

    pub fn groupby<E: AsRef<[Expr]>>(
        self,
        keys: Arc<Vec<Expr>>,
        aggs: E,
        apply: Option<Arc<dyn DataFrameUdf>>,
        maintain_order: bool,
        dynamic_options: Option<DynamicGroupOptions>,
        rolling_options: Option<RollingGroupOptions>,
    ) -> Self {
        let current_schema = self.0.schema();
        let aggs = rewrite_projections(aggs.as_ref().to_vec(), current_schema, keys.as_ref());

        let schema1 = utils::expressions_to_schema(&keys, current_schema, Context::Default);
        let schema2 = utils::expressions_to_schema(&aggs, current_schema, Context::Aggregation);
        let schema = Schema::try_merge(&[schema1, schema2]).unwrap();

        LogicalPlan::Aggregate {
            input: Box::new(self.0),
            keys,
            aggs,
            schema: Arc::new(schema),
            apply,
            maintain_order,
            options: GroupbyOptions {
                dynamic: dynamic_options,
                rolling: rolling_options,
            },
        }
        .into()
    }

    pub fn build(self) -> LogicalPlan {
        self.0
    }

    pub fn from_existing_df(df: DataFrame) -> Self {
        let schema = Arc::new(df.schema());
        LogicalPlan::DataFrameScan {
            df: Arc::new(df),
            schema,
            projection: None,
            selection: None,
        }
        .into()
    }

    pub fn sort(self, by_column: Vec<Expr>, reverse: Vec<bool>) -> Self {
        LogicalPlan::Sort {
            input: Box::new(self.0),
            by_column,
            reverse,
        }
        .into()
    }

    pub fn explode(self, columns: Vec<Expr>) -> Self {
        let columns = rewrite_projections(columns, self.0.schema(), &[]);
        // columns to string
        let columns = columns
            .iter()
            .map(|e| {
                if let Expr::Column(name) = e {
                    (**name).to_owned()
                } else {
                    panic!("expected column expression")
                }
            })
            .collect();
        LogicalPlan::Explode {
            input: Box::new(self.0),
            columns,
        }
        .into()
    }

    pub fn melt(self, id_vars: Arc<Vec<String>>, value_vars: Arc<Vec<String>>) -> Self {
        let schema = det_melt_schema(&value_vars, self.0.schema());
        LogicalPlan::Melt {
            input: Box::new(self.0),
            id_vars,
            value_vars,
            schema,
        }
        .into()
    }

    pub fn distinct(self, options: DistinctOptions) -> Self {
        LogicalPlan::Distinct {
            input: Box::new(self.0),
            options,
        }
        .into()
    }

    pub fn slice(self, offset: i64, len: u32) -> Self {
        LogicalPlan::Slice {
            input: Box::new(self.0),
            offset,
            len,
        }
        .into()
    }

    pub fn join(
        self,
        other: LogicalPlan,
        left_on: Vec<Expr>,
        right_on: Vec<Expr>,
        options: JoinOptions,
    ) -> Self {
        let schema_left = self.0.schema();
        let schema_right = other.schema();

        // column names of left table
        let mut names: HashSet<&String, RandomState> = HashSet::default();
        // fields of new schema
        let mut fields = vec![];

        for f in schema_left.fields() {
            names.insert(f.name());
            fields.push(f.clone());
        }

        let right_names: HashSet<_, RandomState> = right_on
            .iter()
            .map(|e| utils::expr_output_name(e).expect("could not find name"))
            .collect();

        for f in schema_right.fields() {
            let name = f.name();

            if !right_names.iter().any(|s| s.as_ref() == name) {
                if names.contains(name) {
                    let new_name = format!("{}{}", name, options.suffix.as_ref());
                    let field = Field::new(&new_name, f.data_type().clone());
                    fields.push(field)
                } else {
                    fields.push(f.clone())
                }
            }
        }

        let schema = Arc::new(Schema::new(fields));

        LogicalPlan::Join {
            input_left: Box::new(self.0),
            input_right: Box::new(other),
            schema,
            left_on,
            right_on,
            options,
        }
        .into()
    }
    pub fn map<F>(
        self,
        function: F,
        optimizations: AllowedOptimizations,
        schema: Option<SchemaRef>,
        name: &'static str,
    ) -> Self
    where
        F: DataFrameUdf + 'static,
    {
        let options = LogicalPlanUdfOptions {
            predicate_pd: optimizations.predicate_pushdown,
            projection_pd: optimizations.projection_pushdown,
            fmt_str: name,
        };

        LogicalPlan::Udf {
            input: Box::new(self.0),
            function: Arc::new(function),
            options,
            schema,
        }
        .into()
    }
}

pub(crate) fn det_melt_schema(value_vars: &[String], input_schema: &Schema) -> SchemaRef {
    let mut fields = input_schema
        .fields()
        .iter()
        .filter(|field| !value_vars.contains(field.name()))
        .cloned()
        .collect::<Vec<_>>();

    fields.reserve(2);

    let value_dtype = input_schema
        .field_with_name(&value_vars[0])
        .expect("field not found")
        .data_type();

    fields.push(Field::new("variable", DataType::Utf8));
    fields.push(Field::new("value", value_dtype.clone()));

    Arc::new(Schema::new(fields))
}
