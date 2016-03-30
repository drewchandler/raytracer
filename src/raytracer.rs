use super::camera::Camera;
use super::color::Color;
use super::ray::Ray;
use super::scene::Scene;

pub struct Raytracer {
    pub camera: Camera,
    pub scene: Scene
}

impl Raytracer {
    pub fn pixel(&self, x: u32, y: u32) -> Color {
        let ray = self.camera.camera_ray(x, y);
        self.cast(&ray)
    }

    fn cast(&self, r: &Ray) -> Color {
        match self.scene.nearest_interection(&r) {
            Some(i) => i.object.color.clone(),
            _ => self.scene.background.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Raytracer;
    use super::super::camera::Camera;
    use super::super::color::Color;
    use super::super::object::Object;
    use super::super::point::Point;
    use super::super::scene::Scene;
    use super::super::shapes::sphere::Sphere;
    use super::super::vector::Vector;

    #[test]
    fn test_pixel_use_background_when_no_intersection() {
        let background = Color::new(0, 0, 0);
        let camera = Camera::new(
            Point::new(0.0, 0.0, 100.0),
            Vector::new(0.0, 1.0, 0.0),
            Vector::new(0.0, 0.0, 1.0),
            100.0,
            640,
            480
        );
        let scene = Scene {
            background: background.clone(),
            objects: vec![]
        };
        let raytracer = Raytracer {
            camera: camera,
            scene: scene
        };

        let result = raytracer.pixel(0, 0);

        assert_eq!(result, background);
    }

    #[test]
    fn test_pixel_uses_intersected_objects_color() {
        let object_color = Color::new(0, 255, 0);
        let camera = Camera::new(
            Point::new(0.0, 0.0, 100.0),
            Vector::new(0.0, 1.0, 0.0),
            Vector::new(0.0, 0.0, 1.0),
            100.0,
            640,
            480
        );
        let scene = Scene {
            background: Color::new(0, 0, 0),
            objects: vec![
                Object {
                    shape: Box::new(Sphere::new(
                        Point::new(0.0, 0.0, -100.0),
                        10.0
                    )),
                    color: object_color.clone()
                }
            ]
        };
        let raytracer = Raytracer {
            camera: camera,
            scene: scene
        };

        let result = raytracer.pixel(320, 240);

        assert_eq!(result, object_color);
    }
}
