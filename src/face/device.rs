//! Device selection — §7.4.1.
//!
//! # This file is load-bearing for a reason that is not obvious from its size
//!
//! **`candle_core::Device` is constructed in exactly one function, and it is
//! [`select_device`] below.** A second `Device::Cpu` or `Device::new_cuda`
//! anywhere else in the tree is a review-blocking defect (§19.6.5), because it
//! is what turns the next device change from a one-line edit into a
//! search-and-replace. CI enforces this with a grep — see
//! `scripts/check-device-construction.sh` and §20.10.
//!
//! The second property this file exists to hold: **a GPU that is not there is
//! not an error.** No driver, no card, a busy card, a failed CUDA init — all
//! the same answer, which is the CPU.

use candle_core::Device;

use crate::config::DeviceRequestConfig;

/// What was actually resolved. Reported on `/health` beside what was requested,
/// so that "did the Face layer get the device it asked for?" can be answered
/// without reading logs (§9.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceKind {
    Cpu,
    Cuda { ordinal: usize },
}

impl DeviceKind {
    /// The `/health` `device` field: `"cpu"` or `"cuda:0"`.
    #[must_use]
    pub fn as_health_string(self) -> String {
        match self {
            Self::Cpu => "cpu".to_string(),
            Self::Cuda { ordinal } => format!("cuda:{ordinal}"),
        }
    }

    /// Which `[face.*_profile]` section is live on this device (§7.5.4).
    #[must_use]
    pub fn profile_name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu_profile",
            Self::Cuda { .. } => "cuda_profile",
        }
    }

    #[must_use]
    pub fn is_cuda(self) -> bool {
        matches!(self, Self::Cuda { .. })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceRequest {
    /// Never use the device, even where one exists.
    Cpu,
    /// CUDA where available, CPU otherwise. The default.
    Auto,
    /// CUDA specifically — still falls back rather than failing, because a
    /// commentary layer that refuses to start is worse than a slow one.
    Cuda { ordinal: usize },
}

impl DeviceRequest {
    #[must_use]
    pub fn from_config(request: DeviceRequestConfig, device_index: usize) -> Self {
        match request {
            DeviceRequestConfig::Cpu => Self::Cpu,
            DeviceRequestConfig::Auto => Self::Auto,
            DeviceRequestConfig::Cuda => Self::Cuda {
                ordinal: device_index,
            },
        }
    }

    #[must_use]
    pub fn ordinal(self) -> Option<usize> {
        match self {
            Self::Cuda { ordinal } => Some(ordinal),
            Self::Cpu | Self::Auto => None,
        }
    }

    /// The `/health` `device_requested` field.
    #[must_use]
    pub fn as_health_string(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Auto => "auto",
            Self::Cuda { .. } => "cuda",
        }
    }
}

/// Resolve the inference device, exactly once, at startup.
///
/// The only function in the crate permitted to construct a `Device`.
pub fn select_device(request: DeviceRequest) -> (Device, DeviceKind) {
    match request {
        DeviceRequest::Cpu => (Device::Cpu, DeviceKind::Cpu),

        #[cfg(feature = "cuda")]
        DeviceRequest::Auto | DeviceRequest::Cuda { .. } => {
            let ordinal = request.ordinal().unwrap_or(0);
            match Device::new_cuda(ordinal) {
                Ok(device) => (device, DeviceKind::Cuda { ordinal }),
                // No driver, no card, card busy, CUDA init failed: all the same
                // answer. A GPU that is not there is not an error.
                Err(error) => {
                    tracing::warn!(
                        %error,
                        ordinal,
                        "CUDA requested but unavailable; falling back to CPU. \
                         The cpu_profile model is now the live one — see §7.5.4."
                    );
                    (Device::Cpu, DeviceKind::Cpu)
                }
            }
        }

        // Built without the feature: the request is honoured as far as it can
        // be, and the mismatch is logged once at startup, not per request.
        #[cfg(not(feature = "cuda"))]
        DeviceRequest::Auto | DeviceRequest::Cuda { .. } => {
            if matches!(request, DeviceRequest::Cuda { .. }) {
                tracing::warn!(
                    "face.device requests CUDA but this binary was built without \
                     --features cuda; using CPU"
                );
            }
            (Device::Cpu, DeviceKind::Cpu)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_is_always_honoured() {
        let (_, kind) = select_device(DeviceRequest::Cpu);
        assert_eq!(kind, DeviceKind::Cpu);
    }

    /// §20.10's most important test: with CUDA requested on a machine with no
    /// device, the process resolves to CPU rather than failing. On a CI runner
    /// with no GPU this is the whole assertion; on the target host the resolved
    /// device is CUDA and the assertion is that it did not panic getting there.
    #[test]
    fn requesting_cuda_never_fails() {
        let (_, kind) = select_device(DeviceRequest::Cuda { ordinal: 0 });
        assert!(matches!(kind, DeviceKind::Cpu | DeviceKind::Cuda { .. }));
    }

    #[test]
    fn auto_never_fails() {
        let (_, kind) = select_device(DeviceRequest::Auto);
        assert!(matches!(kind, DeviceKind::Cpu | DeviceKind::Cuda { .. }));
    }

    /// A binary built without the feature cannot resolve to CUDA, whatever the
    /// configuration says.
    #[cfg(not(feature = "cuda"))]
    #[test]
    fn the_default_build_has_no_cuda_path() {
        for request in [DeviceRequest::Auto, DeviceRequest::Cuda { ordinal: 0 }] {
            let (_, kind) = select_device(request);
            assert_eq!(kind, DeviceKind::Cpu);
        }
    }

    #[test]
    fn the_resolved_device_names_its_profile() {
        assert_eq!(DeviceKind::Cpu.profile_name(), "cpu_profile");
        assert_eq!(
            DeviceKind::Cuda { ordinal: 0 }.profile_name(),
            "cuda_profile"
        );
        assert_eq!(DeviceKind::Cuda { ordinal: 1 }.as_health_string(), "cuda:1");
    }
}
