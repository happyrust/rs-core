use surrealdb::types::{RecordId, ToSql, Value};

use crate::utils::RecordIdExt;

pub fn value_to_string(value: &Value) -> String {
	match value {
		Value::String(s) => s.clone(),
		Value::Number(n) => n.to_string(),
		Value::RecordId(rid) => rid.to_raw(),
		Value::Bool(b) => b.to_string(),
		other => {
			let mut buf = String::new();
			other.fmt_sql(&mut buf);
			buf
		}
	}
}

pub fn value_to_f32(value: &Value) -> f32 {
	match value {
		Value::Number(n) => n.to_f64().unwrap_or_default() as f32,
		Value::String(s) => s.parse::<f32>().unwrap_or_default(),
		Value::Bool(b) => {
			if *b {
				1.0
			} else {
				0.0
			}
		}
		_ => 0.0,
	}
}

pub fn value_to_i32(value: &Value) -> i32 {
	match value {
		Value::Number(n) => n.to_int().unwrap_or_default() as i32,
		Value::String(s) => s.parse::<i32>().unwrap_or_default(),
		Value::Bool(b) => {
			if *b {
				1
			} else {
				0
			}
		}
		_ => 0,
	}
}

pub fn value_to_bool(value: &Value) -> bool {
	match value {
		Value::Bool(b) => *b,
		Value::Number(n) => n.to_int().map(|i| i != 0).unwrap_or(false),
		Value::String(s) => matches!(s.as_str(), "true" | "TRUE" | "1"),
		_ => false,
	}
}
