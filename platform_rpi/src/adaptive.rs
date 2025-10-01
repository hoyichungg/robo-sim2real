pub use r2_core::control::adaptive::map_gain;

#[cfg(test)]
mod tests {
    use super::map_gain;
    #[test]
    fn smoke() {
        assert!((map_gain(0.11, 0.02, 0.2, 0.6, 1.2) - 0.9).abs() < 1e-6);
    }
}
