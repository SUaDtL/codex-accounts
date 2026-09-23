#![forbid(unsafe_code)]
use crate::CryptoError;
use zeroize::Zeroize;

// Private injection boundary. No public/custom entropy provider or fallback.
pub(crate) trait Entropy {
    fn fill(&mut self, output: &mut [u8]) -> Result<(), CryptoError>;
}
pub(crate) struct SystemEntropy;
impl Entropy for SystemEntropy {
    fn fill(&mut self, output: &mut [u8]) -> Result<(), CryptoError> {
        getrandom::fill(output).map_err(|_| CryptoError::RandomnessUnavailable)
    }
}
pub(crate) fn fill(rng: &mut impl Entropy, output: &mut [u8]) -> Result<(), CryptoError> {
    if rng.fill(output).is_err() {
        output.zeroize();
        return Err(CryptoError::RandomnessUnavailable);
    }
    Ok(())
}
#[cfg(test)]
pub(crate) struct FailingEntropy;
#[cfg(test)]
impl Entropy for FailingEntropy {
    fn fill(&mut self, output: &mut [u8]) -> Result<(), CryptoError> {
        output.fill(0x53);
        Err(CryptoError::RandomnessUnavailable)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_randomness_failure_clears_destination() {
        let mut bytes = [0x41; 32];
        assert_eq!(
            fill(&mut FailingEntropy, &mut bytes),
            Err(CryptoError::RandomnessUnavailable)
        );
        assert!(bytes.iter().all(|b| *b == 0));
    }
}
