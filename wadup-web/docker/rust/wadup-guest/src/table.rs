use crate::metadata;
use crate::types::{Column, DataType, Value};

pub struct Table {
    name: String,
}

impl Table {
    pub fn define(name: impl Into<String>, columns: Vec<(&str, DataType)>) -> Result<Self, String> {
        let name = name.into();

        let cols: Vec<Column> = columns
            .into_iter()
            .map(|(n, t)| Column {
                name: n.to_string(),
                data_type: t,
            })
            .collect();

        metadata::add_table(name.clone(), cols);

        Ok(Table { name })
    }

    pub fn insert(&self, values: &[Value]) -> Result<(), String> {
        metadata::add_row(self.name.clone(), values.to_vec());
        Ok(())
    }

    /// Flush accumulated metadata to file.
    ///
    /// This is optional - WADUP will automatically process any unflushed
    /// metadata after the module's process() function returns.
    pub fn flush() -> Result<(), String> {
        metadata::flush()
    }
}

pub struct TableBuilder {
    name: String,
    columns: Vec<(&'static str, DataType)>,
}

impl TableBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
        }
    }

    pub fn column(mut self, name: &'static str, data_type: DataType) -> Self {
        self.columns.push((name, data_type));
        self
    }

    pub fn build(self) -> Result<Table, String> {
        Table::define(self.name, self.columns)
    }
}

/// Flush accumulated metadata to file.
///
/// This is optional - WADUP will automatically process any unflushed
/// metadata after the module's process() function returns.
pub fn flush() -> Result<(), String> {
    metadata::flush()
}
