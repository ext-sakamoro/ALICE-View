//! Export functionality for SDF models
//!
//! Supports GLB and OBJ export via ALICE-SDF's Marching Cubes mesher.
//! Author: Moroya Sakamoto

use crate::decoder::asdf::SdfContent;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Glb,
    Obj,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Glb => "glb",
            ExportFormat::Obj => "obj",
        }
    }

    pub fn filter_name(&self) -> &'static str {
        match self {
            ExportFormat::Glb => "glTF Binary",
            ExportFormat::Obj => "Wavefront OBJ",
        }
    }
}

/// Export status messages
#[derive(Debug, Clone)]
pub enum ExportStatus {
    Started(String),
    Progress(String),
    Done(String),
    Error(String),
}

/// Start mesh export in background thread
pub fn export_mesh(
    sdf_content: &SdfContent,
    format: ExportFormat,
    resolution: u32,
    status_tx: Sender<ExportStatus>,
) {
    let tree = sdf_content.tree.clone();
    let bounds = sdf_content.bounds;

    thread::spawn(move || {
        let _ = status_tx.send(ExportStatus::Started(
            format!("Exporting as .{} (res={})", format.extension(), resolution),
        ));

        // Show save dialog
        let save_path = rfd::FileDialog::new()
            .add_filter(format.filter_name(), &[format.extension()])
            .set_file_name(format!("export.{}", format.extension()))
            .save_file();

        let path = match save_path {
            Some(p) => p,
            None => {
                let _ = status_tx.send(ExportStatus::Error("Export cancelled".to_string()));
                return;
            }
        };

        let _ = status_tx.send(ExportStatus::Progress("Generating mesh...".to_string()));

        match generate_and_save(&tree, bounds, resolution, &path, format) {
            Ok(info) => {
                let _ = status_tx.send(ExportStatus::Done(
                    format!("Saved: {} ({})", path.display(), info),
                ));
            }
            Err(e) => {
                let _ = status_tx.send(ExportStatus::Error(format!("Export failed: {}", e)));
            }
        }
    });
}

fn generate_and_save(
    tree: &alice_sdf::types::SdfTree,
    bounds: (glam::Vec3, glam::Vec3),
    resolution: u32,
    path: &PathBuf,
    format: ExportFormat,
) -> anyhow::Result<String> {
    use alice_sdf::prelude::*;

    // Generate mesh via marching cubes
    let config = MarchingCubesConfig {
        resolution: resolution as usize,
        compute_normals: true,
        compute_uvs: true,
        ..Default::default()
    };
    let mesh = sdf_to_mesh(&tree.root, bounds.0, bounds.1, &config);
    let vertex_count = mesh.vertices.len();
    let tri_count = mesh.indices.len() / 3;

    match format {
        ExportFormat::Glb => {
            let glb_config = GltfConfig::default();
            export_glb(&mesh, path, &glb_config, None)?;
        }
        ExportFormat::Obj => {
            let obj_config = ObjConfig::default();
            export_obj(&mesh, path, &obj_config, None)?;
        }
    }

    Ok(format!("{} vertices, {} triangles", vertex_count, tri_count))
}
