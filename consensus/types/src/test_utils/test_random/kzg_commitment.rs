use super::*;

#[cfg(feature = "kzg")]
impl TestRandom for KzgCommitment {
    fn random_for_test(rng: &mut impl rand::RngCore) -> Self {
        KzgCommitment(<[u8; 48] as TestRandom>::random_for_test(rng))
    }
}

#[cfg(not(feature = "kzg"))]
impl TestRandom for KzgCommitment {
    fn random_for_test(rng: &mut impl rand::RngCore) -> Self {
        Self {
            bytes: <[u8; 48] as TestRandom>::random_for_test(rng),
        }
    }
}
