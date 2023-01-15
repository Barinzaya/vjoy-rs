use std::string::{FromUtf16Error};

use crate::{DeviceId, DeviceSlot, TryIntoDeviceIdError, Version, Versions, VJoyLock, util, VersionError};

#[derive(Clone, Debug)]
pub struct Interface {
    lock: VJoyLock,
}

impl Interface {
    // TODO: Error type?
    pub fn new() -> Result<Interface, NewInterfaceError> {
        let lock = VJoyLock::new()
            .ok_or(NewInterfaceError::Locked)?;

        let enabled = unsafe { vjoy_sys::vJoyEnabled() } != 0;
        enabled.then_some(Interface { lock })
            .ok_or(NewInterfaceError::NotAvailable)
    }

    pub(crate) fn from_lock(lock: VJoyLock) -> Interface {
        Interface { lock }
    }

    pub fn device_manufacturer(&self) -> Result<String, FromUtf16Error> {
        unsafe {
            let ptr = vjoy_sys::GetvJoyManufacturerString();
            util::decode_utf16(ptr as *const _)
        }
    }

    pub fn device_product(&self) -> Result<String, FromUtf16Error> {
        unsafe {
            let ptr = vjoy_sys::GetvJoyProductString();
            util::decode_utf16(ptr as *const _)
        }
    }

    pub fn device_serial(&self) -> Result<String, FromUtf16Error> {
        unsafe {
            let ptr = vjoy_sys::GetvJoySerialNumberString();
            util::decode_utf16(ptr as *const _)
        }
    }

    pub fn device_slot(&self, id: impl TryInto<DeviceId>) -> Result<Option<DeviceSlot>, DeviceSlotError> {
        if let Ok(id) = id.try_into() {
            if self.num_slots()? >= id.into() {
                return Ok(Some(DeviceSlot::new(id, self.lock.clone())));
            }
        }

        Ok(None)
    }

    pub fn device_slots(&self) -> Result<impl Iterator<Item = DeviceSlot> + DoubleEndedIterator, DeviceSlotsError> {
        let lock = self.lock.clone();
        Ok((0..self.num_slots()?)
            .map(move |id| DeviceSlot::new(DeviceId::from_index(id).unwrap(), lock.clone())))
    }

    pub fn num_devices(&self) -> Result<usize, NumDevicesError> {
        let mut num = 0;
        let success = unsafe { vjoy_sys::GetNumberExistingVJD(&mut num) } != 0;

		success.then_some(num)
			.ok_or(NumDevicesError::Failed)
            .and_then(|n| usize::try_from(n)
                .map_err(|_| NumDevicesError::Invalid))
    }

    #[cfg(feature = "const-slots")]
    pub fn num_slots(&self) -> Result<usize, NumSlotsError> {
        u8::try_from(vjoy_sys::VJOY_MAX_N_DEVICES)
                .map_err(|_| NumSlotsError::Invalid)
                .map(|n| n as usize)
    }

    #[cfg(not(feature = "const-slots"))]
    pub fn num_slots(&self) -> Result<usize, NumSlotsError> {
        let mut num = 0;
        let success = unsafe { vjoy_sys::GetvJoyMaxDevices(&mut num) } != 0;

		success.then_some(num)
            .ok_or(NumSlotsError::Failed)
			.and_then(|n| u8::try_from(n)
                .map_err(|_| NumSlotsError::Invalid)
                .map(|n| n as usize))
    }

    pub fn versions(&self) -> Versions {
        let (mut interface_version, mut driver_version) = (0u16, 0u16);
        unsafe { vjoy_sys::DriverMatch(&mut interface_version, &mut driver_version); }

        Versions {
            driver_version: Version::from_raw(driver_version).ok_or(VersionError::Failed),
            interface_version: Version::from_raw(interface_version).ok_or(VersionError::Failed),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum DeviceSlotError {
    #[error(transparent)]
    Id(#[from] TryIntoDeviceIdError),

    #[error(transparent)]
    MaxDevices(#[from] NumSlotsError),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum DeviceSlotsError {
    #[error(transparent)]
    MaxDevices(#[from] NumSlotsError),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum NewInterfaceError {
    #[error("An instance of the vJoy interface already exists.")]
    Locked,

    #[error("No vJoy driver is available.")]
    NotAvailable,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum NumDevicesError {
    #[error("The vJoy Interface library reported failure in getting the number of existing devices.")]
    Failed,

    #[error("The vJoy Interface library returned an invalid number of existing devices.")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum NumSlotsError {
    #[error("The vJoy Interface library reported failure in getting the number of device slots.")]
    Failed,

    #[error("The vJoy Interface library returned an invalid number of device slots.")]
    Invalid,
}
