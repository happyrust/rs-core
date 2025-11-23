use aios_core::*;
use anyhow::Result;
use glam::DVec3;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
struct PdmsTestCase {
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

fn parse_pdms_direction(desc: &str) -> Option<DVec3> {
    // Format 1: "N 88.958 U" -> Start N, rotate 88.958 towards U
    // Format 2: "N 0.0451 W 1.0416 D" -> Start N, rotate 0.0451 W, then 1.0416 D

    // Simplified parser logic:
    // 1. Identify Main Axis (N, S, E, W, U, D)
    // 2. Parse sequence of (Angle, Axis)

    let parts: Vec<&str> = desc.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let main_axis_str = parts[0];
    let mut current_vec = get_axis_vec(main_axis_str)?;

    let mut i = 1;
    while i < parts.len() {
        if let Ok(angle) = parts[i].parse::<f64>() {
            if i + 1 >= parts.len() {
                break;
            }
            let target_axis_str = parts[i + 1];
            let target_vec = get_axis_vec(target_axis_str)?;

            // Rotate current_vec towards target_vec by angle degrees
            // The rotation is usually around the cross product axis
            // Or simplified: mix the vectors
            // For small angles/orthogonal axes:
            // V_new = cos(a)*V_old + sin(a)*V_target (if V_old, V_target orthogonal)

            // PDMS logic: "N x W" -> Rotate N around U axis? or simply rotate in N-W plane?
            // "N x W" implies rotation in horizontal plane.
            // "N x D" implies rotation in vertical plane.

            let angle_rad = angle.to_radians();

            // Ensure orthogonality for rotation plane
            let rotation_axis = current_vec.cross(target_vec);
            if rotation_axis.length_squared() > 1e-6 {
                // Rotate current_vec towards target_vec
                // Actually, simple linear combination for orthogonal start/target
                // V = cos(a) * Start + sin(a) * Target
                // But we must check if Start/Target are orthogonal
                if current_vec.dot(target_vec).abs() < 1e-3 {
                    current_vec = current_vec * angle_rad.cos() + target_vec * angle_rad.sin();
                } else {
                    // If not orthogonal, it's more complex, but PDMS usually gives orthogonal reference
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
        _ => None,
    }
}

fn parse_wori(wori_str: &str) -> Option<(DVec3, DVec3)> {
    // "Orientation Y is N 88.958 U and Z is N 0.0451 W 1.0416 D"
    // Split by "and"
    let parts: Vec<&str> = wori_str.split(" and ").collect();

    let mut y_axis = DVec3::Y;
    let mut z_axis = DVec3::Z;

    for part in parts {
        let part = part.trim();
        if part.starts_with("Orientation ") {
            // Handle "Orientation Y is ..."
            let content = part.strip_prefix("Orientation ").unwrap();
            if let Some((axis, desc)) = parse_axis_def(content) {
                if axis == "Y" {
                    y_axis = desc;
                } else if axis == "Z" {
                    z_axis = desc;
                }
            }
        } else {
            // Handle "Z is ..."
            if let Some((axis, desc)) = parse_axis_def(part) {
                if axis == "Y" {
                    y_axis = desc;
                } else if axis == "Z" {
                    z_axis = desc;
                }
            }
        }
    }

    Some((y_axis, z_axis))
}

fn parse_axis_def(s: &str) -> Option<(&str, DVec3)> {
    // "Y is N 88.958 U"
    let parts: Vec<&str> = s.split(" is ").collect();
    if parts.len() != 2 {
        return None;
    }

    let axis_name = parts[0].trim();
    let dir_desc = parts[1].trim();

    let vec = parse_pdms_direction(dir_desc)?;
    Some((axis_name, vec))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize database connection
    init_surreal().await?;
    println!("🚀 Starting Batch Spatial Verification (PDMS Format)...");

    // Load test cases
    let file_path = "test-files/spatial_pdms_cases.json";
    let file = File::open(file_path).expect("Failed to open test cases file");
    let reader = BufReader::new(file);
    let test_cases: Vec<PdmsTestCase> =
        serde_json::from_reader(reader).expect("Failed to parse test cases");

    let mut passed_count = 0;
    let mut failed_count = 0;

    for case in &test_cases {
        println!("\n--------------------------------------------------");
        println!("🧪 Running Test: {}", case.refno);
        println!("📝 Expected WPOS: {}", case.wpos_str);
        println!("📝 Expected WORI: {}", case.wori_str);

        let target_refno = RefnoEnum::from(case.refno.replace("/", "_").as_str());

        // --- 1. Verify WPOS ---
        let expected_pos = parse_wpos(&case.wpos_str).expect("Failed to parse WPOS string");
        let mut calculated_wpos = DVec3::ZERO;
        let mut pos_verified = false;

        // Logic to calculate WPOS (using the fix from analysis)
        let att = get_named_attmap(target_refno).await?;
        let owner_refno = att.get_owner();
        let owner_att = get_named_attmap(owner_refno).await?;
        let owner_type = owner_att.get_type_str();

        let (gensec_refno, spine_refno) = if owner_type == "SPINE" {
            (owner_att.get_owner(), Some(owner_refno))
        } else if owner_type == "GENSEC" || owner_type == "WALL" {
            let children = get_children_refnos(owner_refno).await?;
            let s_ref = children.into_iter().find(|&r| {
                true // Simplified check
            });
            (owner_refno, None) // Usually GENSEC transform is enough for POS
        } else {
            (owner_refno, None)
        };

        // Try retrieving local position
        if let Some(local_pos) = att.get_position() {
            let local_pos_d = local_pos.as_dvec3();

            // Get GENSEC world matrix
            if let Some(gensec_mat) = get_world_mat4(gensec_refno, false).await? {
                calculated_wpos = gensec_mat.transform_point3(local_pos_d);

                let diff = calculated_wpos - expected_pos;
                if diff.length() < 1.0 {
                    // 1mm tolerance
                    println!("✅ WPOS Verified: Diff {:.4}mm", diff.length());
                    pos_verified = true;
                } else {
                    println!(
                        "❌ WPOS Failed: Expected {:?}, Got {:?}, Diff {:.4}mm",
                        expected_pos,
                        calculated_wpos,
                        diff.length()
                    );
                }
            } else {
                println!(
                    "❌ Failed to get World Matrix for Owner/GENSEC: {}",
                    gensec_refno
                );
            }
        } else {
            println!("❌ Target has no local position (POS/POSS/POSE/etc.)");
        }

        // --- 2. Verify Orientation ---
        let mut ori_verified = false;

        if let Some(s_ref) = spine_refno {
            let spine_att = get_named_attmap(s_ref).await?;
            if let Some(ydir) = spine_att.get_dvec3("YDIR") {
                if let Ok(pts) = get_spline_pts(gensec_refno).await {
                    if pts.len() >= 2 {
                        let spine_dir = (pts[1] - pts[0]).normalize();
                        let quat =
                            cal_spine_orientation_basis_with_ydir(spine_dir, Some(ydir), false);

                        let z_axis = quat * DVec3::Z;
                        let y_axis = quat * DVec3::Y;

                        // Parse expected orientation
                        if let Some((expected_y, expected_z)) = parse_wori(&case.wori_str) {
                            let y_dot = y_axis.dot(expected_y);
                            let z_dot = z_axis.dot(expected_z);

                            // Dot product should be close to 1.0
                            if y_dot > 0.9999 && z_dot > 0.9999 {
                                println!("✅ WORI Verified:");
                                println!(
                                    "   Y Axis Match: {:.6} (Expected {:?} vs Got {:?})",
                                    y_dot, expected_y, y_axis
                                );
                                println!(
                                    "   Z Axis Match: {:.6} (Expected {:?} vs Got {:?})",
                                    z_dot, expected_z, z_axis
                                );
                                ori_verified = true;
                            } else {
                                println!("❌ WORI Failed:");
                                println!(
                                    "   Y Axis Dot: {:.6} (Expected {:?} vs Got {:?})",
                                    y_dot, expected_y, y_axis
                                );
                                println!(
                                    "   Z Axis Dot: {:.6} (Expected {:?} vs Got {:?})",
                                    z_dot, expected_z, z_axis
                                );
                            }
                        } else {
                            println!("❌ Failed to parse WORI string: {}", case.wori_str);
                        }
                    }
                }
            }
        } else {
            println!("⚠️ Skipping Orientation check (No SPINE found)");
            // If no spine, we can't verify orientation with this logic, but maybe shouldn't fail the whole test?
            // For this specific test case, we expect it to pass.
        }

        if pos_verified && ori_verified {
            passed_count += 1;
        } else {
            failed_count += 1;
        }
    }

    println!("\n==================================================");
    println!("📊 Test Summary");
    println!("Total: {}", test_cases.len());
    println!("Passed: {}", passed_count);
    println!("Failed: {}", failed_count);

    if failed_count > 0 {
        Err(anyhow::anyhow!("Some tests failed"))
    } else {
        Ok(())
    }
}
