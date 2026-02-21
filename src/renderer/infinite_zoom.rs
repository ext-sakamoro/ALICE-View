//! Infinite zoom LOD calculation
//!
//! Since procedural content is generated from mathematical descriptions,
//! we can zoom indefinitely without quality loss. This module handles
//! LOD (Level of Detail) calculations for infinite zoom.

// LodLevel, Precision, InfiniteZoomManager and helper functions are a complete
// LOD subsystem that will be wired into the shader pipeline in a future pass.
#![allow(dead_code)]

/// LOD level based on zoom factor
#[derive(Debug, Clone, Copy)]
pub struct LodLevel {
    /// Current zoom factor
    pub zoom: f32,
    /// Number of detail iterations to compute
    pub iterations: u32,
    /// Precision level for calculations
    pub precision: Precision,
}

/// Calculation precision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// Standard f32 precision
    Standard,
    /// Double precision f64
    Double,
    /// Arbitrary precision (for extreme zoom)
    Arbitrary,
}

impl LodLevel {
    /// Calculate LOD level from zoom factor
    pub fn from_zoom(zoom: f32) -> Self {
        let iterations = calculate_iterations(zoom);
        let precision = calculate_precision(zoom);

        Self {
            zoom,
            iterations,
            precision,
        }
    }

    /// Get recommended texture resolution for current LOD
    pub fn recommended_resolution(&self, base_resolution: u32) -> u32 {
        // Scale resolution with zoom, capped at reasonable limits
        let scaled = (base_resolution as f32 * self.zoom.sqrt()) as u32;
        scaled.clamp(64, 8192)
    }
}

/// Calculate number of iterations based on zoom level
fn calculate_iterations(zoom: f32) -> u32 {
    // More iterations = more detail
    // Logarithmic scaling: zoom 1x = 4 iterations, zoom 1000x = 14 iterations
    let base = 4;
    let extra = (zoom.log10().max(0.0) * 3.0) as u32;
    (base + extra).min(20)
}

/// Calculate required precision based on zoom level
fn calculate_precision(zoom: f32) -> Precision {
    if zoom > 1_000_000.0 {
        Precision::Arbitrary
    } else if zoom > 10_000.0 {
        Precision::Double
    } else {
        Precision::Standard
    }
}

/// Procedural LOD manager
pub struct InfiniteZoomManager {
    current_lod: LodLevel,
    /// Cached calculations for performance
    cached_iterations: Option<u32>,
}

impl InfiniteZoomManager {
    pub fn new() -> Self {
        Self {
            current_lod: LodLevel::from_zoom(1.0),
            cached_iterations: None,
        }
    }

    /// Update LOD based on new zoom level
    pub fn update(&mut self, zoom: f32) -> bool {
        let new_lod = LodLevel::from_zoom(zoom);

        // Check if LOD changed significantly
        let changed = new_lod.iterations != self.current_lod.iterations
            || new_lod.precision != self.current_lod.precision;

        if changed {
            self.current_lod = new_lod;
            self.cached_iterations = None;
        }

        changed
    }

    /// Get current LOD level
    pub fn current(&self) -> &LodLevel {
        &self.current_lod
    }

    /// Get shader parameters for current LOD
    pub fn shader_params(&self) -> LodShaderParams {
        LodShaderParams {
            iterations: self.current_lod.iterations,
            zoom: self.current_lod.zoom,
            use_double_precision: if self.current_lod.precision != Precision::Standard { 1 } else { 0 },
            _padding: 0,
        }
    }
}

impl Default for InfiniteZoomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters to pass to shaders for LOD rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LodShaderParams {
    pub iterations: u32,
    pub zoom: f32,
    pub use_double_precision: u32,
    _padding: u32,
}

impl Default for LodShaderParams {
    fn default() -> Self {
        Self {
            iterations: 4,
            zoom: 1.0,
            use_double_precision: 0,
            _padding: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_levels() {
        let lod_1x = LodLevel::from_zoom(1.0);
        assert_eq!(lod_1x.iterations, 4);
        assert_eq!(lod_1x.precision, Precision::Standard);

        let lod_1000x = LodLevel::from_zoom(1000.0);
        assert!(lod_1000x.iterations > lod_1x.iterations);

        let lod_extreme = LodLevel::from_zoom(10_000_000.0);
        assert_eq!(lod_extreme.precision, Precision::Arbitrary);
    }
}
