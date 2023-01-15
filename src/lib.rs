mod device;
mod interface;
mod lock;
mod util;
mod version;

pub use vjoy_sys as sys;
pub use crate::device::*;
pub use crate::interface::*;
pub use crate::version::*;

use crate::lock::{VJoyLock};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
#[error(transparent)]
pub enum Error {
	DeviceSlot(#[from] DeviceSlotError),
	DeviceSlots(#[from] DeviceSlotsError),
	NewInterface(#[from] NewInterfaceError),
	NumDevices(#[from] NumDevicesError),
	NumSlots(#[from] NumSlotsError),

	Apply(#[from] ApplyError),
	AxisRange(#[from] AxisRangeError),
	DeviceIdFromIndex(#[from] DeviceIdFromIndexError),
	DeviceIdFromRaw(#[from] DeviceIdFromRawError),
	GetAxis(#[from] GetAxisError),
	NumButtons(#[from] NumButtonsError),
	NumContPov(#[from] NumContPovError),
	NumDiscPov(#[from] NumDiscPovError),
	SetAxis(#[from] SetAxisError),
	SetButton(#[from] SetButtonError),
	TryIntoDeviceId(#[from] TryIntoDeviceIdError),

	DriverVersion(#[from] DriverVersionError),
	InterfaceVersion(#[from] InterfaceVersionError),
}
