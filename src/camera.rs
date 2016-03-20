use super::point::Point;
use super::ray::Ray;
use super::vector::Vector;

pub struct Camera {
    location: Point,
    u: Vector,
    v: Vector,
    n: Vector,
    distance: f64,
    width: u32,
    height: u32
}

impl Camera {
    pub fn new(location: Point, v: Vector, n: Vector, distance: f64, width: u32, height: u32) -> Camera {
        let u = v.cross(&n).normalize();

        Camera {
            location: location,
            u: u,
            v: v,
            n: n,
            distance: distance,
            width: width,
            height: height
        }
    }

    pub fn camera_ray(&self, x: u32, y: u32) -> Ray {
        let direction = self.u.scale(x as f64 - (self.width as f64 / 2.0) + 0.5) +
                        self.v.scale((self.height as f64 / 2.0) - y as f64 - 0.5) +
                        self.n.scale(-self.distance);

        Ray::new(self.location.clone(), direction.normalize())
    }
}

#[cfg(test)]
mod tests {
    use super::Camera;
    use super::super::point::Point;
    use super::super::vector::Vector;

    #[test]
    fn test_camera_ray() {
        let camera = Camera::new(
            Point::new(0.0, 0.0, 100.0),
            Vector::new(0.0, 1.0, 0.0),
            Vector::new(0.0, 0.0, 1.0),
            100.0,
            640,
            480
        );
        let camera_ray = camera.camera_ray(0, 0);

        assert_eq!(camera_ray.origin, camera.location);
        assert_eq!(camera_ray.direction, Vector::new(-319.5, 239.5, -100.0).normalize());
    }
}
