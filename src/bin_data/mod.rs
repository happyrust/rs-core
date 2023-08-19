use std::collections::{HashMap};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use aios_core::pdms_types::{AttrInfo, AttrVal, DbAttributeType};
use aios_core::tool::db_tool::db1_dehash;

pub mod test_data;


pub fn convert_str_to_bytes(data_str: &str) -> Vec<u8> {
    data_str.trim().split_whitespace().map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}

pub fn get_default_att(t: i32, num: i32) -> (AttrVal, DbAttributeType) {
    match t {
        2 => (AttrVal::DoubleType(0.0), DbAttributeType::DOUBLE),
        // 3 => (AttrVal::WordType("unset".to_string()), DbAttributeType::WORD),
        // 0x3 也可能是WORD
        // 怎样自动判定为WORD呢？
        3 => (AttrVal::IntegerType(0), DbAttributeType::INTEGER),
        4 | 0x8| 0x10 | 0x11 => (AttrVal::ElementType("".to_string()), DbAttributeType::ELEMENT),
        5 => (AttrVal::BoolType(false), DbAttributeType::BOOL),
        6 => {
            if num == 3 {
                (AttrVal::Vec3Type([0.0; 3]), DbAttributeType::Vec3Type)
            } else {
                (AttrVal::DoubleArrayType(vec![]), DbAttributeType::DOUBLEVEC)
            }
        }
        7 => (AttrVal::IntArrayType(vec![]), DbAttributeType::INTVEC),
        // 0xA => STRING, 0XE => TYPEX, 0xF => NAME
        0xA | 0xE | 0xF => (AttrVal::StringType("".to_string()), DbAttributeType::STRING),
        _ => {
            (AttrVal::InvalidType, DbAttributeType::INTEGER)
        }
    }
}


pub fn generate_att_info_json() -> anyhow::Result<()> {
    let mut noun_att_map = HashMap::new();
    for entry in fs::read_dir("bins")? {
        let entry = entry?;
        let path = entry.path();
        dbg!(&path);
        let mut bytes = vec![];
        let mut file = File::open(path)?;
        file.read_to_end(&mut bytes)?;
        // let mut bytes = include_bytes!("bins/att_rest.bin");
        // println!("{:#4X?}", &bytes[0..0x10]);

        let attr_pointer = bytes.as_ptr() as *mut u8 as *mut i32;
        unsafe {
            let type_hash = *attr_pointer.offset(1) as u32;
            let type_name = db1_dehash(type_hash);
            let mut attr_info_map = HashMap::new();
            dbg!(type_name);
            let total_attr_cnt = *attr_pointer.offset(9) as isize;
            let mut k = 0xE;
            // let mut hashmap = BTreeMap::new();
            for _i in 0..total_attr_cnt {
                let hash = *attr_pointer.offset(k);
                let offset = (*attr_pointer.offset(k + 8)) as u32;
                let type_id = *attr_pointer.offset(k + 2);
                let word_cnt = *attr_pointer.offset(k + 3);
                let name = db1_dehash(hash as u32);
                //跳过一部分属性
                if !name.starts_with("UDA") {
                    println!("{:#4X}: {}, offset: {:#4X}", hash, &name, offset);
                    println!("{:#4X}, {:#4X}", type_id, word_cnt);
                    // hashmap.insert(val, att_offset);
                    let (default_val, att_type) = get_default_att(type_id, word_cnt);
                    let att_info = AttrInfo {
                        name,
                        hash,
                        offset,
                        default_val,
                        att_type,
                    };
                    dbg!(&att_info);
                    attr_info_map.insert(hash, att_info);
                }
                k += *attr_pointer.offset(k + 1) as isize;
            }
            noun_att_map.insert(type_hash as i32, attr_info_map);
        }
    }

    let json_str = serde_json::to_string(&noun_att_map)?;
    let mut file = File::create("att_append.json")?;
    file.write(json_str.as_bytes())?;

    Ok(())
}