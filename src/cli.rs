use super::raytracer::RenderSettings;

pub const HELP: &'static str = "Usage: raytracer [options]

  --scene demo|original   Scene to render (default: demo)
  --output PATH          PNG output path (default: out.png)
  --width N              Image width, 1..8192 (default: 640)
  --height N             Image height, 1..8192 (default: 480)
  --samples N            N x N antialiasing/lens samples, 1..16 (default: 2)
  --depth N              Reflection/refraction bounces, 0..16 (default: 5)
  --light-samples N      N x N area light samples, 1..64 (default: 3)
  --aperture R           Lens radius; 0 is a pinhole (default: 0)
  --focus D              Focus plane distance in scene units (default: 11)
  -h, --help             Show this help

Use more than one sample per axis with a nonzero aperture for depth of field.
The original scene uses its original, much larger world units.";

pub struct Options {
    pub scene: String,
    pub output: String,
    pub width: u32,
    pub height: u32,
    pub render: RenderSettings,
    pub light_samples: u32,
    pub aperture: f64,
    pub focus: f64
}

impl Default for Options {
    fn default() -> Options {
        Options { scene: "demo".to_string(), output: "out.png".to_string(), width: 640, height: 480,
            render: RenderSettings::default(), light_samples: 3, aperture: 0.0, focus: 11.0 }
    }
}

fn integer(flag: &str, value: &str, min: u32, max: u32) -> Result<u32, String> {
    match value.parse::<u32>() {
        Ok(number) if number >= min && number <= max => Ok(number),
        _ => Err(format!("{} must be an integer between {} and {}", flag, min, max))
    }
}

fn number(flag: &str, value: &str, allow_zero: bool) -> Result<f64, String> {
    match value.parse::<f64>() {
        Ok(number) if number.is_finite() && (number > 0.0 || (allow_zero && number == 0.0)) => Ok(number),
        _ => Err(format!("{} must be a finite {} number", flag, if allow_zero { "nonnegative" } else { "positive" }))
    }
}

impl Options {
    pub fn parse(args: &[String]) -> Result<Option<Options>, String> {
        let mut options = Options::default();
        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            if flag == "--help" || flag == "-h" { return Ok(None); }
            if index + 1 == args.len() { return Err(format!("Missing value for {}", flag)); }
            let value = args[index + 1].as_str();
            match flag {
                "--width" => options.width = try!(integer(flag, value, 1, 8192)),
                "--height" => options.height = try!(integer(flag, value, 1, 8192)),
                "--samples" => options.render.samples_per_axis = try!(integer(flag, value, 1, 16)),
                "--depth" => options.render.max_depth = try!(integer(flag, value, 0, 16)),
                "--light-samples" => options.light_samples = try!(integer(flag, value, 1, 64)),
                "--aperture" => options.aperture = try!(number(flag, value, true)),
                "--focus" => options.focus = try!(number(flag, value, false)),
                "--output" => options.output = value.to_string(),
                "--scene" => options.scene = value.to_string(),
                _ => return Err(format!("Unknown option: {} (use --help)", flag))
            }
            index += 2;
        }
        if options.scene != "demo" && options.scene != "original" {
            return Err("--scene must be demo or original".to_string());
        }
        if !options.output.to_lowercase().ends_with(".png") { return Err("--output must have a .png extension".to_string()); }
        if options.aperture > 0.0 && options.render.samples_per_axis == 1 {
            return Err("Depth of field requires --samples of at least 2".to_string());
        }
        Ok(Some(options))
    }
}

#[cfg(test)]
mod tests {
    use super::Options;

    fn args(words: &[&str]) -> Vec<String> { words.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn parses_render_controls() {
        let options = Options::parse(&args(&["--width", "320", "--samples", "4", "--depth", "0", "--output", "render.png"])).unwrap().unwrap();
        assert_eq!(options.width, 320);
        assert_eq!(options.render.samples_per_axis, 4);
        assert_eq!(options.render.max_depth, 0);
        assert_eq!(options.output, "render.png");
        assert!(Options::parse(&args(&["--help"])).unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_or_missing_arguments() {
        for words in &[vec!["--width", "0"], vec!["--height", "-1"], vec!["--samples", "17"],
            vec!["--depth", "17"], vec!["--focus", "NaN"], vec!["--aperture", "inf"],
            vec!["--scene", "unknown"], vec!["--output", "render.jpg"], vec!["--width"],
            vec!["--bogus", "3"], vec!["--aperture", "0.1", "--samples", "1"]] {
            assert!(Options::parse(&args(words)).is_err(), "unexpected success: {:?}", words);
        }
    }
}
