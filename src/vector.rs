use std::ops::{Add, Sub, Neg};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64
}

impl Vector {
    pub fn new(dx: f64, dy: f64, dz: f64) -> Vector {
        Vector { dx: dx, dy: dy, dz: dz }
    }

    pub fn magnitude(&self) -> f64 {
        (self.dx.powi(2) + self.dy.powi(2) + self.dz.powi(2)).sqrt()
    }

    pub fn normalize(&self) -> Vector {
        let magnitude = self.magnitude();
        assert!(magnitude > 0.0 && magnitude.is_finite(), "Cannot normalize a zero or non-finite vector");

        Vector::new(
            self.dx / magnitude,
            self.dy / magnitude,
            self.dz / magnitude
        )
    }

    pub fn cross(&self, o: &Vector) -> Vector {
        Vector::new(
            self.dy * o.dz - self.dz * o.dy,
            self.dz * o.dx - self.dx * o.dz,
            self.dx * o.dy - self.dy * o.dx
        )
    }

    pub fn scale(&self, scalar: f64) -> Vector {
        Vector::new(
            self.dx * scalar,
            self.dy * scalar,
            self.dz * scalar
        )
    }

    pub fn dot(&self, other: &Vector) -> f64 {
        self.dx * other.dx + self.dy * other.dy + self.dz * other.dz
    }

    /// Reflect an incoming direction about a unit surface normal.
    pub fn reflect(&self, normal: &Vector) -> Vector {
        *self - normal.scale(2.0 * self.dot(normal))
    }
}

impl Sub for Vector {
    type Output = Vector;

    fn sub(self, other: Vector) -> Vector {
        self + other.scale(-1.0)
    }
}

impl Neg for Vector {
    type Output = Vector;

    fn neg(self) -> Vector {
        self.scale(-1.0)
    }
}

impl Add for Vector {
    type Output = Vector;

    fn add(self, o: Vector) -> Vector {
        Vector::new(
            self.dx + o.dx,
            self.dy + o.dy,
            self.dz + o.dz
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Vector;

    #[test]
    fn test_dot_and_reflection() {
        let incoming = Vector::new(1.0, -1.0, 0.0);
        let normal = Vector::new(0.0, 1.0, 0.0);
        assert_eq!(incoming.dot(&normal), -1.0);
        assert_eq!(incoming.reflect(&normal), Vector::new(1.0, 1.0, 0.0));
    }

    #[test]
    #[should_panic]
    fn test_zero_vector_cannot_be_normalized() {
        Vector::new(0.0, 0.0, 0.0).normalize();
    }

    #[test]
    fn test_magnitude() {
        let v = Vector::new(1.0, 2.0, 3.0);

        assert_eq!(v.magnitude(), 3.7416573867739413);
    }

    #[test]
    fn test_normalize() {
        let v = Vector::new(1.0, 2.0, 3.0);
        let normalized = v.normalize();

        assert_eq!(
            normalized,
            Vector::new(0.2672612419124244, 0.5345224838248488, 0.8017837257372732)
        );
    }

    #[test]
    fn test_cross() {
        let v1 = Vector::new(0.0, 1.0, 0.0);
        let v2 = Vector::new(0.0, 0.0, 1.0);
        let cross = v1.cross(&v2);

        assert_eq!(cross, Vector::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_scale() {
        let v = Vector::new(1.0, 2.0, 3.0);

        let result = v.scale(3.0);

        assert_eq!(result.dx, 3.0);
        assert_eq!(result.dy, 6.0);
        assert_eq!(result.dz, 9.0);
    }

    #[test]
    fn test_add() {
        let v1 = Vector::new(1.0, 3.0, 5.0);
        let v2 = Vector::new(2.0, 4.0, 6.0);

        let result = v1 + v2;

        assert_eq!(result.dx, 3.0);
        assert_eq!(result.dy, 7.0);
        assert_eq!(result.dz, 11.0);
    }
}
