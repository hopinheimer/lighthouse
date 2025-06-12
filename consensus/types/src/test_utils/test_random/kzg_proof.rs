use super::*;

#[cfg(feature = "kzg")]
use kzg::BYTES_PER_COMMITMENT;

#[cfg(feature = "kzg")]
impl TestRandom for KzgProof {
    fn random_for_test(rng: &mut impl RngCore) -> Self {
        let mut bytes = [0; BYTES_PER_COMMITMENT];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }
}

#[cfg(not(feature = "kzg"))]
impl TestRandom for KzgProof {
    fn random_for_test(rng: &mut impl RngCore) -> Self {
        Self {
            bytes: <[u8; 48] as TestRandom>::random_for_test(rng),
        }
    }
}
