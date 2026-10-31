//! Versioned scene descriptions, translated into the renderer's existing types.
use super::camera::Camera;
use super::cli::Options;
use super::color::Color;
use super::light::Light;
use super::material::{Bump, Material, Pattern};
use super::object::Object;
use super::point::Point;
use super::raytracer::Raytracer;
use super::scene::{Haze, Scene};
use super::shapes::cube::Cube;
use super::shapes::mesh::Mesh;
use super::shapes::plane::Plane;
use super::shapes::sphere::Sphere;
use super::shapes::transformed::Transformed;
use super::shapes::triangle::Triangle;
use super::shapes::Shape;
use super::texture::Environment;
use super::transform::Transform;
use super::vector::Vector;
use rustc_serialize::json::Json;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

struct Node<'a> {
    value: &'a Json,
    path: String,
}

impl<'a> Node<'a> {
    fn error<T>(&self, message: &str) -> Result<T, String> {
        Err(format!("{}: {}", self.path, message))
    }

    fn fields(&self, allowed: &[&str]) -> Result<(), String> {
        let map = match self.value.as_object() {
            Some(map) => map,
            None => return self.error("expected an object"),
        };
        for key in map.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(format!("{}.{}: unknown field", self.path, key));
            }
        }
        Ok(())
    }

    fn optional(&self, key: &str) -> Option<Node<'a>> {
        self.value
            .as_object()
            .and_then(|map| map.get(key))
            .map(|value| Node {
                value: value,
                path: format!("{}.{}", self.path, key),
            })
    }

    fn get(&self, key: &str) -> Result<Node<'a>, String> {
        self.optional(key)
            .ok_or(format!("{}.{}: required field", self.path, key))
    }

    fn text(&self) -> Result<&'a str, String> {
        match self.value.as_string() {
            Some(text) => Ok(text),
            None => self.error("expected a string"),
        }
    }

    fn number(&self) -> Result<f64, String> {
        match self.value.as_f64() {
            Some(value) if value.is_finite() => Ok(value),
            _ => self.error("expected a finite number"),
        }
    }

    fn range(&self, low: f64, high: f64) -> Result<f64, String> {
        let value = try!(self.number());
        if value < low || value > high {
            return self.error(&format!("must be between {} and {}", low, high));
        }
        Ok(value)
    }

    fn positive(&self) -> Result<f64, String> {
        let value = try!(self.number());
        if value <= 0.0 {
            return self.error("must be positive");
        }
        Ok(value)
    }

    fn integer(&self, low: u32, high: u32) -> Result<u32, String> {
        let value = try!(self.range(low as f64, high as f64));
        if value.fract() != 0.0 {
            return self.error("expected an integer");
        }
        Ok(value as u32)
    }

    fn items(&self) -> Result<Vec<Node<'a>>, String> {
        match self.value.as_array() {
            Some(items) => Ok(items
                .iter()
                .enumerate()
                .map(|(index, value)| Node {
                    value: value,
                    path: format!("{}[{}]", self.path, index),
                })
                .collect()),
            None => self.error("expected an array"),
        }
    }

    fn triple(&self) -> Result<[f64; 3], String> {
        let items = try!(self.items());
        if items.len() != 3 {
            return self.error("expected exactly three numbers");
        }
        Ok([
            try!(items[0].number()),
            try!(items[1].number()),
            try!(items[2].number()),
        ])
    }

    fn point(&self) -> Result<Point, String> {
        let v = try!(self.triple());
        Ok(Point::new(v[0], v[1], v[2]))
    }

    fn vector(&self) -> Result<Vector, String> {
        let v = try!(self.triple());
        let result = Vector::new(v[0], v[1], v[2]);
        if !result.magnitude().is_finite() || result.magnitude() < 1.0e-12 {
            return self.error("expected a nonzero, finite direction");
        }
        Ok(result)
    }

    fn color(&self) -> Result<Color, String> {
        let items = try!(self.items());
        if items.len() != 3 {
            return self.error("expected three sRGB integers in 0..255");
        }
        Ok(Color::new(
            try!(items[0].integer(0, 255)) as u8,
            try!(items[1].integer(0, 255)) as u8,
            try!(items[2].integer(0, 255)) as u8,
        ))
    }
}

pub struct SceneFile {
    json: Json,
    path: PathBuf,
}

impl SceneFile {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<SceneFile, String> {
        let path = path.as_ref();
        let mut file = try!(File::open(path).map_err(|e| format!("{}: {}", path.display(), e)));
        let mut text = String::new();
        try!(file
            .read_to_string(&mut text)
            .map_err(|e| format!("{}: {}", path.display(), e)));
        SceneFile::parse(&text, path)
    }

    pub fn parse(text: &str, path: &Path) -> Result<SceneFile, String> {
        let json = try!(Json::from_str(text).map_err(|e| format!("{}: {}", path.display(), e)));
        let file = SceneFile {
            json: json,
            path: path.to_path_buf(),
        };
        {
            let root = file.root();
            try!(root.fields(&[
                "$schema",
                "version",
                "title",
                "description",
                "render",
                "camera",
                "background",
                "ambient",
                "environment",
                "haze",
                "materials",
                "lights",
                "objects"
            ]));
            try!(try!(root.get("version")).integer(1, 1));
            for key in &["$schema", "title", "description"] {
                if let Some(node) = root.optional(key) {
                    try!(node.text());
                }
            }
        }
        Ok(file)
    }

    fn root(&self) -> Node {
        Node {
            value: &self.json,
            path: self.path.display().to_string(),
        }
    }

    /// File defaults fill only flags the user did not explicitly set, even when
    /// an explicit CLI value happens to equal the built-in default.
    pub fn apply_defaults(&self, options: &mut Options) -> Result<(), String> {
        let root = self.root();
        if let Some(render) = root.optional("render") {
            try!(render.fields(&["width", "height", "samples", "light_samples", "depth"]));
            for &(key, flag, low, high) in &[
                ("width", "--width", 1, 8192),
                ("height", "--height", 1, 8192),
                ("samples", "--samples", 1, 16),
                ("light_samples", "--light-samples", 1, 64),
                ("depth", "--depth", 0, 16),
            ] {
                if let Some(node) = render.optional(key) {
                    let value = try!(node.integer(low, high));
                    if !options.specified(flag) {
                        match key {
                            "width" => options.width = value,
                            "height" => options.height = value,
                            "samples" => options.render.samples_per_axis = value,
                            "light_samples" => options.light_samples = value,
                            _ => options.render.max_depth = value,
                        }
                    }
                }
            }
        }
        let camera = try!(root.get("camera"));
        try!(camera.fields(&["from", "at", "up", "fov", "aperture", "focus"]));
        if let Some(node) = camera.optional("aperture") {
            let value = try!(node.range(0.0, 1.0e6));
            if !options.specified("--aperture") {
                options.aperture = value;
            }
        }
        let focus = match camera.optional("focus") {
            Some(node) => try!(node.positive()),
            None => (try!(try!(camera.get("from")).point()) - try!(try!(camera.get("at")).point()))
                .magnitude(),
        };
        if !options.specified("--focus") {
            options.focus = focus;
        }
        if let Some(environment) = root.optional("environment") {
            try!(environment.fields(&[
                "type",
                "path",
                "seed",
                "rotation",
                "time",
                "loop_duration",
                "steps"
            ]));
            if let Some(node) = environment.optional("rotation") {
                let value = try!(node.number());
                if !options.specified("--sky-rotation") {
                    options.sky_rotation = value;
                }
            }
            if let Some(node) = environment.optional("time") {
                let value = try!(node.range(-1000000.0, 1000000.0));
                if !options.specified("--time") {
                    options.time = value;
                }
            }
            if let Some(node) = environment.optional("loop_duration") {
                let value = try!(node.positive());
                if !options.specified("--cloud-loop") {
                    options.cloud_loop = value;
                }
            }
            if let Some(node) = environment.optional("steps") {
                let value = try!(node.integer(8, 128));
                if !options.specified("--cloud-steps") {
                    options.cloud_steps = value;
                }
            }
        }
        options.validate_sampling()
    }

    pub fn build(&self, options: &Options) -> Result<Raytracer, String> {
        let root = self.root();
        let camera = try!(root.get("camera"));
        let from = try!(try!(camera.get("from")).point());
        let at = try!(try!(camera.get("at")).point());
        let up = match camera.optional("up") {
            Some(node) => try!(node.vector()),
            None => Vector::new(0.0, 1.0, 0.0),
        };
        let direction = from - at;
        if !direction.magnitude().is_finite()
            || direction.magnitude() < 1.0e-12
            || direction.normalize().cross(&up.normalize()).magnitude() < 1.0e-8
        {
            return camera
                .error("from and at must differ; up must not be parallel to the view direction");
        }
        let fov = match camera.optional("fov") {
            Some(node) => try!(node.range(1.0, 179.0)),
            None => 50.0,
        };
        let camera = Camera::look_at(from, at, up, fov, options.width, options.height)
            .with_lens(options.aperture, options.focus);
        let background = match root.optional("background") {
            Some(node) => try!(node.color()),
            None => Color::new(0, 0, 0),
        };
        let mut scene = Scene::new(background);
        if let Some(node) = root.optional("ambient") {
            scene.ambient = try!(node.color());
        }
        if let Some(node) = root.optional("haze") {
            try!(node.fields(&["start", "end"]));
            let start = try!(try!(node.get("start")).range(0.0, 1.0e12));
            let end = try!(try!(node.get("end")).positive());
            if end <= start {
                return node.error("end must be greater than start");
            }
            scene.haze = Some(Haze::new(start, end));
        }
        scene.environment = try!(self.environment(root.optional("environment"), options));
        let mut materials = BTreeMap::new();
        if let Some(node) = root.optional("materials") {
            let map = match node.value.as_object() {
                Some(map) => map,
                None => return node.error("expected named materials"),
            };
            for (name, value) in map {
                if name.is_empty() {
                    return node.error("material names cannot be empty");
                }
                let material = try!(parse_material(&Node {
                    value: value,
                    path: format!("{}.{}", node.path, name)
                }));
                materials.insert(name.clone(), material);
            }
        }
        for node in try!(try!(root.get("objects")).items()) {
            let object = try!(self.object(&node, &materials));
            scene.objects.push(object);
        }
        if let Some(lights) = root.optional("lights") {
            for node in try!(lights.items()) {
                scene
                    .lights
                    .push(try!(parse_light(&node, options.light_samples)));
            }
        }
        Ok(Raytracer {
            camera: camera,
            scene: scene,
        })
    }

    fn asset(&self, node: &Node) -> Result<PathBuf, String> {
        let path = try!(node.text());
        if path.is_empty() {
            return node.error("asset path cannot be empty");
        }
        Ok(self.path.parent().unwrap_or(Path::new(".")).join(path))
    }

    fn environment(
        &self,
        node: Option<Node>,
        options: &Options,
    ) -> Result<Option<Environment>, String> {
        let environment = match node {
            Some(node) => {
                let kind = try!(node.get("type"));
                let seed = match node.optional("seed") {
                    Some(value) => try!(value.integer(0, ::std::u32::MAX)),
                    None => 17,
                };
                let mut name = try!(kind.text());
                if !["clouds", "volume", "panorama"].contains(&name) {
                    return kind.error("expected clouds, volume, or panorama");
                }
                if options.sky.is_some() {
                    return Ok(None);
                }
                if options.specified("--clouds") {
                    name = if options.clouds == "volume" {
                        "volume"
                    } else {
                        "clouds"
                    };
                }
                match name {
                    "clouds" => Environment::cloudy(seed),
                    "volume" => Environment::volumetric(seed, options.cloud_steps),
                    "panorama" => {
                        let path = try!(self.asset(&try!(node.get("path"))));
                        try!(Environment::load(&path).map_err(|e| format!(
                            "{}.path ({}): {}",
                            node.path,
                            path.display(),
                            e
                        )))
                    }
                    _ => unreachable!(),
                }
            }
            None => {
                if !options.specified("--clouds") {
                    return Ok(None);
                }
                if options.clouds == "volume" {
                    Environment::volumetric(17, options.cloud_steps)
                } else {
                    Environment::cloudy(17)
                }
            }
        };
        Ok(Some(environment))
    }

    fn object(
        &self,
        node: &Node,
        materials: &BTreeMap<String, (Color, Material)>,
    ) -> Result<Object, String> {
        let kind = try!(try!(node.get("type")).text());
        let mut fields = vec!["type", "name", "color", "material", "transform"];
        let geometry = match kind {
            "sphere" => vec!["center", "radius"],
            "plane" => vec!["point", "normal"],
            "cube" => vec!["min", "max"],
            "triangle" => vec!["vertices"],
            "mesh" => vec!["path", "fit"],
            _ => return node.error("type must be sphere, plane, cube, triangle, or mesh"),
        };
        fields.extend(geometry);
        try!(node.fields(&fields));
        if let Some(name) = node.optional("name") {
            try!(name.text());
        }
        let (mut color, material) = match node.optional("material") {
            Some(value) => match value.value.as_string() {
                Some(name) => match materials.get(name) {
                    Some(material) => *material,
                    None => return value.error(&format!("unknown material {:?}", name)),
                },
                None => try!(parse_material(&value)),
            },
            None => (Color::new(255, 255, 255), Material::default()),
        };
        if let Some(value) = node.optional("color") {
            color = try!(value.color());
        }
        let shape: Box<Shape> = match kind {
            "sphere" => {
                let center = match node.optional("center") {
                    Some(value) => try!(value.point()),
                    None => Point::new(0.0, 0.0, 0.0),
                };
                let radius = match node.optional("radius") {
                    Some(value) => try!(value.positive()),
                    None => 1.0,
                };
                Box::new(Sphere::new(center, radius))
            }
            "plane" => {
                let point = match node.optional("point") {
                    Some(value) => try!(value.point()),
                    None => Point::new(0.0, 0.0, 0.0),
                };
                let normal = match node.optional("normal") {
                    Some(value) => try!(value.vector()),
                    None => Vector::new(0.0, 1.0, 0.0),
                };
                Box::new(Plane::new(point, normal))
            }
            "cube" => {
                let min = match node.optional("min") {
                    Some(value) => try!(value.point()),
                    None => Point::new(-1.0, -1.0, -1.0),
                };
                let max = match node.optional("max") {
                    Some(value) => try!(value.point()),
                    None => Point::new(1.0, 1.0, 1.0),
                };
                if min.x >= max.x || min.y >= max.y || min.z >= max.z {
                    return node.error("cube min must be smaller than max on every axis");
                }
                Box::new(Cube::new(min, max))
            }
            "triangle" => {
                let vertices = try!(try!(node.get("vertices")).items());
                if vertices.len() != 3 {
                    return node.error("triangle requires three vertices");
                }
                let a = try!(vertices[0].point());
                let b = try!(vertices[1].point());
                let c = try!(vertices[2].point());
                let area = (b - a).cross(&(c - a)).magnitude();
                if !area.is_finite() || area < 1.0e-12 {
                    return node.error("triangle vertices must not be collinear");
                }
                Box::new(Triangle::new(a, b, c))
            }
            _ => {
                let path = try!(self.asset(&try!(node.get("path"))));
                let mut mesh = try!(Mesh::load(&path).map_err(|e| format!(
                    "{}.path ({}): {}",
                    node.path,
                    path.display(),
                    e
                )));
                if let Some(value) = node.optional("fit") {
                    mesh = mesh.fitted(try!(value.positive()));
                }
                Box::new(mesh)
            }
        };
        let shape = match node.optional("transform") {
            Some(value) => {
                Box::new(Transformed::new(shape, try!(parse_transform(&value)))) as Box<Shape>
            }
            None => shape,
        };
        Ok(Object::with_material(shape, color, material))
    }
}

fn parse_material(node: &Node) -> Result<(Color, Material), String> {
    try!(node.fields(&[
        "preset",
        "color",
        "ambient",
        "diffuse",
        "specular",
        "shininess",
        "reflective",
        "transparency",
        "refractive_index",
        "roughness",
        "transmission_roughness",
        "pattern",
        "bump"
    ]));
    let mut material = match node.optional("preset") {
        Some(value) => match try!(value.text()) {
            "glass" => Material::glass(1.5),
            "mirror" => Material {
                reflective: 0.95,
                ..Material::default()
            },
            "unlit" => Material::unlit(),
            "default" => Material::default(),
            _ => return value.error("preset must be default, glass, mirror, or unlit"),
        },
        None => Material::default(),
    };
    let color = match node.optional("color") {
        Some(value) => try!(value.color()),
        None => Color::new(255, 255, 255),
    };
    for key in &[
        "ambient",
        "diffuse",
        "specular",
        "reflective",
        "transparency",
        "roughness",
        "transmission_roughness",
    ] {
        if let Some(value) = node.optional(key) {
            let v = try!(value.range(0.0, 1.0));
            match *key {
                "ambient" => material.ambient = v,
                "diffuse" => material.diffuse = v,
                "specular" => material.specular = v,
                "reflective" => material.reflective = v,
                "transparency" => material.transparency = v,
                "roughness" => material.roughness = v,
                _ => material.transmission_roughness = v,
            }
        }
    }
    if let Some(value) = node.optional("shininess") {
        material.shininess = try!(value.range(1.0, 1.0e9));
    }
    if let Some(value) = node.optional("refractive_index") {
        material.refractive_index = try!(value.positive());
    }
    if let Some(value) = node.optional("bump") {
        try!(value.fields(&["scale", "strength", "seed"]));
        material.bump = Some(Bump {
            scale: try!(try!(value.get("scale")).positive()),
            strength: try!(try!(value.get("strength")).range(0.0, 1.0e6)),
            seed: match value.optional("seed") {
                Some(seed) => try!(seed.integer(0, ::std::u32::MAX)),
                None => 17,
            },
        });
    }
    if let Some(value) = node.optional("pattern") {
        try!(value.fields(&["type", "color_a", "color_b", "size"]));
        let a = try!(try!(value.get("color_a")).color());
        let b = try!(try!(value.get("color_b")).color());
        let size = try!(try!(value.get("size")).positive());
        material.pattern = Some(match try!(try!(value.get("type")).text()) {
            "checkers" => Pattern::Checkers(a, b, size),
            "checkers3d" => Pattern::Checkers3D(a, b, size),
            "stripes" => Pattern::Stripes(a, b, size),
            "marble" => Pattern::Marble(a, b, size),
            "wood" => Pattern::Wood(a, b, size),
            "stone" => Pattern::Stone(a, b, size),
            _ => return value.error("unknown pattern type"),
        });
    }
    Ok((color, material))
}

fn parse_transform(node: &Node) -> Result<Transform, String> {
    let mut transform = Transform::identity();
    for op in try!(node.items()) {
        try!(op.fields(&["scale", "translate", "rotate_x", "rotate_y", "rotate_z"]));
        if op.value.as_object().unwrap().len() != 1 {
            return op.error("each transform must contain exactly one operation");
        }
        if let Some(value) = op.optional("scale") {
            let v = try!(value.triple());
            if v.iter().any(|s| *s == 0.0 || !(1.0 / s).is_finite()) {
                return value.error("scale components must be nonzero with finite reciprocals");
            }
            transform = transform.scale(v[0], v[1], v[2]);
        } else if let Some(value) = op.optional("translate") {
            let v = try!(value.triple());
            transform = transform.translate(v[0], v[1], v[2]);
        } else {
            for key in &["rotate_x", "rotate_y", "rotate_z"] {
                if let Some(value) = op.optional(key) {
                    let radians = (try!(value.number()) % 360.0) * ::std::f64::consts::PI / 180.0;
                    transform = match *key {
                        "rotate_x" => transform.rotate_x(radians),
                        "rotate_y" => transform.rotate_y(radians),
                        _ => transform.rotate_z(radians),
                    };
                }
            }
        }
        if !transform.is_finite() {
            return op.error("transform composition must remain finite");
        }
    }
    Ok(transform)
}

fn parse_light(node: &Node, steps: u32) -> Result<Light, String> {
    let kind = try!(try!(node.get("type")).text());
    let mut fields = vec!["type", "color", "intensity"];
    let geometry = match kind {
        "point" => vec!["position"],
        "directional" => vec!["direction"],
        "area" => vec!["corner", "u", "v"],
        _ => return node.error("unknown light type"),
    };
    fields.extend(geometry);
    try!(node.fields(&fields));
    let color = match node.optional("color") {
        Some(value) => try!(value.color()),
        None => Color::new(255, 255, 255),
    };
    let intensity = match node.optional("intensity") {
        Some(value) => try!(value.range(0.0, 1.0e12)),
        None => 1.0,
    };
    match kind {
        "point" => Ok(Light::point(
            try!(try!(node.get("position")).point()),
            color,
            intensity,
        )),
        "directional" => Ok(Light::directional(
            try!(try!(node.get("direction")).vector()),
            color,
            intensity,
        )),
        _ => {
            let u = try!(try!(node.get("u")).vector());
            let v = try!(try!(node.get("v")).vector());
            if u.normalize().cross(&v.normalize()).magnitude() < 1.0e-8 {
                return node.error("area light u and v must not be parallel");
            }
            Ok(Light::area(
                try!(try!(node.get("corner")).point()),
                u,
                v,
                steps,
                color,
                intensity,
            ))
        }
    }
}
