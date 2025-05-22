use std::io::Read;
use std::path::PathBuf;
use crate::{NamedAttrMap, RefU64, SurlValue, SUL_DB};
use cached::proc_macro::cached;

pub async fn define_common_functions() -> anyhow::Result<()> {
    let target_dir = std::fs::read_dir("resource/surreal")?.into_iter()
        .map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).collect::<Vec<PathBuf>>();
    for file in target_dir {
        println!("载入surreal {}",file.file_name().unwrap().to_str().unwrap().to_string());
        let mut file = std::fs::File::open(file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        SUL_DB.query(content).await?;
    }
    Ok(())
}
