// compute complete u64 with lower address and high address
#[inline]
pub fn compute_u64(lower: u32, high: u32) -> u64 {
    ((high as u64) << 32) | (lower as u64)
}

#[cfg(test)]
mod tests {
    use super::compute_u64;

    #[test]
    fn test_compute_u64() {
        assert_eq!(compute_u64(0x01, 0x01), 0x0000_0001_0000_0001);
        assert_eq!(compute_u64(0x00, 0x01), 0x0000_0001_0000_0000);
        assert_eq!(compute_u64(0x01, 0x00), 0x0000_0000_0000_0001);
    }
}
