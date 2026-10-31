extern crate image;

mod camera;
mod cli;
mod color;
mod demo;
mod light;
mod material;
mod shapes;
mod object;
mod point;
mod ray;
mod raytracer;
mod scene;
mod vector;
mod transform;

use std::{env, io, process};
use std::io::Write;
use std::time::Instant;

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = match try!(cli::Options::parse(&args)) {
        Some(options) => options,
        None => { println!("{}", cli::HELP); return Ok(()); }
    };
    let mut raytracer = if options.scene == "original" {
        demo::original(options.width, options.height)
    } else {
        demo::demo(options.width, options.height, options.light_samples)
    };
    raytracer.camera = raytracer.camera.with_lens(options.aperture, options.focus);
    let start = Instant::now();
    println!("Rendering {} at {}x{}, {} samples/pixel, {} bounces...", options.scene, options.width, options.height,
        options.render.samples_per_axis * options.render.samples_per_axis, options.render.max_depth);
    let image = raytracer.render(&options.render);
    try!(image.save(&options.output).map_err(|error| format!("Could not save {}: {}", options.output, error)));
    println!("Saved {} ({:?})", options.output, start.elapsed());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        let _ = writeln!(&mut io::stderr(), "Error: {}", error);
        process::exit(1);
    }
}
