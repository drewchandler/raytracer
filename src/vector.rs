use std::ops::Add;

#[derive(Debug, PartialEq)]
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
