//! snarkjs integration for Groth16 proof generation
//!
//! Uses snarkjs as an alternative to Barretenberg for generating Groth16 proofs.
//! snarkjs is mature and battle-tested (used by Tornado Cash, etc.)

use std::fs;
use std::path::Path;
use std::process::Command;

/// Generate Groth16 proof using snarkjs
///
/// Prerequisites:
/// - Node.js installed
/// - snarkjs installed globally: npm install -g snarkjs
/// - Powers of Tau file downloaded (one-time setup)
///
/// Returns: (proof_path, public_inputs_path, verifying_key_path)
pub fn generate_groth16_proof_snarkjs(
    witness_path: &Path,
    circuit_path: &Path,
    output_prefix: &Path,
) -> Result<(String, String, String), String> {
    println!("Generating Groth16 proof with snarkjs...");

    // Check if snarkjs is installed
    let snarkjs_check = Command::new("snarkjs")
        .arg("--version")
        .output();

    if snarkjs_check.is_err() {
        return Err(
            "snarkjs not found. Install with: npm install -g snarkjs\n\
             See: https://github.com/iden3/snarkjs"
                .to_string(),
        );
    }

    let output_dir = output_prefix.parent().unwrap_or(Path::new("."));
    let base_name = output_prefix.file_stem().unwrap().to_str().unwrap();

    // File paths
    let acir_path = circuit_path.join("target/circuits.json");
    let witness_gz_path = circuit_path.join("target/circuits.gz");
    let r1cs_path = output_dir.join(format!("{}.r1cs", base_name));
    let zkey_path = output_dir.join(format!("{}.zkey", base_name));
    let vkey_path = output_dir.join(format!("{}.vkey.json", base_name));
    let proof_path = output_dir.join(format!("{}.proof.json", base_name));
    let public_path = output_dir.join(format!("{}.public.json", base_name));
    let proof_bin_path = output_dir.join(format!("{}.groth16.proof", base_name));
    let public_bin_path = output_dir.join(format!("{}.public_inputs", base_name));

    // Step 1: Export ACIR to R1CS (if not already done)
    if !r1cs_path.exists() {
        println!("Step 1/5: Exporting Noir circuit to R1CS...");

        // Use nargo to export
        let export_output = Command::new("nargo")
            .current_dir(circuit_path)
            .arg("export")
            .arg("--output")
            .arg(&r1cs_path)
            .output()
            .map_err(|e| format!("Failed to run nargo export: {}", e))?;

        if !export_output.status.success() {
            let stderr = String::from_utf8_lossy(&export_output.stderr);
            return Err(format!("nargo export failed: {}", stderr));
        }

        println!("  ✓ R1CS exported to {}", r1cs_path.display());
    } else {
        println!("Step 1/5: Using existing R1CS file");
    }

    // Step 2: Download Powers of Tau (if needed)
    let ptau_path = output_dir.join("powersOfTau28_hez_final_14.ptau");
    if !ptau_path.exists() {
        println!("Step 2/5: Downloading Powers of Tau (one-time setup)...");
        println!("  This may take a few minutes (200MB download)");

        let download_output = Command::new("curl")
            .arg("-L")
            .arg("https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_14.ptau")
            .arg("-o")
            .arg(&ptau_path)
            .output()
            .map_err(|e| format!("Failed to download Powers of Tau: {}", e))?;

        if !download_output.status.success() {
            return Err("Failed to download Powers of Tau file".to_string());
        }

        println!("  ✓ Powers of Tau downloaded");
    } else {
        println!("Step 2/5: Using existing Powers of Tau file");
    }

    // Step 3: Generate zkey (proving key + verification key)
    if !zkey_path.exists() {
        println!("Step 3/5: Generating proving/verification keys...");

        let setup_output = Command::new("snarkjs")
            .arg("groth16")
            .arg("setup")
            .arg(&r1cs_path)
            .arg(&ptau_path)
            .arg(&zkey_path)
            .output()
            .map_err(|e| format!("Failed to run snarkjs setup: {}", e))?;

        if !setup_output.status.success() {
            let stderr = String::from_utf8_lossy(&setup_output.stderr);
            return Err(format!("snarkjs setup failed: {}", stderr));
        }

        println!("  ✓ Keys generated");

        // Export verification key
        let vkey_output = Command::new("snarkjs")
            .arg("zkey")
            .arg("export")
            .arg("verificationkey")
            .arg(&zkey_path)
            .arg(&vkey_path)
            .output()
            .map_err(|e| format!("Failed to export verification key: {}", e))?;

        if !vkey_output.status.success() {
            let stderr = String::from_utf8_lossy(&vkey_output.stderr);
            return Err(format!("Failed to export vkey: {}", stderr));
        }

        println!("  ✓ Verification key exported");
    } else {
        println!("Step 3/5: Using existing proving/verification keys");
    }

    // Step 4: Convert witness to JSON format for snarkjs
    println!("Step 4/5: Converting witness format...");

    // Read the gzipped witness and decompress
    let witness_data = fs::read(&witness_gz_path)
        .map_err(|e| format!("Failed to read witness: {}", e))?;

    // For now, snarkjs expects witness in different format
    // We'll need to convert from Noir's format to snarkjs format
    // This is a placeholder - actual implementation would parse the witness
    let witness_json_path = output_dir.join(format!("{}.witness.json", base_name));

    // TODO: Convert Noir witness to snarkjs witness format
    // For now, we'll document this limitation
    println!("  ⚠ Warning: Witness format conversion not yet implemented");
    println!("  You may need to use Barretenberg or implement witness conversion");

    // Step 5: Generate proof
    println!("Step 5/5: Generating Groth16 proof...");

    let prove_output = Command::new("snarkjs")
        .arg("groth16")
        .arg("prove")
        .arg(&zkey_path)
        .arg(&witness_json_path)
        .arg(&proof_path)
        .arg(&public_path)
        .output()
        .map_err(|e| format!("Failed to run snarkjs prove: {}", e))?;

    if !prove_output.status.success() {
        let stderr = String::from_utf8_lossy(&prove_output.stderr);
        return Err(format!("snarkjs prove failed: {}", stderr));
    }

    println!("  ✓ Proof generated");

    // Convert JSON proof to binary format for Solana
    convert_proof_to_binary(&proof_path, &proof_bin_path)?;
    convert_public_inputs_to_binary(&public_path, &public_bin_path)?;

    println!("\n✓ Groth16 proof generation complete!");
    println!("  Proof: {}", proof_bin_path.display());
    println!("  Public inputs: {}", public_bin_path.display());
    println!("  Verification key: {}", vkey_path.display());

    Ok((
        proof_bin_path.to_str().unwrap().to_string(),
        public_bin_path.to_str().unwrap().to_string(),
        vkey_path.to_str().unwrap().to_string(),
    ))
}

/// Convert JSON proof to binary format
fn convert_proof_to_binary(json_path: &Path, bin_path: &Path) -> Result<(), String> {
    // Read JSON proof
    let json_str = fs::read_to_string(json_path)
        .map_err(|e| format!("Failed to read proof JSON: {}", e))?;

    let proof: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse proof JSON: {}", e))?;

    // Extract proof components (pi_a, pi_b, pi_c)
    // Convert to binary format expected by Solana

    // For now, write the JSON as-is
    // TODO: Implement proper binary conversion
    fs::write(bin_path, json_str.as_bytes())
        .map_err(|e| format!("Failed to write binary proof: {}", e))?;

    Ok(())
}

/// Convert JSON public inputs to binary format
fn convert_public_inputs_to_binary(json_path: &Path, bin_path: &Path) -> Result<(), String> {
    // Read JSON public inputs
    let json_str = fs::read_to_string(json_path)
        .map_err(|e| format!("Failed to read public inputs JSON: {}", e))?;

    // For now, write the JSON as-is
    // TODO: Implement proper binary conversion
    fs::write(bin_path, json_str.as_bytes())
        .map_err(|e| format!("Failed to write binary public inputs: {}", e))?;

    Ok(())
}

/// Generate Solidity verifier contract using snarkjs
pub fn generate_solidity_verifier_snarkjs(
    vkey_path: &Path,
    output_path: &Path,
) -> Result<(), String> {
    println!("Generating Solidity verifier contract...");

    let output = Command::new("snarkjs")
        .arg("zkey")
        .arg("export")
        .arg("solidityverifier")
        .arg(vkey_path)
        .arg(output_path)
        .output()
        .map_err(|e| format!("Failed to generate verifier: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Verifier generation failed: {}", stderr));
    }

    println!("✓ Verifier contract generated: {}", output_path.display());

    Ok(())
}
