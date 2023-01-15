use std::fmt::{Debug, Display};
use std::num::{NonZeroU16};

#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd)]
pub struct Versions {
    pub(crate) driver_version: Result<Version, VersionError>,
    pub(crate) interface_version: Result<Version, VersionError>,
}

impl Versions {
    pub fn driver_version(&self) -> Result<Version, DriverVersionError> {
        self.driver_version.map_err(Into::into)
    }

    pub fn interface_version(&self) -> Result<Version, InterfaceVersionError> {
        self.interface_version.map_err(Into::into)
    }

    pub fn sdk_version(&self) -> Version {
        static_assertions::const_assert_eq!(vjoy_sys::VERSION_N as u16 as u32, vjoy_sys::VERSION_N);
        static_assertions::const_assert_ne!(vjoy_sys::VERSION_N as u16, 0);

        Version::from_raw(vjoy_sys::VERSION_N as u16)
            .unwrap()
    }
}


#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd)]
pub struct Version(NonZeroU16);

impl Version {
    pub fn from_raw(raw: u16) -> Option<Self> {
        NonZeroU16::new(raw).map(Version)
    }

    pub fn into_raw(self) -> NonZeroU16 {
        self.0
    }

    pub fn major(&self) -> u8 {
        (self.0.get() >> 8) as u8 & 0xf
    }

    pub fn minor(&self) -> u8 {
        (self.0.get() >> 4) as u8 & 0xf
    }

    pub fn patch(&self) -> u8 {
        self.0.get() as u8 & 0xf
    }

    pub fn parts(&self) -> (u8, u8, u8) {
        (self.major(), self.minor(), self.patch())
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version(0x{:04x}/{})", self.0, self)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
#[error("Failed to get driver version number: {}", .0)]
pub struct DriverVersionError(#[from] VersionError);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
#[error("Failed to get interface version number: {}", .0)]
pub struct InterfaceVersionError(#[from] VersionError);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum VersionError {
    #[error("The vJoy interface did not return a version number.")]
    Failed,
}
