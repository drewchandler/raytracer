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
    height: u32,
    aperture: f64,
    focus_distance: f64
}

impl Camera {
    pub fn new(location: Point, v: Vector, n: Vector, distance: f64, width: u32, height: u32) -> Camera {
        assert!(width > 0 && height > 0);
        assert!(distance > 0.0 && distance.is_finite());
        let n = n.normalize();
        let u = v.cross(&n).normalize();
        let v = n.cross(&u).normalize();

        Camera {
            location: location,
            u: u,
            v: v,
            n: n,
            distance: distance,
            width: width,
            height: height,
            aperture: 0.0,
            focus_distance: distance
        }
    }

    pub fn camera_ray(&self, x: u32, y: u32) -> Ray {
        self.sample_ray(x as f64 + 0.5, y as f64 + 0.5)
    }

    /// Subpixel coordinates, measured from the image's top-left corner.
    pub fn sample_ray(&self, x: f64, y: f64) -> Ray {
        let direction = self.u.scale(x - (self.width as f64 / 2.0)) +
                        self.v.scale((self.height as f64 / 2.0) - y) +
                        self.n.scale(-self.distance);

        Ray::new(self.location.clone(), direction.normalize())
    }

    pub fn look_at(location: Point, target: Point, up: Vector, fov_degrees: f64, width: u32, height: u32) -> Camera {
        assert!(fov_degrees > 0.0 && fov_degrees < 180.0 && fov_degrees.is_finite());
        let distance = width as f64 / (2.0 * (fov_degrees * ::std::f64::consts::PI / 360.0).tan());
        Camera::new(location, up, location - target, distance, width, height)
    }

    pub fn with_lens(mut self, aperture_radius: f64, focus_distance: f64) -> Camera {
        assert!(aperture_radius >= 0.0 && aperture_radius.is_finite());
        assert!(focus_distance > 0.0 && focus_distance.is_finite());
        self.aperture = aperture_radius;
        self.focus_distance = focus_distance;
        self
    }

    /// Uniform disk lens sampling; a zero aperture is the original pinhole camera.
    pub fn lens_ray(&self, x: f64, y: f64, lens_u: f64, lens_v: f64) -> Ray {
        let ray = self.sample_ray(x, y);
        if self.aperture == 0.0 { return ray; }
        let radius = lens_u.sqrt() * self.aperture;
        let angle = 2.0 * ::std::f64::consts::PI * lens_v;
        let origin = self.location + self.u.scale(radius * angle.cos()) + self.v.scale(radius * angle.sin());
        let focus = ray.at(self.focus_distance / -ray.direction.dot(&self.n));
        Ray::new(origin, (focus - origin).normalize())
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
}

#[cfg(test)]
mod tests {
    use super::Camera;
    use super::super::point::Point;
    use super::super::vector::Vector;

    #[test]
    fn look_at_center_ray_points_at_target() {
        let location = Point::new(2.0, 3.0, 4.0);
        let target = Point::new(-1.0, 0.0, 0.0);
        let camera = Camera::look_at(location, target, Vector::new(0.0, 2.0, 0.0), 60.0, 101, 51);
        let ray = camera.camera_ray(50, 25);
        assert!((ray.direction - (target - location).normalize()).magnitude() < 1.0e-10);
        assert!((camera.u.dot(&camera.v)).abs() < 1.0e-10);
    }

    #[test]
    fn lens_rays_converge_on_focus_plane() {
        let camera = Camera::look_at(Point::new(0.0, 0.0, 0.0), Point::new(0.0, 0.0, -1.0),
            Vector::new(0.0, 1.0, 0.0), 90.0, 100, 100).with_lens(0.1, 5.0);
        let a = camera.lens_ray(50.0, 50.0, 0.2, 0.3);
        let b = camera.lens_ray(50.0, 50.0, 0.7, 0.8);
        let pa = a.at(-5.0 / a.direction.dz);
        let pb = b.at(-5.0 / b.direction.dz);
        assert!((pa - pb).magnitude() < 1.0e-10);
        assert!(a.origin != b.origin);
    }

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
