use super::color::Color;
use super::point::Point;
use super::color::LinearColor;
use super::noise::Perlin;
use super::vector::Vector;

/// Procedural textures are evaluated in the shape's local coordinate system.
#[derive(Clone, Copy, Debug)]
pub enum Pattern {
    /// A checkerboard in the local x/z plane (stable on a horizontal floor).
    Checkers(Color, Color, f64),
    /// Alternating cubes, useful for solid objects rather than a plane boundary.
    Checkers3D(Color, Color, f64),
    Stripes(Color, Color, f64),
    Marble(Color, Color, f64),
    Wood(Color, Color, f64),
    Stone(Color, Color, f64)
}

impl Pattern {
    pub fn color_at(&self, point: &Point) -> Color {
        self.linear_at(point).to_color()
    }

    pub fn linear_at(&self, point: &Point) -> LinearColor {
        let procedural = match *self {
            Pattern::Marble(a,b,s) | Pattern::Wood(a,b,s) | Pattern::Stone(a,b,s) => Some((a,b,s)),
            _ => None
        };
        if let Some((a,b,size)) = procedural {
            assert!(size>0.0 && size.is_finite());
            let p=Vector::new(point.x,point.y,point.z).scale(1.0/size);
            let n=Perlin::new(17);
            let t=match *self {
                Pattern::Marble(_,_,_) => (0.5+0.5*(p.dx*3.0+p.dy+7.0*n.turbulence(p,5)).sin()).powf(7.0),
                Pattern::Wood(_,_,_) => 0.5+0.5*((p.dx*p.dx+p.dz*p.dz).sqrt()*12.0+5.0*n.turbulence(p,4)).sin(),
                _ => (0.5+0.9*n.fbm(p.scale(3.0),5)).max(0.0).min(1.0)
            };
            return a.linear()*(1.0-t)+b.linear()*t;
        }
        let (a, b, size, parity) = match *self {
            Pattern::Checkers(a, b, size) => (a, b, size, (point.x / size).floor() + (point.z / size).floor()),
            Pattern::Checkers3D(a, b, size) => {
                (a, b, size, (point.x / size).floor() + (point.y / size).floor() + (point.z / size).floor())
            },
            Pattern::Stripes(a, b, size) => (a, b, size, (point.x / size).floor()),
            _ => unreachable!()
        };
        assert!(size > 0.0 && size.is_finite(), "Pattern size must be positive and finite");
        if parity % 2.0 == 0.0 { a.linear() } else { b.linear() }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Bump { pub scale: f64, pub strength: f64, pub seed: u32 }

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub ambient: f64,
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
    /// Fraction of light reflected as a perfect mirror, in [0, 1].
    pub reflective: f64,
    /// Fraction transmitted through the remaining surface, in [0, 1].
    pub transparency: f64,
    pub refractive_index: f64,
    pub pattern: Option<Pattern>,
    pub bump: Option<Bump>,
    pub roughness: f64,
    pub transmission_roughness: f64
}

impl Default for Material {
    fn default() -> Material {
        Material {
            ambient: 0.08,
            diffuse: 0.8,
            specular: 0.4,
            shininess: 80.0,
            reflective: 0.0,
            transparency: 0.0,
            refractive_index: 1.0,
            pattern: None, bump: None, roughness: 0.0, transmission_roughness: 0.0
        }
    }
}

impl Material {
    /// The original renderer's flat object color, independent of lights.
    pub fn unlit() -> Material {
        Material { ambient: 1.0, diffuse: 0.0, specular: 0.0, ..Material::default() }
    }

    pub fn glass(refractive_index: f64) -> Material {
        Material {
            ambient: 0.0, diffuse: 0.0, specular: 0.9, shininess: 200.0,
            transparency: 1.0, refractive_index: refractive_index,
            ..Material::default()
        }
    }

    pub fn validate(&self) {
        for coefficient in &[self.ambient, self.diffuse, self.specular, self.reflective, self.transparency, self.roughness, self.transmission_roughness] {
            assert!(coefficient.is_finite() && *coefficient >= 0.0 && *coefficient <= 1.0,
                    "Material coefficients must be between zero and one");
        }
        assert!(self.shininess.is_finite() && self.shininess >= 1.0);
        assert!(self.refractive_index.is_finite() && self.refractive_index > 0.0);
        if let Some(bump)=self.bump { assert!(bump.scale>0.0 && bump.scale.is_finite() && bump.strength>=0.0 && bump.strength.is_finite()); }
        if let Some(pattern) = self.pattern {
            pattern.color_at(&Point::new(0.0, 0.0, 0.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Material, Pattern};
    use super::super::color::Color;
    use super::super::point::Point;

    #[test]
    fn patterns_repeat_across_negative_coordinates() {
        let white = Color::new(255, 255, 255);
        let black = Color::new(0, 0, 0);
        let checkers = Pattern::Checkers(white, black, 1.0);
        assert_eq!(checkers.color_at(&Point::new(0.99, 0.0, 0.0)), white);
        assert_eq!(checkers.color_at(&Point::new(1.01, 0.0, 0.0)), black);
        assert_eq!(checkers.color_at(&Point::new(-0.01, 0.0, 0.0)), black);
        assert_eq!(checkers.color_at(&Point::new(1.01, 0.0, 1.01)), white);
        assert_eq!(checkers.color_at(&Point::new(0.5, -1.0e-12, 0.5)), white);
        assert_eq!(checkers.color_at(&Point::new(0.5, 1.0e-12, 0.5)), white);
        let cubes = Pattern::Checkers3D(white, black, 1.0);
        assert_eq!(cubes.color_at(&Point::new(0.5, 1.01, 0.5)), black);
        let stripes = Pattern::Stripes(white, black, 0.5);
        assert_eq!(stripes.color_at(&Point::new(-0.1, 4.0, 7.0)), black);
        assert_eq!(stripes.color_at(&Point::new(1.1, 4.0, 7.0)), white);
    }

    #[test]
    #[should_panic]
    fn rejects_invalid_refractive_index() {
        Material::glass(0.0).validate();
    }

    #[test]
    fn noise_materials_vary_in_three_dimensions_and_stay_in_color_range() {
        let a=Color::new(30,50,80);let b=Color::new(230,200,170);
        for pattern in &[Pattern::Marble(a,b,0.5),Pattern::Wood(a,b,0.5),Pattern::Stone(a,b,0.5)] {
            let p=Point::new(0.2,0.3,0.4);let q=Point::new(1.31,-0.7,0.89);
            let x=pattern.linear_at(&p);let y=pattern.linear_at(&q);
            assert_eq!(x,pattern.linear_at(&p));assert!(x!=y);
            assert!(x.r>=a.linear().r && x.r<=b.linear().r);
        }
    }
}
