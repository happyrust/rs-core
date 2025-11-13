pub mod record_id_ext;
pub mod surreal_response;
pub mod svg_generator;
pub mod value_ext;
pub mod lod_path_detector;

pub use record_id_ext::{IntoRecordId, RecordIdExt};
pub use surreal_response::{take_option, take_single, take_vec};
pub use value_ext::{value_to_bool, value_to_f32, value_to_i32, value_to_string};
pub use lod_path_detector::build_mesh_path;
