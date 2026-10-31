use super::camera::Camera;
use super::color::Color;
use super::light::Light;
use super::material::{Material, Pattern, Bump};
use super::object::Object;
use super::point::Point;
use super::raytracer::Raytracer;
use super::scene::{Scene, Haze};
use super::shapes::sphere::Sphere;
use super::shapes::plane::Plane;
use super::vector::Vector;
use super::texture::Environment;
use super::shapes::mesh::Mesh;
use super::shapes::transformed::Transformed;
use super::transform::Transform;

/// The original two spheres, retained as a flat-color regression/demo scene.
pub fn original(width: u32, height: u32) -> Raytracer {
    let mut scene = Scene::new(Color::new(0, 149, 205));
    scene.objects = vec![
        Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, -350.0), 300.0)),
            Color::new(255, 0, 0), Material::unlit()),
        Object::with_material(Box::new(Sphere::new(Point::new(-250.0, -250.0, -500.0), 300.0)),
            Color::new(0, 255, 0), Material::unlit())
    ];
    Raytracer {
        camera: Camera::new(Point::new(0.0, 0.0, 100.0), Vector::new(0.0, 1.0, 0.0),
            Vector::new(0.0, 0.0, 1.0), 100.0 * width as f64 / 640.0, width, height),
        scene: scene
    }
}

/// The classic ray-tracing study: polished spheres above a checkerboard.
pub fn demo(width: u32, height: u32, light_samples: u32) -> Raytracer {
    spheres(width,height,light_samples,true)
}

pub fn classic(width: u32,height: u32,light_samples: u32) -> Raytracer {
    spheres(width,height,light_samples,false)
}

fn spheres(width: u32, height: u32, light_samples: u32, textured: bool) -> Raytracer {
    let sky = Color::new(111, 160, 211);
    let mut scene = Scene::new(sky);
    scene.environment = Some(Environment::cloudy(17));
    scene.haze = Some(Haze::new(20.0, 100.0));
    scene.lights = vec![
        Light::area(Point::new(-4.5, 6.5, 3.0), Vector::new(2.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.5),
            light_samples, Color::new(255, 247, 231), 105.0),
        Light::directional(Vector::new(-1.0, -2.0, -1.0), Color::new(190, 216, 255), 0.18)
    ];

    let floor = Material {
        reflective: 0.12, specular: 0.1, shininess: 120.0,
        pattern: Some(Pattern::Checkers(Color::new(235, 232, 222), Color::new(28, 36, 50), 1.2)),
        ..Material::default()
    };
    scene.objects.push(Object::with_material(
        Box::new(Plane::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0))), Color::new(255, 255, 255), floor));
    // Two large optical studies, with smaller colored spheres for their reflections.
    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(-1.35, 1.15, -0.35), 1.15)),
        Color::new(240, 244, 250), Material { reflective: 0.94, specular: 0.6, shininess: 300.0, ..Material::default() }));
    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(1.25, 1.05, 0.0), 1.05)),
        Color::new(250, 253, 255), Material::glass(1.5)));
    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(-0.15, 0.5, 1.9), 0.5)),
        Color::new(218, 33, 43), Material { reflective: 0.22, specular: 0.65, shininess: 220.0, ..Material::default() }));

    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(-3.0, 0.6, -1.7), 0.6)),
        Color::new(36, 80, 205), Material { reflective: 0.18, specular: 0.6, shininess: 180.0, ..Material::default() }));
    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(2.9, 0.65, -1.7), 0.65)),
        Color::new(218, 155, 49), Material { reflective: 0.35, specular: 0.55, shininess: 180.0, ..Material::default() }));

    if textured {
        scene.objects[0].material.bump=Some(Bump {scale:8.0,strength:0.012,seed:7});
        scene.objects[1].material.roughness=0.12;
        scene.objects[2].material.transmission_roughness=0.18;
        scene.objects[2].material.roughness=0.12;
        scene.objects[3].material.pattern=Some(Pattern::Marble(Color::new(238,230,221),Color::new(62,75,100),0.35));
        scene.objects[3].material.bump=Some(Bump {scale:6.0,strength:0.012,seed:5});
        scene.objects[4].material.pattern=Some(Pattern::Wood(Color::new(62,23,10),Color::new(193,115,42),0.7));
        scene.objects[4].material.roughness=0.25;
        scene.objects[5].material.pattern=Some(Pattern::Stone(Color::new(53,68,62),Color::new(180,190,176),0.18));
        scene.objects[5].material.bump=Some(Bump {scale:14.0,strength:0.02,seed:9});
        scene.objects[5].material.roughness=0.4;
    }
    Raytracer {
        camera: Camera::look_at(Point::new(0.0, 2.6, 8.5), Point::new(0.0, 1.0, 0.0),
            Vector::new(0.0, 1.0, 0.0), 50.0, width, height),
        scene: scene
    }
}

pub fn mesh_scene(width: u32,height: u32,light_samples: u32,mesh: Mesh) -> Raytracer {
    let mut tracer=classic(width,height,light_samples);
    tracer.scene.objects.truncate(1);
    let shape=Transformed::new(Box::new(mesh.fitted(3.6)),Transform::identity().rotate_y(-0.35));
    tracer.scene.objects.push(Object::with_material(Box::new(shape),Color::new(206,124,64),
        Material {reflective:0.45,roughness:0.06,shininess:180.0,..Material::default()}));
    tracer.scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(-2.25,0.55,0.7),0.55)),Color::new(255,255,255),
        Material {pattern:Some(Pattern::Marble(Color::new(235,230,210),Color::new(45,66,90),0.3)),reflective:0.15,..Material::default()}));
    tracer.scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(2.15,0.7,0.5),0.7)),Color::new(250,253,255),Material::glass(1.5)));
    tracer.camera=Camera::look_at(Point::new(0.8,2.6,8.0),Point::new(0.0,0.85,0.0),Vector::new(0.0,1.0,0.0),48.0,width,height);
    tracer
}
