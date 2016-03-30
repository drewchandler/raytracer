extern crate image;

mod camera;
mod color;
mod shapes;
mod object;
mod point;
mod ray;
mod raytracer;
mod scene;
mod vector;

fn main() {
    let scene = scene::Scene {
        background: color::Color::new(0, 149, 205),
        objects: vec![
            object::Object {
                shape: Box::new(shapes::sphere::Sphere::new(
                    point::Point::new(0.0, 0.0, -350.0),
                    300.0
                )),
                color: color::Color::new(255, 0, 0)
            },
            object::Object {
                shape: Box::new(shapes::sphere::Sphere::new(
                    point::Point::new(-250.0, -250.0, -500.0),
                    300.0
                )),
                color: color::Color::new(0, 255, 0)
            }
        ]
    };
    let camera = camera::Camera::new(
        point::Point::new(0.0, 0.0, 100.0),
        vector::Vector::new(0.0, 1.0, 0.0),
        vector::Vector::new(0.0, 0.0, 1.0),
        100.0,
        640,
        480
    );

    let raytracer = raytracer::Raytracer {
        camera: camera,
        scene: scene
    };

    let img = image::ImageBuffer::from_fn(640, 480, |x, y| {
        let color = raytracer.pixel(x, y);

        image::Rgb([color.r, color.g, color.b])
    });

    let _ = img.save("out.png");
}
