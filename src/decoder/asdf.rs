//! ALICE-SDF file decoder
//!
//! Loads .asdf binary and .asdf.json files for 3D SDF visualization.
//! Includes WGSL transpilation for GPU raymarching.
//! Author: Moroya Sakamoto

use alice_sdf::prelude::*;
use alice_sdf::compiled::WgslShader;
use anyhow::{Context, Result};
use std::path::Path;

/// Loaded SDF content for 3D visualization
#[derive(Debug, Clone)]
pub struct SdfContent {
    /// The SDF tree
    pub tree: SdfTree,
    /// Node count
    pub node_count: usize,
    /// Bounding box (computed from SDF)
    pub bounds: (Vec3, Vec3),
    /// File format version
    pub version: String,
}

impl SdfContent {
    /// Load from .asdf or .asdf.json file
    pub fn load(path: &Path) -> Result<Self> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let tree = if path.to_string_lossy().ends_with(".asdf.json") {
            // JSON format
            tracing::info!("Loading ASDF JSON: {:?}", path);
            alice_sdf::load(path).context("Failed to load ASDF JSON")?
        } else if extension == "asdf" {
            // Binary format
            tracing::info!("Loading ASDF binary: {:?}", path);
            alice_sdf::load(path).context("Failed to load ASDF binary")?
        } else {
            anyhow::bail!("Unknown SDF format: {}", extension);
        };

        let node_count = tree.node_count() as usize;

        // Compute approximate bounds by sampling
        let bounds = Self::compute_bounds(&tree.root);

        // Get version from file info if available
        let info: Option<String> = get_info(path).ok();
        let version = info
            .and_then(|i: String| i.lines().next().map(|s| s.to_string()))
            .unwrap_or_else(|| "0.1.0".to_string());

        Ok(Self {
            tree,
            node_count,
            bounds,
            version,
        })
    }

    /// Compute approximate bounding box by sampling
    fn compute_bounds(node: &SdfNode) -> (Vec3, Vec3) {
        // Start with default bounds
        let mut min = Vec3::splat(-2.0);
        let mut max = Vec3::splat(2.0);

        // Sample to find actual bounds
        let resolution = 32;
        let step = 4.0 / resolution as f32;

        for iz in 0..resolution {
            for iy in 0..resolution {
                for ix in 0..resolution {
                    let p = Vec3::new(
                        -2.0 + ix as f32 * step,
                        -2.0 + iy as f32 * step,
                        -2.0 + iz as f32 * step,
                    );

                    // Use alice_sdf::eval function
                    let d = eval(node, p);
                    if d < 0.0 {
                        // Inside the surface, expand bounds
                        min = min.min(p - Vec3::splat(0.1));
                        max = max.max(p + Vec3::splat(0.1));
                    }
                }
            }
        }

        (min, max)
    }

    /// Get the SDF root node
    pub fn root(&self) -> &SdfNode {
        &self.tree.root
    }

    /// Generate WGSL shader code for this SDF
    ///
    /// Uses ALICE-SDF's WgslShader transpiler to convert the SDF tree
    /// to optimized WGSL code for GPU evaluation.
    pub fn to_wgsl(&self) -> String {
        let shader = WgslShader::transpile(&self.tree.root);
        tracing::info!(
            "Transpiled SDF to WGSL: {} nodes → {} bytes, {} helpers",
            self.node_count,
            shader.source.len(),
            shader.helper_count
        );
        shader.source
    }

    /// Get the raw WGSL source with metadata
    pub fn to_wgsl_with_metadata(&self) -> (String, usize, usize) {
        let shader = WgslShader::transpile(&self.tree.root);
        (shader.source, self.node_count, shader.helper_count)
    }
}

/// Check if a file is an ASDF file
pub fn is_asdf_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.ends_with(".asdf") || path_str.ends_with(".asdf.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_asdf_file() {
        assert!(is_asdf_file(Path::new("model.asdf")));
        assert!(is_asdf_file(Path::new("model.asdf.json")));
        assert!(!is_asdf_file(Path::new("model.obj")));
        assert!(!is_asdf_file(Path::new("model.alice")));
    }
}
