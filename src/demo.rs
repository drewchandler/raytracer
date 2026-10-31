use super::camera::Camera;
use super::color::Color;
use super::light::Light;
use super::material::{Material, Pattern};
use super::object::Object;
use super::point::Point;
use super::raytracer::Raytracer;
use super::scene::Scene;
use super::shapes::sphere::Sphere;
use super::shapes::plane::Plane;
use super::shapes::cube::Cube;
use super::shapes::triangle::Triangle;
use super::shapes::transformed::Transformed;
use super::transform::Transform;
use super::vector::Vector;

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

/// A small scene exercising the same object/shape architecture with richer materials.
pub fn demo(width: u32, height: u32, light_samples: u32) -> Raytracer {
    let mut scene = Scene::new(Color::new(35, 46, 65));
    scene.lights = vec![
        Light::area(Point::new(-4.0, 7.0, 1.0), Vector::new(4.0, 0.0, 0.0), Vector::new(0.0, 0.0, 3.0),
            light_samples, Color::new(255, 244, 226), 110.0),
        Light::directional(Vector::new(-1.0, -2.0, -1.0), Color::new(181, 209, 255), 0.25)
    ];

    let floor = Material {
        reflective: 0.18, specular: 0.15, shininess: 120.0,
        pattern: Some(Pattern::Checkers(Color::new(194, 199, 207), Color::new(66, 81, 99), 1.2)),
        ..Material::default()
    };
    scene.objects.push(Object::with_material(
        Box::new(Plane::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0))), Color::new(255, 255, 255), floor));
    scene.objects.push(Object::with_material(
        Box::new(Plane::new(Point::new(0.0, 0.0, -6.0), Vector::new(0.0, 0.0, 1.0))),
        Color::new(82, 105, 133), Material { specular: 0.0, ..Material::default() }));

    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(-1.7, 1.0, -0.4), 1.0)),
        Color::new(203, 47, 51), Material { reflective: 0.12, shininess: 150.0, ..Material::default() }));
    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(1.0, 1.0, 0.3), 1.0)),
        Color::new(245, 252, 255), Material::glass(1.5)));
    scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(-0.3, 0.6, 1.7), 0.6)),
        Color::new(225, 230, 240), Material { reflective: 0.88, specular: 0.3, shininess: 200.0, ..Material::default() }));

    let cube = Transformed::new(Box::new(Cube::new(Point::new(-1.0, -1.0, -1.0), Point::new(1.0, 1.0, 1.0))),
        Transform::identity().scale(0.7, 0.7, 0.7).rotate_y(-0.3).translate(2.2, 0.7, -2.0));
    scene.objects.push(Object::new(Box::new(cube), Color::new(46, 155, 116)));

    let triangle = Transformed::new(Box::new(Triangle::new(
        Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0), Point::new(0.0, 2.5, 0.0))),
        Transform::identity().rotate_y(-0.25).translate(-4.3, 0.0, -0.8));
    scene.objects.push(Object::with_material(Box::new(triangle), Color::new(232, 163, 63), Material {
        pattern: Some(Pattern::Stripes(Color::new(232, 163, 63), Color::new(147, 84, 38), 0.3)),
        reflective: 0.1, ..Material::default()
    }));

    Raytracer {
        camera: Camera::look_at(Point::new(6.8, 4.0, 8.0), Point::new(0.0, 0.9, -0.4),
            Vector::new(0.0, 1.0, 0.0), 53.0, width, height),
        scene: scene
    }
}
