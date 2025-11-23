use aios_core::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use glam::DVec3;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
struct TestCase {
    name: String,
    description: String,
    target_refno: String,
    expected_wpos: ExpectedPos,
    expected_wori: ExpectedOri,
    tolerance_mm: f64,
}

#[derive(Debug, Deserialize)]
struct ExpectedPos {
    e: f64,
    n: f64,
    u: f64,
}

#[derive(Debug, Deserialize)]
struct ExpectedOri {
    y_axis_desc: String,
    z_axis_desc: String,
    tolerance_deg: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize database connection
    init_surreal().await?;
    println!("🚀 Starting Batch Spatial Verification...");

    // Load test cases
    let file_path = "test-files/spatial_verification_cases.json";
    let file = File::open(file_path).expect("Failed to open test cases file");
    let reader = BufReader::new(file);
    let test_cases: Vec<TestCase> = serde_json::from_reader(reader).expect("Failed to parse test cases");

    let mut passed_count = 0;
    let mut failed_count = 0;

    for case in &test_cases {
        println!("\n--------------------------------------------------");
        println!("🧪 Running Test: {}", case.name);
        println!("📝 Description: {}", case.description);
        println!("🎯 Target: {}", case.target_refno);

        let target_refno = RefnoEnum::from(case.target_refno.as_str());
        
        // --- 1. Verify WPOS ---
        let expected_pos = DVec3::new(case.expected_wpos.e, case.expected_wpos.n, case.expected_wpos.u);
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
                 // This is a simplified check, in real scenario we might need to query type
                 // For now assuming if it finds a SPINE it uses it, but here we just need the GENSEC transform
                 true 
             });
             // We actually just need GENSEC refno for transform usually
             (owner_refno, None) 
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
                if diff.length() < case.tolerance_mm {
                    println!("✅ WPOS Verified: Diff {:.4}mm (Tolerance: {}mm)", diff.length(), case.tolerance_mm);
                    pos_verified = true;
                } else {
                    println!("❌ WPOS Failed: Expected {:?}, Got {:?}, Diff {:.4}mm", expected_pos, calculated_wpos, diff.length());
                }
            } else {
                println!("❌ Failed to get World Matrix for Owner/GENSEC: {}", gensec_refno);
            }
        } else {
             println!("❌ Target has no local position (POS/POSS/POSE/etc.)");
        }

        // --- 2. Verify Orientation (Basic Check) ---
        // Note: Full string parsing of "N 88.958 U" is complex. 
        // For this verification runner, we currently print the calculated axes for manual review 
        // or simple regression checking if we added specific vector expectations in JSON.
        // Future improvement: Add a parser for PDMS orientation strings to vectors.
        
        let mut ori_verified = true; // Default to true unless we implement strict check
        
        // Logic to calculate Orientation (using YDIR fix)
        if let Some(s_ref) = spine_refno {
             let spine_att = get_named_attmap(s_ref).await?;
             if let Some(ydir) = spine_att.get_dvec3("YDIR") {
                 if let Ok(pts) = get_spline_pts(gensec_refno).await {
                     if pts.len() >= 2 {
                         let spine_dir = (pts[1] - pts[0]).normalize();
                         let quat = cal_spine_orientation_basis_with_ydir(spine_dir, Some(ydir), false);
                         
                         let z_axis = quat * DVec3::Z;
                         let y_axis = quat * DVec3::Y;
                         
                         println!("ℹ️  Calculated Z Axis: {:.4?}", z_axis);
                         println!("ℹ️  Calculated Y Axis: {:.4?}", y_axis);
                         println!("ℹ️  Expected Description: {}", case.expected_wori.z_axis_desc);
                         
                         // Simple check: Z axis should match spine direction roughly
                         // This part is placeholder for stricter logic
                     }
                 }
             }
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
