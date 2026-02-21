//! ALICE File Creator - CLI tool to create .alice files
//!
//! Usage:
//!   alice-create linear --slope 0.005 --intercept 25.0 --samples 1000 -o sensor_data.alice
//!   alice-create mandelbrot --iterations 256 -o fractal.alice
//!   alice-create julia --cx -0.7 --cy 0.27 -o julia.alice
//!   alice-create perlin --seed 12345 --scale 5.0 -o terrain.alice

use alice_view::decoder::alice::*;
use std::fs;

fn print_usage() {
    println!("ALICE File Creator");
    println!("==================");
    println!();
    println!("Usage:");
    println!("  alice-create linear --slope <f32> --intercept <f32> [--samples <u32>] [-o <file>]");
    println!("  alice-create linear-q16 --slope <i32> --intercept <i32> [--samples <u32>] [-o <file>]");
    println!("  alice-create mandelbrot [--iterations <u32>] [--cx <f64>] [--cy <f64>] [-o <file>]");
    println!("  alice-create julia [--cx <f64>] [--cy <f64>] [--iterations <u32>] [-o <file>]");
    println!("  alice-create perlin [--seed <u64>] [--scale <f32>] [--octaves <u32>] [-o <file>]");
    println!("  alice-create demo [-o <file>]");
    println!();
    println!("Options:");
    println!("  -o, --output <file>   Output file path (default: output.alice)");
    println!("  --sensor-id <id>      Sensor ID metadata");
    println!("  --unit <unit>         Unit of measurement (e.g., °C, m/s)");
    println!();
    println!("Examples:");
    println!("  alice-create linear --slope 0.005 --intercept 25.0 --samples 1000 -o temp.alice");
    println!("  alice-create mandelbrot -o fractal.alice");
    println!("  alice-create demo --sensor-id TEMP-001 --unit °C -o demo.alice");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    let command = &args[1];

    // Parse common options
    let mut output_path = "output.alice".to_string();
    let mut sensor_id: Option<String> = None;
    let mut unit: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--sensor-id" => {
                if i + 1 < args.len() {
                    sensor_id = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--unit" => {
                if i + 1 < args.len() {
                    unit = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    let result = match command.as_str() {
        "linear" => create_linear(&args[2..], sensor_id, unit),
        "linear-q16" => create_linear_q16(&args[2..], sensor_id, unit),
        "mandelbrot" => create_mandelbrot(&args[2..]),
        "julia" => create_julia(&args[2..]),
        "perlin" => create_perlin(&args[2..]),
        "demo" => create_demo(sensor_id, unit),
        "-h" | "--help" | "help" => {
            print_usage();
            return;
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            return;
        }
    };

    match result {
        Ok(file) => {
            // Find output path from args again (since we parsed separately)
            let mut final_output = output_path;
            for i in 0..args.len() - 1 {
                if args[i] == "-o" || args[i] == "--output" {
                    final_output = args[i + 1].clone();
                    break;
                }
            }

            let bytes = file.to_bytes();
            if let Err(e) = fs::write(&final_output, &bytes) {
                eprintln!("Failed to write file: {}", e);
                return;
            }

            println!("✅ Created: {}", final_output);
            println!("   Type: {}", file.content_type_name());
            println!("   Equation: {}", file.equation_string());
            println!("   Size: {} bytes", bytes.len());
            println!("   Compression: {:.0}x", file.compression_ratio());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

fn create_linear(args: &[String], sensor_id: Option<String>, unit: Option<String>) -> anyhow::Result<AliceFile> {
    let mut slope: f32 = 0.005;
    let mut intercept: f32 = 25.0;
    let mut samples: u32 = 1000;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--slope" if i + 1 < args.len() => {
                slope = args[i + 1].parse()?;
                i += 2;
            }
            "--intercept" if i + 1 < args.len() => {
                intercept = args[i + 1].parse()?;
                i += 2;
            }
            "--samples" if i + 1 < args.len() => {
                samples = args[i + 1].parse()?;
                i += 2;
            }
            _ => i += 1,
        }
    }

    // Convert float to Q16.16
    let slope_q16 = (slope * 65536.0) as i32;
    let intercept_q16 = (intercept * 65536.0) as i32;

    let mut builder = AliceFileBuilder::from_linear(slope_q16, intercept_q16, samples);
    if let Some(id) = sensor_id {
        builder = builder.sensor_id(&id);
    }
    if let Some(u) = unit {
        builder = builder.unit(&u);
    }

    builder.build()
}

fn create_linear_q16(args: &[String], sensor_id: Option<String>, unit: Option<String>) -> anyhow::Result<AliceFile> {
    let mut slope_q16: i32 = 32767;
    let mut intercept_q16: i32 = 163840000;
    let mut samples: u32 = 1000;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--slope" if i + 1 < args.len() => {
                slope_q16 = args[i + 1].parse()?;
                i += 2;
            }
            "--intercept" if i + 1 < args.len() => {
                intercept_q16 = args[i + 1].parse()?;
                i += 2;
            }
            "--samples" if i + 1 < args.len() => {
                samples = args[i + 1].parse()?;
                i += 2;
            }
            _ => i += 1,
        }
    }

    let mut builder = AliceFileBuilder::from_linear(slope_q16, intercept_q16, samples);
    if let Some(id) = sensor_id {
        builder = builder.sensor_id(&id);
    }
    if let Some(u) = unit {
        builder = builder.unit(&u);
    }

    builder.build()
}

fn create_mandelbrot(args: &[String]) -> anyhow::Result<AliceFile> {
    let mut iterations: u32 = 256;
    let mut cx: f64 = -0.75;
    let mut cy: f64 = 0.0;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--iterations" if i + 1 < args.len() => {
                iterations = args[i + 1].parse()?;
                i += 2;
            }
            "--cx" if i + 1 < args.len() => {
                cx = args[i + 1].parse()?;
                i += 2;
            }
            "--cy" if i + 1 < args.len() => {
                cy = args[i + 1].parse()?;
                i += 2;
            }
            _ => i += 1,
        }
    }

    AliceFileBuilder::mandelbrot(iterations, cx, cy).build()
}

fn create_julia(args: &[String]) -> anyhow::Result<AliceFile> {
    let mut iterations: u32 = 256;
    let mut cx: f64 = -0.7;
    let mut cy: f64 = 0.27;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--iterations" if i + 1 < args.len() => {
                iterations = args[i + 1].parse()?;
                i += 2;
            }
            "--cx" if i + 1 < args.len() => {
                cx = args[i + 1].parse()?;
                i += 2;
            }
            "--cy" if i + 1 < args.len() => {
                cy = args[i + 1].parse()?;
                i += 2;
            }
            _ => i += 1,
        }
    }

    AliceFileBuilder::julia(iterations, cx, cy).build()
}

fn create_perlin(args: &[String]) -> anyhow::Result<AliceFile> {
    let mut seed: u64 = 12345;
    let mut scale: f32 = 5.0;
    let mut octaves: u32 = 6;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" if i + 1 < args.len() => {
                seed = args[i + 1].parse()?;
                i += 2;
            }
            "--scale" if i + 1 < args.len() => {
                scale = args[i + 1].parse()?;
                i += 2;
            }
            "--octaves" if i + 1 < args.len() => {
                octaves = args[i + 1].parse()?;
                i += 2;
            }
            _ => i += 1,
        }
    }

    AliceFileBuilder::perlin(seed, scale, octaves).build()
}

fn create_demo(sensor_id: Option<String>, unit: Option<String>) -> anyhow::Result<AliceFile> {
    // Create a demo file similar to what ALICE-Edge outputs
    // slope = 0.005 (temperature increase per sample)
    // intercept = 25.0 (base temperature)
    // samples = 1000

    let slope_q16 = 32767;       // ~0.5 in Q16.16
    let intercept_q16 = 163824115; // ~2499.76 in Q16.16

    let mut builder = AliceFileBuilder::from_linear(slope_q16, intercept_q16, 1000);

    if let Some(id) = sensor_id {
        builder = builder.sensor_id(&id);
    } else {
        builder = builder.sensor_id("TEMP-001");
    }

    if let Some(u) = unit {
        builder = builder.unit(&u);
    } else {
        builder = builder.unit("°C");
    }

    builder = builder.timestamp("2025-01-30T12:00:00Z");

    builder.build()
}
