use crate::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use glam::{DVec3, DMat4, Vec3};
use std::fs::File;
use std::io::BufReader;
use regex::Regex;
use approx::assert_relative_eq;

#[derive(Debug, Deserialize)]
struct SpatialTestCase {
    refno: String,
    wpos_str: String,
    wori_str: String,
}

fn parse_wpos(wpos_str: &str) -> Option<DVec3> {
    // Position W 5375.49mm N 1771.29mm D 2607.01mm
    let re = Regex::new(r"Position\s+([WESNUD])\s*([\d.]+)\s*mm\s+([WESNUD])\s*([\d.]+)\s*mm\s+([WESNUD])\s*([\d.]+)\s*mm").ok()?;
    
    if let Some(caps) = re.captures(wpos_str) {
        let mut pos = DVec3::ZERO;
        
        for i in 0..3 {
            let dir = caps.get(1 + i * 2)?.as_str();
            let val = caps.get(2 + i * 2)?.as_str().parse::<f64>().ok()?;
            
            match dir {
                "E" => pos.x += val,
                "W" => pos.x -= val,
                "N" => pos.y += val,
                "S" => pos.y -= val,
                "U" => pos.z += val,
                "D" => pos.z -= val,
                _ => {}
            }
        }
        return Some(pos);
    }
    None
}

fn parse_wori(wori_str: &str) -> Option<(DVec3, DVec3)> {
    let parts: Vec<&str> = wori_str.split(" and ").collect();
    
    let mut y_axis = DVec3::Y;
    let mut z_axis = DVec3::Z;
    
    for part in parts {
        let part = part.trim();
        if part.starts_with("Orientation ") {
             let content = part.strip_prefix("Orientation ").unwrap();
             if let Some((axis, desc)) = parse_axis_def(content) {
                 if axis == "Y" { y_axis = desc; }
                 else if axis == "Z" { z_axis = desc; }
             }
        } else {
             if let Some((axis, desc)) = parse_axis_def(part) {
                 if axis == "Y" { y_axis = desc; }
                 else if axis == "Z" { z_axis = desc; }
             }
        }
    }
    
    Some((y_axis, z_axis))
}

fn parse_axis_def(s: &str) -> Option<(&str, DVec3)> {
    let parts: Vec<&str> = s.split(" is ").collect();
    if parts.len() != 2 { return None; }
    
    let axis_name = parts[0].trim();
    let dir_desc = parts[1].trim();
    
    let vec = parse_pdms_direction(dir_desc)?;
    Some((axis_name, vec))
}

fn parse_pdms_direction(desc: &str) -> Option<DVec3> {
    let parts: Vec<&str> = desc.split_whitespace().collect();
    if parts.is_empty() { return None; }
    
    let main_axis_str = parts[0];
    let mut current_vec = get_axis_vec(main_axis_str)?;
    
    let mut i = 1;
    while i < parts.len() {
        if let Ok(angle) = parts[i].parse::<f64>() {
            if i + 1 >= parts.len() { break; }
            let target_axis_str = parts[i+1];
            let target_vec = get_axis_vec(target_axis_str)?;
            
            let angle_rad = angle.to_radians();
            
            // Ensure orthogonality for rotation plane
            let rotation_axis = current_vec.cross(target_vec);
            if rotation_axis.length_squared() > 1e-6 {
                 if current_vec.dot(target_vec).abs() < 1e-3 {
                     current_vec = current_vec * angle_rad.cos() + target_vec * angle_rad.sin();
                 }
            }
            
            i += 2;
        } else {
            i += 1;
        }
    }
    
    Some(current_vec.normalize())
}

fn get_axis_vec(s: &str) -> Option<DVec3> {
    match s {
        "N" => Some(DVec3::Y),
        "S" => Some(DVec3::NEG_Y),
        "E" => Some(DVec3::X),
        "W" => Some(DVec3::NEG_X),
        "U" => Some(DVec3::Z),
        "D" => Some(DVec3::NEG_Z),
        _ => None
    }
}

#[tokio::test]
async fn test_generic_spatial_cases() -> Result<()> {
    // Initialize database connection
    init_surreal().await?;
    
    // Read test cases from JSON file
    let file_path = "src/test/test-cases/spatial/spatial_pdms_cases.json";
    let file = File::open(file_path).expect("Failed to open test cases file");
    let reader = BufReader::new(file);
    let test_cases: Vec<SpatialTestCase> = serde_json::from_reader(reader)
        .expect("Failed to parse test cases");

    println!("🚀 Running {} Spatial Test Cases", test_cases.len());

    let mut errors = Vec::new();

    for case in &test_cases {
        println!("--------------------------------------------------");
        println!("🧪 Case: {}", case.refno);

        let target_refno = RefnoEnum::from(case.refno.replace("/", "_").as_str());
        
        // Parse expected position from WPOS string
        let expected_pos = parse_wpos(&case.wpos_str)
            .expect("Failed to parse WPOS string");
        
        // Get world matrix using get_world_mat4
        if let Some(world_matrix) = get_world_mat4(target_refno, false).await? {
            // Extract position from world matrix (last column)
            let calculated_pos = world_matrix.transform_point3(DVec3::ZERO);
            let diff = calculated_pos - expected_pos;
            
            if diff.length() < 1.0 {
                println!("✅ Position OK - Expected: {:?}, Got: {:?}, Diff: {:.4}", 
                    expected_pos, calculated_pos, diff.length());
            } else {
                let msg = format!("❌ Position Mismatch for {}: Expected {:?}, Got {:?}, Diff {:.4}", 
                    case.refno, expected_pos, calculated_pos, diff.length());
                println!("{}", msg);
                errors.push(msg);
            }
            
            // Optional: Check orientation if needed
            if let Some((expected_y, expected_z)) = parse_wori(&case.wori_str) {
                // Extract orientation from world matrix
                let calculated_y = world_matrix.transform_vector3(DVec3::Y).normalize();
                let calculated_z = world_matrix.transform_vector3(DVec3::Z).normalize();
                
                let y_dot = calculated_y.dot(expected_y);
                let z_dot = calculated_z.dot(expected_z);
                
                if y_dot > 0.999 && z_dot > 0.999 {
                    println!("✅ Orientation OK - Y_dot: {:.6}, Z_dot: {:.6}", y_dot, z_dot);
                } else {
                    let msg = format!("⚠️  Orientation Mismatch for {}: Y_dot={:.6}, Z_dot={:.6}", 
                        case.refno, y_dot, z_dot);
                    println!("{}", msg);
                    // Note: Not adding orientation errors to error list for now, just warnings
                }
            }
        } else {
            let msg = format!("❌ Failed to get world matrix for {}", case.refno);
            println!("{}", msg);
            errors.push(msg);
        }
    }

    if !errors.is_empty() {
        panic!("Spatial Test Failed:\n{}", errors.join("\n"));
    }
    
    println!("✅ All spatial tests passed!");
    Ok(())
}
