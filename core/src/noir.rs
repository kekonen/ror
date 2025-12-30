// Noir circuit integration module
// Provides functionality to execute nargo CLI and parse circuit outputs

#[cfg(feature = "std")]
use std::process::Command;

#[cfg(feature = "std")]
use alloc::vec::Vec;

/// Outputs from Noir circuit execution
#[derive(Debug, Clone)]
pub struct NoirCircuitOutputs {
    pub walks: u64,
    pub steps: u64,
    pub binary_image: [u8; 256],
}

/// Parse nargo execute output to extract circuit return values
///
/// Example nargo output:
/// ```
/// [circuits] Circuit output: (13, 92, [0, 0, 0, ...])
/// ```
#[cfg(feature = "std")]
pub fn parse_nargo_output(output: &str) -> Result<NoirCircuitOutputs, String> {
    // Find the line with "Circuit output:"
    let output_line = output
        .lines()
        .find(|line| line.contains("Circuit output:"))
        .ok_or("No circuit output found in nargo output")?;

    // Extract the tuple part after "Circuit output: "
    let tuple_start = output_line
        .find("Circuit output: ")
        .ok_or("Malformed output line")?
        + "Circuit output: ".len();

    let tuple_str = &output_line[tuple_start..];

    // Parse the tuple: (walks, steps, [array...])
    // Simple parser: expecting format "(u64, u64, [u8; 256])"

    if !tuple_str.starts_with('(') {
        return Err("Output doesn't start with '('".to_string());
    }

    // Find first comma (after walks)
    let first_comma = tuple_str
        .find(',')
        .ok_or("Missing first comma in output")?;

    let walks_str = tuple_str[1..first_comma].trim();
    let walks = walks_str
        .parse::<u64>()
        .map_err(|e| format!("Failed to parse walks: {}", e))?;

    // Find second comma (after steps)
    let remaining = &tuple_str[first_comma + 1..];
    let second_comma = remaining
        .find(',')
        .ok_or("Missing second comma in output")?;

    let steps_str = remaining[..second_comma].trim();
    let steps = steps_str
        .parse::<u64>()
        .map_err(|e| format!("Failed to parse steps: {}", e))?;

    // Parse the array
    let array_start_idx = first_comma + 1 + second_comma + 1;
    let array_part = &tuple_str[array_start_idx..];

    let array_content = array_part
        .trim()
        .strip_prefix('[')
        .ok_or("Array doesn't start with '['")?
        .strip_suffix(')')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or("Array doesn't end with '])'")? ;

    // Split by commas and parse each number
    let numbers: Result<Vec<u8>, _> = array_content
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<u8>())
        .collect();

    let binary_vec = numbers.map_err(|e| format!("Failed to parse array element: {}", e))?;

    if binary_vec.len() != 256 {
        return Err(format!("Expected 256 bytes, got {}", binary_vec.len()));
    }

    let mut binary_image = [0u8; 256];
    binary_image.copy_from_slice(&binary_vec);

    Ok(NoirCircuitOutputs {
        walks,
        steps,
        binary_image,
    })
}

/// Execute nargo circuit with given private key and return outputs
///
/// This writes a minimal Prover.toml, calls `nargo execute`, and parses the output.
#[cfg(feature = "std")]
pub fn execute_noir_circuit(
    private_key: &[u8; 32],
    circuits_dir: &str,
) -> Result<NoirCircuitOutputs, String> {
    use std::fs;
    use std::path::Path;

    // Write minimal Prover.toml
    let prover_toml = generate_prover_toml(private_key);
    let prover_path = Path::new(circuits_dir).join("Prover.toml");

    fs::write(&prover_path, prover_toml)
        .map_err(|e| format!("Failed to write Prover.toml: {}", e))?;

    // Execute nargo
    let output = Command::new("nargo")
        .arg("execute")
        .current_dir(circuits_dir)
        .output()
        .map_err(|e| format!("Failed to run nargo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("nargo execute failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_nargo_output(&stdout)
}

/// Generate Prover.toml content for a given private key
fn generate_prover_toml(private_key: &[u8; 32]) -> String {
    let mut toml = String::from("# Auto-generated Prover.toml\n");
    toml.push_str("private_key = [\n");

    for (i, byte) in private_key.iter().enumerate() {
        if i % 8 == 0 {
            toml.push_str("    ");
        }
        toml.push_str(&format!("\"0x{:02x}\"", byte));
        if i < 31 {
            toml.push_str(", ");
        }
        if i % 8 == 7 {
            toml.push('\n');
        }
    }

    toml.push_str("]\n");
    toml
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nargo_output() {
        // Create test output with exactly 256 zeros
        let mut array_str = String::from("[circuits] Circuit output: (13, 92, [");
        for i in 0..256 {
            array_str.push_str(&i.to_string());
            if i < 255 {
                array_str.push_str(", ");
            }
        }
        array_str.push_str("])");

        let result = parse_nargo_output(&array_str).unwrap();
        assert_eq!(result.walks, 13);
        assert_eq!(result.steps, 92);
        assert_eq!(result.binary_image[0], 0);
        assert_eq!(result.binary_image[1], 1);
        assert_eq!(result.binary_image[10], 10);
        assert_eq!(result.binary_image[255], 255);
    }

    #[test]
    fn test_generate_prover_toml() {
        let pk = [0x12u8; 32];
        let toml = generate_prover_toml(&pk);

        assert!(toml.contains("private_key"));
        assert!(toml.contains("0x12"));
        assert_eq!(toml.matches("0x12").count(), 32);
    }
}
