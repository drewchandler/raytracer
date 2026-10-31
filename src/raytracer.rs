use image::{ImageBuffer, Rgb, RgbImage};
use super::camera::Camera;
use super::color::{Color, LinearColor};
use super::object::Object;
use super::point::Point;
use super::ray::{Ray, EPSILON};
use super::scene::Scene;
use super::vector::Vector;
use super::noise::{hash,random};

pub struct Raytracer {
    pub camera: Camera,
    pub scene: Scene
}

#[derive(Clone, Copy)]
pub struct RenderSettings {
    /// A regular N x N subpixel grid (not the total number of samples).
    pub samples_per_axis: u32,
    pub max_depth: u32
}

impl Default for RenderSettings {
    fn default() -> RenderSettings { RenderSettings { samples_per_axis: 2, max_depth: 5 } }
}

impl RenderSettings {
    pub fn validate(&self) {
        assert!(self.samples_per_axis >= 1 && self.samples_per_axis <= 16);
        assert!(self.max_depth <= 16);
    }
}

/// Snell's law. None means total internal reflection.
fn refract(incoming: &Vector, normal: &Vector, n1: f64, n2: f64) -> Option<Vector> {
    let ratio = n1 / n2;
    let cos_i = (-incoming.dot(normal)).max(0.0).min(1.0);
    let sin2_t = ratio * ratio * (1.0 - cos_i * cos_i);
    if sin2_t > 1.0 { return None; }
    Some(incoming.scale(ratio) + normal.scale(ratio * cos_i - (1.0 - sin2_t).sqrt()))
}

fn schlick(cosine: f64, n1: f64, n2: f64) -> f64 {
    let mut cosine = cosine.max(0.0).min(1.0);
    if n1 > n2 {
        let sin2_t = (n1 / n2).powi(2) * (1.0 - cosine * cosine);
        if sin2_t > 1.0 { return 1.0; }
        cosine = (1.0 - sin2_t).sqrt();
    }
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    // Equal indices have no interface, even at a grazing angle.
    if n1 == n2 { 0.0 } else { r0 + (1.0 - r0) * (1.0 - cosine).powi(5) }
}

fn radical_inverse(mut index: u32) -> f64 {
    let mut result = 0.0;
    let mut weight = 0.5;
    while index > 0 {
        result += (index % 2) as f64 * weight;
        index /= 2;
        weight *= 0.5;
    }
    result
}

/// One deterministic lobe sample per camera sample, avoiding exponential ray branching.
fn rough_direction(ideal: Vector, side: Vector, roughness: f64, seed: u32) -> Vector {
    if roughness==0.0 {return ideal;}
    let reference=if ideal.dy.abs()<0.9 {Vector::new(0.0,1.0,0.0)} else {Vector::new(1.0,0.0,0.0)};
    let u=ideal.cross(&reference).normalize();let v=ideal.cross(&u).normalize();
    let angle=2.0*::std::f64::consts::PI*random(seed.wrapping_add(13));
    let theta=roughness*roughness*random(seed).sqrt()*::std::f64::consts::PI*0.5;
    let direction=(ideal.scale(theta.cos())+(u.scale(angle.cos())+v.scale(angle.sin())).scale(theta.sin())).normalize();
    if direction.dot(&side)>0.0 {direction} else {ideal}
}

impl Raytracer {
    /// Original center-of-pixel entry point, with recursive shading added.
    pub fn pixel(&self, x: u32, y: u32) -> Color {
        self.trace(&self.camera.camera_ray(x, y), 5).to_color()
    }

    pub fn sampled_pixel(&self, x: u32, y: u32, settings: &RenderSettings) -> Color {
        settings.validate();
        let n = settings.samples_per_axis;
        if n == 1 {
            return self.trace(&self.camera.camera_ray(x, y), settings.max_depth).to_color();
        }
        let mut sum = LinearColor::black();
        for sy in 0..n {
            for sx in 0..n {
                let index = sy * n + sx;
                let ray = self.camera.lens_ray(
                    x as f64 + (sx as f64 + 0.5) / n as f64,
                    y as f64 + (sy as f64 + 0.5) / n as f64,
                    (index as f64 + 0.5) / (n * n) as f64, radical_inverse(index));
                sum = sum + self.trace(&ray, settings.max_depth);
            }
        }
        (sum * (1.0 / (n * n) as f64)).to_color()
    }

    pub fn render(&self, settings: &RenderSettings) -> RgbImage {
        settings.validate();
        ImageBuffer::from_fn(self.camera.width(), self.camera.height(), |x, y| {
            let color = self.sampled_pixel(x, y, settings);
            Rgb([color.r, color.g, color.b])
        })
    }

    /// Whitted-style recursive ray tracing; the depth bounds both branches.
    pub fn trace(&self, ray: &Ray, remaining: u32) -> LinearColor {
        let d=ray.direction.normalize();
        let seed=hash(((d.dx+1.0)*1000003.0) as u32 ^ (((d.dy+1.0)*1000033.0) as u32).wrapping_mul(1013)
            ^ (((d.dz+1.0)*1000037.0) as u32).wrapping_mul(1717));
        self.trace_sample(ray,remaining,seed,4)
    }

    fn trace_sample(&self,ray: &Ray,remaining: u32,seed: u32,rough_budget: u32) -> LinearColor {
        let intersections = self.scene.intersections(ray);
        let index = match intersections.iter().position(|i| i.distance > EPSILON) {
            Some(index) => index,
            None => return self.scene.background_ray(ray)
        };
        let hit = &intersections[index];
        let material = &hit.object.material;
        let point = ray.at(hit.distance);
        let incoming = ray.direction.normalize();
        let eye = -incoming;
        let (outward,smooth) = hit.object.shape.normals_for_ray(ray,hit.distance);
        let geometric = if outward.dot(&eye)<0.0 {-outward} else {outward};
        let smooth=if smooth.dot(&outward)<0.0 {-smooth} else {smooth};
        let perturbed=hit.object.shading_normal(&point,smooth);
        let normal=if outward.dot(&eye)<0.0 {-perturbed} else {perturbed};
        let normal=if normal.dot(&eye)>0.0 {normal} else {geometric};
        let over = point + geometric.scale(EPSILON);
        let under = point + geometric.scale(-EPSILON);
        let (local, specular) = self.lighting(hit.object, &point, &over, &normal, &geometric, &eye);

        // Share energy between the local surface, mirror, and glass components.
        let local_weight = (1.0 - material.reflective) * (1.0 - material.transparency);
        let mut result = local * local_weight + specular;
        if remaining == 0 { return self.scene.surface_color(result, hit.distance * ray.direction.magnitude(), ray); }

        let (n1, n2) = self.scene.refractive_indices(index, &intersections);
        let fresnel = if material.transparency > 0.0 { schlick(eye.dot(&normal), n1, n2) } else { 0.0 };
        let reflected_weight = material.reflective + (1.0 - material.reflective) * material.transparency * fresnel;
        let transmitted_weight = (1.0 - material.reflective) * material.transparency * (1.0 - fresnel);
        if reflected_weight > 0.0 {
            let ideal=incoming.reflect(&normal);
            let ideal=if ideal.dot(&geometric)>0.0 {ideal} else {incoming.reflect(&geometric)};
            let count=if material.roughness>0.0 {rough_budget} else {1};
            for sample in 0..count {
                let next=hash(seed^sample.wrapping_mul(7919)^0xa511e9b3);
                let reflected = Ray::new(over,rough_direction(ideal,geometric,material.roughness,next));
                let budget=if material.roughness>0.0 {1} else {rough_budget};
                result = result + self.trace_sample(&reflected, remaining - 1,next,budget) * (reflected_weight/count as f64);
            }
        }
        if transmitted_weight > 0.0 {
            if let Some(direction) = refract(&incoming, &normal, n1, n2) {
                let direction=if direction.dot(&geometric)<0.0 {direction.normalize()} else {-geometric};
                let count=if material.transmission_roughness>0.0 {rough_budget} else {1};
                for sample in 0..count {
                    let next=hash(seed^sample.wrapping_mul(1717)^0x63d83595);
                    let refracted = Ray::new(under,rough_direction(direction,-geometric,material.transmission_roughness,next));
                    let budget=if material.transmission_roughness>0.0 {1} else {rough_budget};
                    result = result + self.trace_sample(&refracted, remaining - 1,next,budget) * (transmitted_weight/count as f64) * hit.object.albedo_at(&point);
                }
            }
        }
        self.scene.surface_color(result, hit.distance * ray.direction.magnitude(), ray)
    }

    fn lighting(&self, object: &Object, point: &Point, over: &Point, normal: &Vector, geometric: &Vector, eye: &Vector) -> (LinearColor, LinearColor) {
        let material = &object.material;
        let base = object.albedo_at(point);
        let mut diffuse = base * self.scene.ambient.linear() * material.ambient;
        let mut specular = LinearColor::black();
        for light in &self.scene.lights {
            for sample in light.samples(over) {
                let cosine = normal.dot(&sample.direction);
                if cosine <= 0.0 || geometric.dot(&sample.direction)<=0.0 { continue; }
                let shadow = Ray::new(*over, sample.direction);
                let illumination = sample.radiance * self.scene.transmittance(&shadow, sample.distance);
                diffuse = diffuse + base * illumination * (material.diffuse * cosine);
                let reflected = (-sample.direction).reflect(normal);
                let alignment = reflected.dot(eye).max(0.0);
                let shininess=(material.shininess*(1.0-material.roughness).powi(2)).max(1.0);
                specular = specular + illumination * (material.specular * alignment.powf(shininess));
            }
        }
        (diffuse, specular)
    }
}

#[cfg(test)]
mod tests {
    use super::{Raytracer, RenderSettings, refract, schlick, rough_direction};
    use super::super::camera::Camera;
    use super::super::color::{Color, LinearColor};
    use super::super::light::Light;
    use super::super::material::{Material, Pattern};
    use super::super::object::Object;
    use super::super::point::Point;
    use super::super::ray::Ray;
    use super::super::scene::Scene;
    use super::super::shapes::sphere::Sphere;
    use super::super::shapes::plane::Plane;
    use super::super::vector::Vector;
    use super::super::texture::{Environment, ImageTexture};

    fn tracer(scene: Scene) -> Raytracer {
        Raytracer { camera: Camera::new(Point::new(0.0, 0.0, 1.0), Vector::new(0.0, 1.0, 0.0),
            Vector::new(0.0, 0.0, 1.0), 1.0, 1, 1), scene: scene }
    }

    fn front_ray() -> Ray {
        Ray::new(Point::new(0.0, 0.0, -3.0), Vector::new(0.0, 0.0, 1.0))
    }

    #[test]
    fn rough_lobes_are_seeded_normalized_and_stay_on_the_correct_side() {
        let ideal=Vector::new(0.2,1.0,0.0).normalize();let side=Vector::new(0.0,1.0,0.0);
        assert_eq!(rough_direction(ideal,side,0.0,1),ideal);
        assert_eq!(rough_direction(ideal,side,0.7,2),rough_direction(ideal,side,0.7,2));
        let mut narrow=0.0;let mut broad=0.0;
        for seed in 0..100 {
            let d=rough_direction(ideal,side,0.7,seed);
            assert!(d.dot(&side)>0.0 && (d.magnitude()-1.0).abs()<1.0e-10);
            narrow+=rough_direction(ideal,side,0.1,seed).dot(&ideal);broad+=d.dot(&ideal);
            assert!(rough_direction(-ideal,-side,0.7,seed).dot(&side)<0.0);
        }
        assert!(broad<narrow-5.0);
    }

    #[test]
    fn test_pixel_use_background_when_no_intersection() {
        let background = Color::new(12, 34, 56);
        assert_eq!(tracer(Scene::new(background)).pixel(0, 0), background);
    }

    #[test]
    fn environment_is_sampled_by_primary_reflected_and_refracted_rays() {
        let image = ::image::ImageBuffer::from_fn(2, 1, |x, _| {
            if x == 0 { ::image::Rgb([255, 0, 0]) } else { ::image::Rgb([0, 0, 255]) }
        });
        let mut scene = Scene::new(Color::new(0, 255, 0));
        scene.environment = Some(Environment::panorama(ImageTexture::from_image(image)));
        let mut raytracer = tracer(scene);
        assert_eq!(raytracer.trace(&front_ray(), 0).to_color(), Color::new(0, 0, 255));
        let mirror = Material { reflective: 1.0, specular: 0.0, ..Material::default() };
        raytracer.scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255), mirror));
        assert_eq!(raytracer.trace(&front_ray(), 1).to_color(), Color::new(255, 0, 0));
        raytracer.scene.objects[0].material = Material::glass(1.0);
        assert_eq!(raytracer.trace(&front_ray(), 2).to_color(), Color::new(0, 0, 255));
    }

    #[test]
    fn test_pixel_uses_intersected_objects_color() {
        let object_color = Color::new(0, 255, 0);
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, -2.0), 1.0)), object_color, Material::unlit()));
        assert_eq!(tracer(scene).pixel(0, 0), object_color);
    }

    #[test]
    fn phong_diffuse_and_specular_and_shadowed_ambient() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(Object::new(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255)));
        scene.lights.push(Light::directional(Vector::new(0.0, 0.0, 1.0), Color::new(255, 255, 255), 1.0));
        let mut raytracer = tracer(scene);
        assert!((raytracer.trace(&front_ray(), 0).r - 1.28).abs() < 1.0e-9);
        // Block the light behind the camera, without blocking its viewing ray.
        raytracer.scene.objects.push(Object::new(Box::new(Sphere::new(Point::new(0.0, 0.0, -6.0), 1.0)), Color::new(255, 255, 255)));
        assert!((raytracer.trace(&front_ray(), 0).r - 0.08).abs() < 1.0e-9);
    }

    #[test]
    fn light_behind_surface_has_no_diffuse_or_specular() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(Object::new(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255)));
        scene.lights.push(Light::directional(Vector::new(0.0, 0.0, -1.0), Color::new(255, 255, 255), 1.0));
        assert!((tracer(scene).trace(&front_ray(), 0).r - 0.08).abs() < 1.0e-9);
    }

    #[test]
    fn multiple_lights_add_linear_illumination() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        let material = Material { ambient: 0.0, diffuse: 1.0, specular: 0.0, ..Material::default() };
        scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255), material));
        scene.lights.push(Light::directional(Vector::new(0.0, 0.0, 1.0), Color::new(255, 0, 0), 0.5));
        scene.lights.push(Light::directional(Vector::new(0.0, 0.0, 1.0), Color::new(0, 255, 0), 0.25));
        assert_eq!(tracer(scene).trace(&front_ray(), 0), LinearColor::new(0.5, 0.25, 0.0));
    }

    #[test]
    fn area_light_produces_a_partial_shadow() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        let floor = Material { ambient: 0.0, diffuse: 1.0, specular: 0.0, ..Material::default() };
        scene.objects.push(Object::with_material(Box::new(Plane::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0))), Color::new(255, 255, 255), floor));
        scene.lights.push(Light::area(Point::new(-2.0, 2.0, -0.5), Vector::new(4.0, 0.0, 0.0),
            Vector::new(0.0, 0.0, 1.0), 2, Color::new(255, 255, 255), 4.0));
        let mut raytracer = tracer(scene);
        let ray = Ray::new(Point::new(0.0, 0.5, 0.0), Vector::new(0.0, -1.0, 0.0));
        let fully_lit = raytracer.trace(&ray, 0).r;
        raytracer.scene.objects.push(Object::new(Box::new(Sphere::new(Point::new(-0.5, 1.0, 0.0), 0.4)), Color::new(255, 255, 255)));
        let partially_lit = raytracer.trace(&ray, 0).r;
        assert!(fully_lit > 0.0);
        assert!((partially_lit - fully_lit * 0.5).abs() < 1.0e-9);
    }

    #[test]
    fn mirrors_reflect_background_and_stop_at_depth_limit() {
        let background = Color::new(80, 120, 200);
        let mut scene = Scene::new(background);
        let mirror = Material { reflective: 1.0, specular: 0.0, ..Material::default() };
        scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255), mirror));
        let raytracer = tracer(scene);
        assert_eq!(raytracer.trace(&front_ray(), 0), LinearColor::black());
        assert_eq!(raytracer.trace(&front_ray(), 1).to_color(), background);
    }

    #[test]
    fn mutually_reflective_planes_terminate() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        let mirror = Material { reflective: 1.0, ..Material::default() };
        for y in &[-1.0, 1.0] {
            scene.objects.push(Object::with_material(Box::new(Plane::new(Point::new(0.0, *y, 0.0), Vector::new(0.0, 1.0, 0.0))), Color::new(255, 255, 255), mirror));
        }
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        assert_eq!(tracer(scene).trace(&ray, 8), LinearColor::black());
    }

    #[test]
    fn transparent_sphere_transmits_an_object_behind_it() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255), Material::glass(1.0)));
        scene.objects.push(Object::with_material(Box::new(Plane::new(Point::new(0.0, 0.0, 3.0), Vector::new(0.0, 0.0, -1.0))), Color::new(255, 0, 0), Material::unlit()));
        assert_eq!(tracer(scene).trace(&front_ray(), 4).to_color(), Color::new(255, 0, 0));
    }

    #[test]
    fn glass_keeps_fresnel_reflection_and_transmission_in_balance() {
        let mut scene = Scene::new(Color::new(0, 0, 255));
        scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255), Material::glass(1.5)));
        scene.objects.push(Object::with_material(Box::new(Plane::new(Point::new(0.0, 0.0, 3.0), Vector::new(0.0, 0.0, -1.0))), Color::new(255, 0, 0), Material::unlit()));
        let c = tracer(scene).trace(&front_ray(), 2);
        // At normal incidence, each interface reflects 4% and transmits 96%.
        assert!((c.r - 0.96 * 0.96).abs() < 1.0e-10);
        assert!((c.b - 0.04).abs() < 1.0e-10);
        assert_eq!(c.g, 0.0);
    }

    #[test]
    fn total_internal_reflection_traces_the_reflected_ray() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 255, 255), Material::glass(1.5)));
        // From this point the upward ray exceeds the glass-air critical angle.
        let ray = Ray::new(Point::new(0.0, 0.0, (0.5f64).sqrt()), Vector::new(0.0, 1.0, 0.0));
        // The reflected direction at 45 degrees travels along -z; use a target there.
        scene.objects.push(Object::with_material(Box::new(Plane::new(Point::new(0.0, 0.0, -0.5), Vector::new(0.0, 0.0, 1.0))), Color::new(255, 0, 0), Material::unlit()));
        assert_eq!(tracer(scene).trace(&ray, 1).to_color(), Color::new(255, 0, 0));
    }

    #[test]
    fn snells_law_bends_toward_normal_and_detects_total_internal_reflection() {
        let k = (0.5f64).sqrt();
        let normal = Vector::new(0.0, 1.0, 0.0);
        let incoming = Vector::new(k, -k, 0.0);
        let direction = refract(&incoming, &normal, 1.0, 1.5).unwrap();
        assert!((direction.dx - k / 1.5).abs() < 1.0e-10);
        assert!((direction.magnitude() - 1.0).abs() < 1.0e-10);
        assert!(refract(&incoming, &normal, 1.5, 1.0).is_none());
    }

    #[test]
    fn fresnel_handles_perpendicular_grazing_and_internal_reflection() {
        assert!((schlick(1.0, 1.0, 1.5) - 0.04).abs() < 1.0e-10);
        assert!((schlick(0.0, 1.0, 1.5) - 1.0).abs() < 1.0e-10);
        assert_eq!(schlick((0.5f64).sqrt(), 1.5, 1.0), 1.0);
        assert_eq!(schlick(0.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn supersampling_blends_edges_before_srgb_conversion() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        let material = Material { pattern: Some(Pattern::Stripes(Color::new(255, 255, 255), Color::new(0, 0, 0), 10.0)), ..Material::unlit() };
        scene.objects.push(Object::with_material(Box::new(Plane::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0))), Color::new(255, 255, 255), material));
        let raytracer = tracer(scene);
        let settings = RenderSettings { samples_per_axis: 2, max_depth: 0 };
        assert_eq!(raytracer.sampled_pixel(0, 0, &settings), Color::new(188, 188, 188));
        let image = raytracer.render(&settings);
        assert_eq!(image.dimensions(), (1, 1));
        assert_eq!(*image.get_pixel(0, 0), ::image::Rgb([188, 188, 188]));
    }
}
