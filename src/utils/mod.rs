pub mod record_id_ext;
pub mod value_ext;
pub mod surreal_response;
pub mod svg_generator;

pub use record_id_ext::{IntoRecordId, RecordIdExt};
pub use value_ext::{value_to_bool, value_to_f32, value_to_i32, value_to_string};
pub use surreal_response::{take_option, take_single, take_vec};
