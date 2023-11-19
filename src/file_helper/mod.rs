use std::fs;
use std::path::{Path, PathBuf};

///获得目录下下面的指定project 数据库文件（000结尾）
#[inline]
pub fn collect_db_dirs<'a, T: AsRef<Path>>(dir: T, projects: impl IntoIterator<Item=&'a str>) -> Vec<PathBuf> {
    projects.into_iter().filter_map(|x| {
        fs::read_dir(dir.as_ref().to_path_buf().join(x))
            .unwrap()
            .into_iter()
            .map(|entry| entry.unwrap().path())
            .find(|x| x.is_dir() && x.file_name().unwrap().to_str().unwrap().ends_with("000"))
    }).collect()
}