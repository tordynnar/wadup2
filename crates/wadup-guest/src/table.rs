use crate::ffi;
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

        let columns_json = serde_json::to_string(&cols)
            .map_err(|e| format!("Failed to serialize columns: {}", e))?;

        unsafe {
            let result = ffi::define_table(
                name.as_ptr(),
                name.len(),
                columns_json.as_ptr(),
                columns_json.len(),
            );

            if result < 0 {
                return Err(format!("Failed to define table '{}'", name));
            }
        }

        Ok(Table { name })
    }

    pub fn insert(&self, values: &[Value]) -> Result<(), String> {
        let values_json = serde_json::to_string(values)
            .map_err(|e| format!("Failed to serialize values: {}", e))?;

        unsafe {
            let result = ffi::insert_row(
                self.name.as_ptr(),
                self.name.len(),
                values_json.as_ptr(),
                values_json.len(),
            );

            if result < 0 {
                return Err(format!("Failed to insert row into '{}'", self.name));
            }
        }

        Ok(())
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
