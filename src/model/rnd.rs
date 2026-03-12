use rand::Rng;

/// Generate a vector sized size fill with random value
///
/// # Example
/// ```
/// use rdp::model::rnd::random;
/// let vector = random(128);
/// assert_eq!(vector.len(), 128);
/// ```
pub fn random(size: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; size];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}
