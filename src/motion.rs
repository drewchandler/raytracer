use super::vector::Vector;

/// A closed orbit through the noise field, with matching position and velocity
/// at the loop boundary. A zero period preserves ordinary linear wind.
pub fn cloud_offset(time: f64, period: f64, velocity: Vector) -> Vector {
    if period == 0.0 { return velocity.scale(time); }
    let phase = 2.0 * ::std::f64::consts::PI * ((time % period) / period);
    let forward = velocity.normalize();
    let sideways = Vector::new(-forward.dz, 0.0, forward.dx);
    forward.scale(0.35 * phase.sin()) + sideways.scale(0.35 * (1.0 - phase.cos()))
}

#[cfg(test)]
mod tests {
    use super::cloud_offset;
    use super::super::vector::Vector;

    #[test]
    fn loop_closes_without_a_position_or_velocity_jump() {
        let wind = Vector::new(0.08, 0.0, 0.025);
        let p = |t| cloud_offset(t, 2.0, wind);
        assert_eq!(p(0.0), p(2.0));
        assert!((p(0.5) - p(2.5)).magnitude() < 1.0e-12);
        assert!((p(0.5) - p(0.0)).magnitude() > 0.1);
        let e = 0.00001;
        let before = (p(2.0) - p(2.0-e)).scale(1.0/e);
        let after = (p(e) - p(0.0)).scale(1.0/e);
        assert!((before-after).magnitude() < 0.0001);
        assert_eq!(cloud_offset(3.0, 0.0, wind), wind.scale(3.0));
    }
}
