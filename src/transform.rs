use super::point::Point;
use super::vector::Vector;

type Matrix = [[f64; 4]; 4];

/// An invertible affine transform. Both matrices are cached once, not per ray.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    forward: Matrix,
    inverse: Matrix
}

fn identity() -> Matrix {
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
}

fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut result = [[0.0; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            for k in 0..4 { result[row][col] += a[row][k] * b[k][col]; }
        }
    }
    result
}

fn apply(matrix: &Matrix, x: f64, y: f64, z: f64, w: f64) -> [f64; 3] {
    let mut result = [0.0; 3];
    for row in 0..3 {
        result[row] = matrix[row][0] * x + matrix[row][1] * y + matrix[row][2] * z + matrix[row][3] * w;
    }
    result
}

impl Transform {
    pub fn identity() -> Transform { Transform { forward: identity(), inverse: identity() } }

    pub fn is_finite(&self) -> bool {
        self.forward.iter().chain(self.inverse.iter()).all(|row|row.iter().all(|v|v.is_finite()))
    }

    /// Chained operations are applied in reading order: scale, rotate, translate.
    pub fn then(&self, next: Transform) -> Transform {
        Transform {
            forward: multiply(&next.forward, &self.forward),
            inverse: multiply(&self.inverse, &next.inverse)
        }
    }

    pub fn translate(&self, x: f64, y: f64, z: f64) -> Transform {
        assert!(x.is_finite() && y.is_finite() && z.is_finite());
        let mut t = Transform::identity();
        for (axis, value) in [x, y, z].iter().enumerate() {
            t.forward[axis][3] = *value;
            t.inverse[axis][3] = -*value;
        }
        self.then(t)
    }

    pub fn scale(&self, x: f64, y: f64, z: f64) -> Transform {
        let mut t = Transform::identity();
        for (axis, value) in [x, y, z].iter().enumerate() {
            assert!(value.is_finite() && *value != 0.0, "Transform scale must be finite and nonzero");
            t.forward[axis][axis] = *value;
            t.inverse[axis][axis] = 1.0 / *value;
        }
        self.then(t)
    }

    fn rotate(&self, a: usize, b: usize, radians: f64) -> Transform {
        assert!(radians.is_finite());
        let mut t = Transform::identity();
        let (s, c) = radians.sin_cos();
        t.forward[a][a] = c; t.forward[a][b] = -s;
        t.forward[b][a] = s; t.forward[b][b] = c;
        t.inverse[a][a] = c; t.inverse[a][b] = s;
        t.inverse[b][a] = -s; t.inverse[b][b] = c;
        self.then(t)
    }

    pub fn rotate_x(&self, radians: f64) -> Transform { self.rotate(1, 2, radians) }
    pub fn rotate_y(&self, radians: f64) -> Transform { self.rotate(2, 0, radians) }
    pub fn rotate_z(&self, radians: f64) -> Transform { self.rotate(0, 1, radians) }

    pub fn point(&self, point: &Point) -> Point {
        let p = apply(&self.forward, point.x, point.y, point.z, 1.0);
        Point::new(p[0], p[1], p[2])
    }

    pub fn inverse_point(&self, point: &Point) -> Point {
        let p = apply(&self.inverse, point.x, point.y, point.z, 1.0);
        Point::new(p[0], p[1], p[2])
    }

    pub fn inverse_vector(&self, vector: &Vector) -> Vector {
        let v = apply(&self.inverse, vector.dx, vector.dy, vector.dz, 0.0);
        Vector::new(v[0], v[1], v[2])
    }

    /// Normals use the inverse transpose, including under nonuniform scaling.
    pub fn normal(&self, normal: &Vector) -> Vector {
        self.normal_vector(normal).normalize()
    }

    pub fn normal_vector(&self, normal: &Vector) -> Vector {
        let m = &self.inverse;
        Vector::new(
            m[0][0] * normal.dx + m[1][0] * normal.dy + m[2][0] * normal.dz,
            m[0][1] * normal.dx + m[1][1] * normal.dy + m[2][1] * normal.dz,
            m[0][2] * normal.dx + m[1][2] * normal.dy + m[2][2] * normal.dz
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Transform;
    use super::super::point::Point;
    use std::f64::consts::PI;

    #[test]
    fn composition_order_and_inverse() {
        let t = Transform::identity().scale(2.0, 3.0, 4.0).rotate_z(PI / 2.0).translate(5.0, 0.0, 0.0);
        let p = Point::new(1.0, 0.0, 0.0);
        let q = t.point(&p);
        assert!((q.x - 5.0).abs() < 1.0e-10 && (q.y - 2.0).abs() < 1.0e-10);
        let back = t.inverse_point(&q);
        assert!((back - p).magnitude() < 1.0e-10);
    }

    #[test]
    fn rotations_about_all_axes_follow_right_hand_rule() {
        let p = Point::new(0.0, 1.0, 0.0);
        let x = Transform::identity().rotate_x(PI / 2.0).point(&p);
        assert!((x - Point::new(0.0, 0.0, 1.0)).magnitude() < 1.0e-10);
        let y = Transform::identity().rotate_y(PI / 2.0).point(&x);
        assert!((y - Point::new(1.0, 0.0, 0.0)).magnitude() < 1.0e-10);
        let z = Transform::identity().rotate_z(PI / 2.0).point(&y);
        assert!((z - p).magnitude() < 1.0e-10);
    }

    #[test]
    #[should_panic]
    fn singular_transforms_are_rejected() { Transform::identity().scale(1.0, 0.0, 1.0); }
}
