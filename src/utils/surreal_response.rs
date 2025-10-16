use anyhow::Context;
use serde::de::DeserializeOwned;
use serde_json;
use surrealdb::IndexedResults;

use crate::SurlValue;

fn decode_value<T: DeserializeOwned>(value: SurlValue) -> anyhow::Result<T> {
    let json = value.into_json_value();
    serde_json::from_value(json).context("无法将 SurrealDB 值反序列化为目标类型")
}

pub fn take_vec<T: DeserializeOwned>(
    response: &mut IndexedResults,
    index: usize,
) -> anyhow::Result<Vec<T>> {
    let values: Vec<SurlValue> = response.take(index)?;
    values.into_iter().map(decode_value).collect()
}

pub fn take_option<T: DeserializeOwned>(
    response: &mut IndexedResults,
    index: usize,
) -> anyhow::Result<Option<T>> {
    let value: Option<SurlValue> = response.take(index)?;
    value.map(decode_value).transpose()
}

pub fn take_single<T: DeserializeOwned>(
    response: &mut IndexedResults,
    index: usize,
) -> anyhow::Result<T> {
    let value: SurlValue = response.take(index)?;
    decode_value(value)
}
