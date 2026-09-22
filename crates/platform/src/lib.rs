//! Platform classification only; not a native filesystem or process adapter.
#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevelopmentLane {
    WindowsX64,
    MacosArm64,
    Unsupported,
}

pub fn classify(os: &str, architecture: &str) -> DevelopmentLane {
    match (os, architecture) {
        ("windows", "x86_64") => DevelopmentLane::WindowsX64,
        ("macos", "aarch64") => DevelopmentLane::MacosArm64,
        _ => DevelopmentLane::Unsupported,
    }
}

pub fn current_lane() -> DevelopmentLane {
    classify(std::env::consts::OS, std::env::consts::ARCH)
}

/// Development target recognition does not qualify any installed Desktop build.
pub const fn has_qualified_native_adapter() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intended_lanes_are_recognized() {
        assert_eq!(classify("windows", "x86_64"), DevelopmentLane::WindowsX64);
        assert_eq!(classify("macos", "aarch64"), DevelopmentLane::MacosArm64);
    }

    #[test]
    fn linux_and_wsl_are_not_desktop_support() {
        assert_eq!(classify("linux", "x86_64"), DevelopmentLane::Unsupported);
        assert_eq!(classify("wsl", "x86_64"), DevelopmentLane::Unsupported);
    }

    #[test]
    fn other_architectures_are_not_silently_supported() {
        assert_eq!(classify("windows", "aarch64"), DevelopmentLane::Unsupported);
        assert_eq!(classify("macos", "x86_64"), DevelopmentLane::Unsupported);
        assert!(!has_qualified_native_adapter());
    }
}
