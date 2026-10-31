extern crate image;
extern crate rustc_serialize;

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
mod texture;
mod noise;
mod volume;
mod motion;
mod scene_file;

use std::{env, io, process};
use std::io::Write;
use std::time::Instant;

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut options = match try!(cli::Options::parse(&args)) {
        Some(options) => options,
        None => { println!("{}", cli::HELP); return Ok(()); }
    };
    let file=match options.scene_file {
        Some(ref path)=>Some(try!(scene_file::SceneFile::load(path))),
        None=>None
    };
    if let Some(ref file)=file { try!(file.apply_defaults(&mut options)); }
    let mut raytracer = if let Some(ref file)=file {
        try!(file.build(&options))
    } else if options.scene == "original" {
        demo::original(options.width, options.height)
    } else if options.scene=="teapot" {
        let mesh=match options.obj {
            Some(ref path)=>try!(shapes::mesh::Mesh::load(path).map_err(|e|format!("Could not load OBJ {}: {}",path,e))),
            None=>try!(shapes::mesh::Mesh::parse(include_str!("../assets/teapot.obj")))
        };
        println!("Loaded {} triangles (BVH accelerated)",mesh.triangle_count());
        demo::mesh_scene(options.width,options.height,options.light_samples,mesh)
    } else if options.scene=="classic" {
        demo::classic(options.width,options.height,options.light_samples)
    } else {
        demo::demo(options.width, options.height, options.light_samples)
    };
    if file.is_none() && options.clouds=="volume" {raytracer.scene.environment=Some(texture::Environment::volumetric(17,options.cloud_steps));}
    if let Some(ref path) = options.sky {
        raytracer.scene.environment = Some(try!(texture::Environment::load(path)
            .map_err(|error| format!("Could not load sky {}: {}", path, error))));
    }
    if let Some(ref mut environment) = raytracer.scene.environment {
        environment.set_rotation(options.sky_rotation);
        environment.set_time(options.time);
        environment.set_loop(options.cloud_loop);
    }
    raytracer.camera = raytracer.camera.with_lens(options.aperture, options.focus);
    if options.check_only {
        println!("Valid scene: {} objects, {} lights",raytracer.scene.objects.len(),raytracer.scene.lights.len());
        return Ok(());
    }
    if let Some(parent)=::std::path::Path::new(&options.output).parent() {
        if parent!=::std::path::Path::new("") {
            try!(::std::fs::create_dir_all(parent).map_err(|e|format!("Could not create {}: {}",parent.display(),e)));
        }
    }
    let start = Instant::now();
    let name=options.scene_file.as_ref().map(|s|s.as_str()).unwrap_or(&options.scene);
    println!("Rendering {} at {}x{}, {} samples/pixel, {} bounces...", name, options.width, options.height,
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
