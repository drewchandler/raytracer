use super::super::point::Point;
use super::super::ray::Ray;
use super::super::vector::Vector;
use super::Shape;
use std::f64;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: Point,
    max: Point,
}
impl Bounds {
    fn empty() -> Bounds {
        Bounds {
            min: Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
    fn add(&mut self, p: Point) {
        self.min = Point::new(
            self.min.x.min(p.x),
            self.min.y.min(p.y),
            self.min.z.min(p.z),
        );
        self.max = Point::new(
            self.max.x.max(p.x),
            self.max.y.max(p.y),
            self.max.z.max(p.z),
        );
    }
    fn contains(&self, p: &Point) -> bool {
        let e = 1.0e-5;
        p.x >= self.min.x - e
            && p.x <= self.max.x + e
            && p.y >= self.min.y - e
            && p.y <= self.max.y + e
            && p.z >= self.min.z - e
            && p.z <= self.max.z + e
    }
    fn intersects(&self, r: &Ray) -> bool {
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::INFINITY;
        for &(o, d, a, b) in &[
            (r.origin.x, r.direction.dx, self.min.x, self.max.x),
            (r.origin.y, r.direction.dy, self.min.y, self.max.y),
            (r.origin.z, r.direction.dz, self.min.z, self.max.z),
        ] {
            if d == 0.0 {
                if o < a || o > b {
                    return false;
                }
            } else {
                let x = (a - o) / d;
                let y = (b - o) / d;
                lo = lo.max(x.min(y));
                hi = hi.min(x.max(y));
                if lo > hi {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone)]
struct Face {
    a: Point,
    b: Point,
    c: Point,
    normals: Option<[Vector; 3]>,
}
impl Face {
    fn bounds(&self) -> Bounds {
        let mut b = Bounds::empty();
        b.add(self.a);
        b.add(self.b);
        b.add(self.c);
        b
    }
    fn barycentric(&self, p: Point) -> Option<(f64, f64)> {
        let e = self.b - self.a;
        let f = self.c - self.a;
        let q = p - self.a;
        let aa = e.dot(&e);
        let ab = e.dot(&f);
        let bb = f.dot(&f);
        let d = aa * bb - ab * ab;
        if d.abs() < 1.0e-20 {
            return None;
        }
        let u = (bb * q.dot(&e) - ab * q.dot(&f)) / d;
        let v = (aa * q.dot(&f) - ab * q.dot(&e)) / d;
        let n = e.cross(&f).normalize();
        if u >= -1.0e-5 && v >= -1.0e-5 && u + v <= 1.0 + 1.0e-5 && q.dot(&n).abs() < 1.0e-5 {
            Some((u, v))
        } else {
            None
        }
    }
    fn hit(&self, r: &Ray) -> Option<(f64, f64, f64)> {
        let e = self.b - self.a;
        let f = self.c - self.a;
        let p = r.direction.cross(&f);
        let d = e.dot(&p);
        if d == 0.0 || d.abs() < 1.0e-10 * e.magnitude() * p.magnitude() {
            return None;
        }
        let s = r.origin - self.a;
        let u = s.dot(&p) / d;
        if u < -1.0e-10 || u > 1.0 + 1.0e-10 {
            return None;
        }
        let q = s.cross(&e);
        let v = r.direction.dot(&q) / d;
        if v < -1.0e-10 || u + v > 1.0 + 1.0e-10 {
            return None;
        }
        let t = f.dot(&q) / d;
        if t.is_finite() {
            Some((t, u, v))
        } else {
            None
        }
    }
    fn normal(&self, u: f64, v: f64) -> Vector {
        if let Some(n) = self.normals {
            let interpolated = n[0].scale(1.0 - u - v) + n[1].scale(u) + n[2].scale(v);
            if interpolated.magnitude() > 1.0e-10 {
                return interpolated.normalize();
            }
        }
        (self.b - self.a).cross(&(self.c - self.a)).normalize()
    }
}

enum Node {
    Leaf(Bounds, Vec<usize>),
    Branch(Bounds, Box<Node>, Box<Node>),
}
impl Node {
    fn build(faces: &[Face], mut indices: Vec<usize>) -> Node {
        let mut b = Bounds::empty();
        for i in &indices {
            let f = faces[*i].bounds();
            b.add(f.min);
            b.add(f.max);
        }
        if indices.len() <= 6 {
            return Node::Leaf(b, indices);
        }
        let extent = b.max - b.min;
        let axis = if extent.dx >= extent.dy && extent.dx >= extent.dz {
            0
        } else if extent.dy >= extent.dz {
            1
        } else {
            2
        };
        let center = |i: usize| {
            let f = &faces[i];
            match axis {
                0 => (f.a.x + f.b.x + f.c.x) / 3.0,
                1 => (f.a.y + f.b.y + f.c.y) / 3.0,
                _ => (f.a.z + f.b.z + f.c.z) / 3.0,
            }
        };
        indices.sort_by(|a, b| center(*a).partial_cmp(&center(*b)).unwrap());
        let mid = indices.len() / 2;
        let right = indices.split_off(mid);
        Node::Branch(
            b,
            Box::new(Node::build(faces, indices)),
            Box::new(Node::build(faces, right)),
        )
    }
    fn bounds(&self) -> Bounds {
        match *self {
            Node::Leaf(b, _) | Node::Branch(b, _, _) => b,
        }
    }
    fn hits(
        &self,
        faces: &[Face],
        r: &Ray,
        out: &mut Vec<(usize, f64, f64, f64)>,
        tested: &mut usize,
    ) {
        if !self.bounds().intersects(r) {
            return;
        }
        match *self {
            Node::Leaf(_, ref indices) => {
                for i in indices {
                    *tested += 1;
                    if let Some((t, u, v)) = faces[*i].hit(r) {
                        out.push((*i, t, u, v));
                    }
                }
            }
            Node::Branch(_, ref left, ref right) => {
                left.hits(faces, r, out, tested);
                right.hits(faces, r, out, tested);
            }
        }
    }
    fn point_normal(&self, faces: &[Face], p: &Point) -> Option<Vector> {
        if !self.bounds().contains(p) {
            return None;
        }
        match *self {
            Node::Leaf(_, ref indices) => {
                for i in indices {
                    if let Some((u, v)) = faces[*i].barycentric(*p) {
                        return Some(faces[*i].normal(u, v));
                    }
                }
                None
            }
            Node::Branch(_, ref left, ref right) => left
                .point_normal(faces, p)
                .or_else(|| right.point_normal(faces, p)),
        }
    }
}

/// A triangle mesh with a median-split bounding volume hierarchy.
pub struct Mesh {
    faces: Vec<Face>,
    root: Node,
}

fn obj_index(text: &str, count: usize) -> Result<usize, String> {
    let n = try!(text
        .parse::<i64>()
        .map_err(|_| format!("invalid OBJ index: {}", text)));
    let index = if n < 0 { count as i64 + n } else { n - 1 };
    if n == 0 || index < 0 || index >= count as i64 {
        Err(format!("OBJ index out of range: {}", text))
    } else {
        Ok(index as usize)
    }
}

impl Mesh {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Mesh, String> {
        let mut file = try!(File::open(path).map_err(|e| e.to_string()));
        let mut text = String::new();
        try!(file.read_to_string(&mut text).map_err(|e| e.to_string()));
        Mesh::parse(&text)
    }
    pub fn parse(text: &str) -> Result<Mesh, String> {
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut faces = Vec::new();
        for (line_number, line) in text.lines().enumerate() {
            let words: Vec<&str> = line
                .split('#')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect();
            if words.is_empty() {
                continue;
            }
            let parsed: Result<(), String> = (|| {
                match words[0] {
                    "v" | "vn" => {
                        if words.len() < 4 {
                            return Err("vertex/normal needs three coordinates".to_string());
                        }
                        let mut values = [0.0; 3];
                        for i in 0..3 {
                            values[i] = try!(words[i + 1]
                                .parse::<f64>()
                                .map_err(|_| "invalid coordinate".to_string()));
                            if !values[i].is_finite() || values[i].abs() > 1.0e12 {
                                return Err("coordinate must be finite and within 1e12".to_string());
                            }
                        }
                        if words[0] == "v" {
                            vertices.push(Point::new(values[0], values[1], values[2]));
                        } else {
                            let n = Vector::new(values[0], values[1], values[2]);
                            if n.magnitude() == 0.0 {
                                return Err("zero OBJ normal".to_string());
                            }
                            normals.push(n.normalize());
                        }
                    }
                    "f" => {
                        if words.len() < 4 {
                            return Err("face needs at least three vertices".to_string());
                        }
                        let mut refs = Vec::new();
                        for word in &words[1..] {
                            let parts: Vec<&str> = word.split('/').collect();
                            if parts.len() > 3 {
                                return Err("invalid face reference".to_string());
                            }
                            let vertex = try!(obj_index(parts[0], vertices.len()));
                            let normal = if parts.len() == 3 && !parts[2].is_empty() {
                                Some(try!(obj_index(parts[2], normals.len())))
                            } else {
                                None
                            };
                            refs.push((vertex, normal));
                        }
                        for i in 1..refs.len() - 1 {
                            let r = [refs[0], refs[i], refs[i + 1]];
                            let a = vertices[r[0].0];
                            let b = vertices[r[1].0];
                            let c = vertices[r[2].0];
                            if (b - a).cross(&(c - a)).magnitude() < 1.0e-12 {
                                return Err("degenerate OBJ face".to_string());
                            }
                            let ns = match (r[0].1, r[1].1, r[2].1) {
                                (Some(x), Some(y), Some(z)) => {
                                    Some([normals[x], normals[y], normals[z]])
                                }
                                _ => None,
                            };
                            faces.push(Face {
                                a: a,
                                b: b,
                                c: c,
                                normals: ns,
                            });
                        }
                    }
                    _ => {} // groups, UVs, and material directives do not change this single-material mesh
                }
                Ok(())
            })();
            if let Err(error) = parsed {
                return Err(format!("OBJ line {}: {}", line_number + 1, error));
            }
        }
        if faces.is_empty() {
            return Err("OBJ contains no faces".to_string());
        }
        Ok(Mesh::from_faces(faces))
    }
    fn from_faces(faces: Vec<Face>) -> Mesh {
        let root = Node::build(&faces, (0..faces.len()).collect());
        Mesh {
            faces: faces,
            root: root,
        }
    }
    pub fn fitted(self, size: f64) -> Mesh {
        assert!(size > 0.0 && size.is_finite());
        let b = self.root.bounds();
        let e = b.max - b.min;
        let scale = size / e.dx.max(e.dy).max(e.dz);
        let center = Point::new(
            (b.min.x + b.max.x) * 0.5,
            b.min.y,
            (b.min.z + b.max.z) * 0.5,
        );
        let convert = |p: Point| {
            let v = (p - center).scale(scale);
            Point::new(v.dx, v.dy, v.dz)
        };
        let faces = self
            .faces
            .into_iter()
            .map(|f| Face {
                a: convert(f.a),
                b: convert(f.b),
                c: convert(f.c),
                normals: f.normals,
            })
            .collect();
        Mesh::from_faces(faces)
    }
    pub fn triangle_count(&self) -> usize {
        self.faces.len()
    }
    fn hits(&self, r: &Ray) -> (Vec<(usize, f64, f64, f64)>, usize) {
        let mut hits = Vec::new();
        let mut tested = 0;
        self.root.hits(&self.faces, r, &mut hits, &mut tested);
        (hits, tested)
    }
}

impl Shape for Mesh {
    fn intersections(&self, r: &Ray) -> Vec<f64> {
        let mut hits: Vec<f64> = self.hits(r).0.into_iter().map(|h| h.1).collect();
        hits.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut unique: Vec<f64> = Vec::new();
        for t in hits {
            if unique
                .last()
                .map_or(true, |last| (t - *last).abs() > 1.0e-7)
            {
                unique.push(t);
            }
        }
        unique
    }
    fn normal_at(&self, p: &Point) -> Vector {
        self.root
            .point_normal(&self.faces, p)
            .expect("point must lie on the mesh surface")
    }
    fn normal_for_ray(&self, r: &Ray, distance: f64) -> Vector {
        self.normals_for_ray(r, distance).1
    }
    fn normals_for_ray(&self, r: &Ray, distance: f64) -> (Vector, Vector) {
        let (hits, _) = self.hits(r);
        let mut selected = None;
        let mut error = f64::INFINITY;
        for (i, t, u, v) in hits {
            let delta = (t - distance).abs();
            if delta < error {
                error = delta;
                let face = &self.faces[i];
                selected = Some((
                    (face.b - face.a).cross(&(face.c - face.a)).normalize(),
                    face.normal(u, v),
                ));
            }
        }
        selected.expect("mesh normal needs an intersection")
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::point::Point;
    use super::super::super::ray::Ray;
    use super::super::super::vector::Vector;
    use super::super::Shape;
    use super::Mesh;
    #[test]
    fn obj_polygons_negative_indices_and_smooth_normals() {
        let mesh = Mesh::parse(
            "v -1 0 0\nv 1 0 0\nv 1 2 0\nv -1 2 0\nvn 0 0 1\nf -4//1 -3//1 -2//1 -1//1\n",
        )
        .unwrap();
        assert_eq!(mesh.triangle_count(), 2);
        let ray = Ray::new(Point::new(0.0, 1.0, -2.0), Vector::new(0.0, 0.0, 1.0));
        assert_eq!(mesh.intersections(&ray), vec![2.0]);
        assert_eq!(mesh.normal_for_ray(&ray, 2.0), Vector::new(0.0, 0.0, 1.0));
    }
    #[test]
    fn obj_rejects_bad_indices_nonfinite_and_degenerate_faces() {
        for text in &[
            "v NaN 0 0",
            "v 0 0 0\nf 0 1 1",
            "v 0 0 0\nf 1 1 1",
            "v 0 0 0\nf 1 2 3",
            "# empty",
        ] {
            assert!(Mesh::parse(text).is_err());
        }
    }

    #[test]
    fn vertex_normals_interpolate_without_changing_geometric_normals() {
        let mesh = Mesh::parse(
            "v 0 1 0\nv -1 0 0\nv 1 0 0\nvn 0 1 -1\nvn -1 0 -1\nvn 1 0 -1\nf 1//1 2//2 3//3",
        )
        .unwrap();
        let ray = Ray::new(Point::new(-0.2, 0.3, -2.0), Vector::new(0.0, 0.0, 1.0));
        let (geometric, shading) = mesh.normals_for_ray(&ray, 2.0);
        assert_eq!(geometric, Vector::new(0.0, 0.0, 1.0));
        assert!((shading - Vector::new(-0.2, 0.3, -1.0).normalize()).magnitude() < 1.0e-10);
    }
    #[test]
    fn bundled_teapot_loads_and_uses_bvh_and_smooth_normals() {
        let mesh = Mesh::parse(include_str!("../../assets/teapot.obj"))
            .unwrap()
            .fitted(3.6);
        assert_eq!(mesh.triangle_count(), 4032);
        let ray = Ray::new(Point::new(0.0, 0.8, 5.0), Vector::new(0.0, 0.0, -1.0));
        let t = mesh.intersect(&ray).unwrap();
        let n = mesh.normal_for_ray(&ray, t);
        assert!((n.magnitude() - 1.0).abs() < 1.0e-10);
        assert!(mesh.hits(&ray).1 < 400);
    }
    #[test]
    fn bvh_matches_brute_force_and_culls_most_triangles() {
        let mut text = String::new();
        for x in 0..100 {
            let a = x * 3;
            text.push_str(&format!(
                "v {} 0 0\nv {} 0 0\nv {} 1 0\nf {} {} {}\n",
                a,
                a + 1,
                a,
                a + 1,
                a + 2,
                a + 3
            ));
        }
        let mesh = Mesh::parse(&text).unwrap();
        for x in &[0, 15, 42, 99] {
            let ray = Ray::new(
                Point::new(*x as f64 * 3.0 + 0.2, 0.2, -2.0),
                Vector::new(0.0, 0.0, 1.0),
            );
            let (hits, tested) = mesh.hits(&ray);
            let brute: Vec<f64> = mesh
                .faces
                .iter()
                .filter_map(|f| f.hit(&ray).map(|h| h.0))
                .collect();
            assert_eq!(brute, vec![2.0]);
            assert_eq!(hits.iter().map(|h| h.1).collect::<Vec<f64>>(), brute);
            assert!(tested < 20, "tested {} of 100 triangles", tested);
        }
    }
}
