use std::cell::RefCell;
use std::fmt::{Display};
use std::num::{NonZeroU8};
use std::ops::{Deref, RangeInclusive};

use crate::interface::{Interface};
use crate::lock::{VJoyLock};

/// A `DeviceId` is a numeric ID representing which slot a vJoy device is in.
///
/// This ID will be an integer starting at 1, and with a standard vJoy driver can span only up to
/// 16. In this library, it is allowed to be as large as 255, but `Interface` will not create
/// `DeviceSlot` instances for device IDs larger than the driver supports.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DeviceId(NonZeroU8);
type RawDeviceId = u8;

impl DeviceId {
	pub fn from_index(index: usize) -> Result<DeviceId, DeviceIdFromIndexError> {
		index.checked_add(1)
			.and_then(|i| i.try_into().ok())
			.and_then(NonZeroU8::new)
			.ok_or(DeviceIdFromIndexError::TooLarge)
			.map(DeviceId)
	}

	pub fn from_raw(raw: RawDeviceId) -> Result<DeviceId, DeviceIdFromRawError> {
		Some(raw)
			.and_then(NonZeroU8::new)
			.ok_or(DeviceIdFromRawError::Zero)
			.map(DeviceId)
	}

	pub fn to_index(self) -> usize {
		self.to_raw() as usize + 1
	}

	pub fn to_raw(self) -> RawDeviceId {
		self.0.get()
	}
}

impl Display for DeviceId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl From<DeviceId> for NonZeroU8 {
	fn from(value: DeviceId) -> Self {
		value.0
	}
}

impl From<DeviceId> for RawDeviceId {
	fn from(value: DeviceId) -> Self {
		value.to_raw()
	}
}

impl From<DeviceId> for usize {
	fn from(value: DeviceId) -> Self {
		value.to_index()
	}
}

impl From<NonZeroU8> for DeviceId {
	fn from(value: NonZeroU8) -> Self {
		DeviceId(value)
	}
}

impl TryFrom<RawDeviceId> for DeviceId {
	type Error = DeviceIdFromRawError;

	fn try_from(raw: RawDeviceId) -> Result<Self, Self::Error> {
		DeviceId::from_raw(raw)
	}
}

impl TryFrom<usize> for DeviceId {
	type Error = DeviceIdFromIndexError;

	fn try_from(i: usize) -> Result<Self, Self::Error> {
		DeviceId::from_index(i)
	}
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeviceSlot {
	id: DeviceId,
	lock: VJoyLock,
}

impl DeviceSlot {
	pub(crate) fn new(id: DeviceId, lock: VJoyLock) -> DeviceSlot {
		DeviceSlot { id, lock }
	}

	pub fn acquire(self) -> Result<OwnedDeviceSlot, DeviceSlot> {
		let acquired = unsafe { vjoy_sys::AcquireVJD(self.id.to_raw() as u32) } != 0;
		if acquired {
			Ok(OwnedDeviceSlot::new(self))
		} else {
			Err(self)
		}
	}

	pub fn axes(&self) -> impl Iterator<Item = Axis> {
		let device = self.clone();

		Axis::all()
			.filter(move |a| device.has_axis(*a))
	}

	#[cfg(feature = "const-range")]
	pub const fn axis_range(&self, _axis: Axis) -> Result<RangeInclusive<i32>, AxisRangeError> {
		Ok(0..=vjoy_sys::VJOY_AXIS_MAX_VALUE as i32)
	}

	#[cfg(not(feature = "const-range"))]
	pub fn axis_range(&self, axis: Axis) -> Result<RangeInclusive<i32>, AxisRangeError> {
		let mut min = 0;
		if unsafe { vjoy_sys::GetVJDAxisMin(self.id.to_raw() as u32, axis.usage(), &mut min) } == 0 {
			return Err(AxisRangeError::MinFailure);
		}

		let mut max = 0;
		if unsafe { vjoy_sys::GetVJDAxisMax(self.id.to_raw() as u32, axis.usage(), &mut max) } == 0 {
			return Err(AxisRangeError::MaxFailure);
		}

		(min <= max)
			.then_some(min..=max)
			.ok_or(AxisRangeError::Invalid)
	}

    pub fn has_axis(&self, axis: Axis) -> bool {
        // TODO: Why does GetVJDAxisExist return true for axes that don't exist?
		//(unsafe { vjoy_sys::GetVJDAxisExist(self.id.to_raw() as u32, axis.usage()) } != 0)

		// Workaround: Use function for axis min, which does fail for axes that don't exist
        let mut min = 0;
		(unsafe { vjoy_sys::GetVJDAxisMin(self.id.to_raw() as u32, axis.usage(), &mut min) } != 0)
	}

	pub fn id(&self) -> DeviceId {
		self.id
	}

	pub fn index(&self) -> usize {
		self.id.into()
	}

	pub fn interface(&self) -> Interface {
		Interface::from_lock(self.lock.clone())
	}

	pub fn is_available(&self) -> bool {
		(unsafe { vjoy_sys::isVJDExists(self.id.to_raw() as u32) } != 0)
	}

	pub fn num_buttons(&self) -> Result<usize, NumButtonsError> {
		let raw = unsafe { vjoy_sys::GetVJDButtonNumber(self.id.to_raw() as u32) };
		raw.try_into().map_err(|_| NumButtonsError::Failed)
	}

	pub fn num_cont_pov(&self) -> Result<usize, NumContPovError> {
		let raw = unsafe { vjoy_sys::GetVJDContPovNumber(self.id.to_raw() as u32) };
		raw.try_into().map_err(|_| NumContPovError::Failed)
	}

	pub fn num_disc_pov(&self) -> Result<usize, NumDiscPovError> {
		let raw = unsafe { vjoy_sys::GetVJDDiscPovNumber(self.id.to_raw() as u32) };
		raw.try_into().map_err(|_| NumDiscPovError::Failed)
	}

	pub fn status(&self) -> Status {
		let raw = unsafe { vjoy_sys::GetVJDStatus(self.id.to_raw() as u32) };
		Status::try_from(raw)
			.expect("vJoy device status received from interface was invalid")
	}
}

#[derive(Debug)]
pub struct OwnedDeviceSlot {
	slot: DeviceSlot,
	state: RefCell<vjoy_sys::JOYSTICK_POSITION>,
}

impl OwnedDeviceSlot {
	fn new(slot: DeviceSlot) -> OwnedDeviceSlot {
		OwnedDeviceSlot {
			state: RefCell::new(vjoy_sys::JOYSTICK_POSITION {
				bDevice: slot.id.try_into()
					.expect("vJoy device ID does not fit into JOYSTICK_POSITION structure"),

				..unsafe { std::mem::zeroed() }
			}),

			slot,
		}
	}

	pub fn get_axis_f32(&self, axis: Axis) -> Result<f32, GetAxisError> {
		let range = self.axis_range(axis)?;
		let raw = self.get_axis_raw(axis);

		if !range.contains(&raw) {
			return Err(GetAxisError::Value);
		}

		let (lo, hi) = range.into_inner();
		let offset = raw.wrapping_sub(lo) as u32;
		let span = hi.wrapping_sub(lo) as u32;

		Ok(offset as f32 / span as f32)
	}

	pub fn get_axis_raw(&self, axis: Axis) -> i32 {
		let state = self.state.borrow();
		match axis {
			Axis::X => state.wAxisX,
			Axis::Y => state.wAxisY,
			Axis::Z => state.wAxisZ,
			Axis::RX => state.wAxisXRot,
			Axis::RY => state.wAxisYRot,
			Axis::RZ => state.wAxisZRot,
			Axis::Slider => state.wSlider,
			Axis::Dial => state.wDial,

			Axis::Accelerator => state.wAccelerator,
			Axis::Aileron => state.wAileron,
			Axis::Brake => state.wBrake,
			Axis::Clutch => state.wClutch,
			Axis::Rudder => state.wRudder,
			Axis::Steering => state.wSteering,
			Axis::Throttle => state.wThrottle,
			Axis::Wheel => state.wWheel,
		}
	}

	pub fn get_button(&self, index: usize) -> Option<bool> {
		let state = self.state.borrow();

		let (word, bit) = match index {
			 0..= 31 => Some((&state.lButtons,    index)),
			32..= 63 => Some((&state.lButtonsEx1, index - 32)),
			64..= 95 => Some((&state.lButtonsEx2, index - 64)),
			96..=127 => Some((&state.lButtonsEx3, index - 96)),
			_ => None,
		}?;

		Some((*word & (1 << bit)) != 0)
	}

	pub fn set_axis_f32(&self, axis: Axis, value: f32) -> Result<(), SetAxisError> {
		if !(0.0..=1.0).contains(&value) {
			return Err(SetAxisError::Value);
		}

		let (lo, hi) = self.axis_range(axis)?.into_inner();
		let span = hi.wrapping_sub(lo) as u32;

		self.set_axis(axis, lo + f32::round(span as f32 * value) as i32);
		Ok(())
	}

	pub fn set_axis_raw(&self, axis: Axis, value: i32) -> Result<(), SetAxisError> {
		let range = self.axis_range(axis)?;
		if !range.contains(&value) {
			return Err(SetAxisError::Value);
		}

		self.set_axis(axis, value);
		Ok(())
	}

	fn set_axis(&self, axis: Axis, value: i32) {
		let mut state = self.state.borrow_mut();
		match axis {
			Axis::X => state.wAxisX = value,
			Axis::Y => state.wAxisY = value,
			Axis::Z => state.wAxisZ = value,
			Axis::RX => state.wAxisXRot = value,
			Axis::RY => state.wAxisYRot = value,
			Axis::RZ => state.wAxisZRot = value,
			Axis::Slider => state.wSlider = value,
			Axis::Dial => state.wDial = value,

			Axis::Accelerator => state.wAccelerator = value,
			Axis::Aileron => state.wAileron = value,
			Axis::Brake => state.wBrake = value,
			Axis::Clutch => state.wClutch = value,
			Axis::Rudder => state.wRudder = value,
			Axis::Steering => state.wSteering = value,
			Axis::Throttle => state.wThrottle = value,
			Axis::Wheel => state.wWheel = value,
		}
	}

	pub fn set_button(&self, index: usize, value: bool) -> Result<(), SetButtonError> {
		let mut state = self.state.borrow_mut();

		let (word, bit) = match index {
			 0..= 31 => Ok((&mut state.lButtons,    index)),
			32..= 63 => Ok((&mut state.lButtonsEx1, index - 32)),
			64..= 95 => Ok((&mut state.lButtonsEx2, index - 64)),
			96..=127 => Ok((&mut state.lButtonsEx3, index - 96)),
			_ => Err(SetButtonError::NoSuchButton),
		}?;

		let mask = 1 << bit;
		*word = if value { *word | mask } else { *word & !mask };
		Ok(())
	}

	pub fn relinquish(self) {}

	pub fn apply(&self) -> Result<(), ApplyError> {
		let state = self.state.borrow();
        let success = unsafe { vjoy_sys::UpdateVJD(self.id.to_raw() as u32, state.deref() as *const _ as *mut _) } != 0;
		success.then_some(()).ok_or(ApplyError::Failed)
	}
}

impl Deref for OwnedDeviceSlot {
	type Target = DeviceSlot;

	fn deref(&self) -> &Self::Target {
		&self.slot
	}
}

impl Drop for OwnedDeviceSlot {
	fn drop(&mut self) {
		unsafe { vjoy_sys::RelinquishVJD(self.id.to_raw() as u32); }
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Axis {
	X,
	Y,
	Z,
	RX,
	RY,
	RZ,
	Slider,
	Dial,

	Accelerator,
	Aileron,
	Brake,
	Clutch,
	Rudder,
	Steering,
	Throttle,
	Wheel,
}

impl Axis {
	pub fn all() -> impl Iterator<Item = Axis > + DoubleEndedIterator + ExactSizeIterator {
		let axes = &[
			Axis::X,
			Axis::Y,
			Axis::Z,
			Axis::RX,
			Axis::RY,
			Axis::RZ,
			Axis::Slider,
			Axis::Dial,

			Axis::Accelerator,
			Axis::Aileron,
			Axis::Brake,
			Axis::Clutch,
			Axis::Rudder,
			Axis::Steering,
			Axis::Throttle,
			Axis::Wheel,
		];

		axes.iter().copied()
	}

	pub fn name(&self) -> &'static str {
		match self {
			Axis::X => "X",
			Axis::Y => "Y",
			Axis::Z => "Z",
			Axis::RX => "RX",
			Axis::RY => "RY",
			Axis::RZ => "RZ",
			Axis::Slider => "Slider",
			Axis::Dial => "Dial",

			Axis::Accelerator => "Accelerator",
			Axis::Aileron => "Aileron",
			Axis::Brake => "Brake",
			Axis::Clutch => "Clutch",
			Axis::Rudder => "Rudder",
			Axis::Steering => "Steering",
			Axis::Throttle => "Throttle",
			Axis::Wheel => "Wheel",
		}
	}

	pub fn usage(&self) -> u32 {
		match self {
			Axis::X => vjoy_sys::HID_USAGE_X,
			Axis::Y => vjoy_sys::HID_USAGE_Y,
			Axis::Z => vjoy_sys::HID_USAGE_Z,
			Axis::RX => vjoy_sys::HID_USAGE_RX,
			Axis::RY => vjoy_sys::HID_USAGE_RY,
			Axis::RZ => vjoy_sys::HID_USAGE_RZ,
			Axis::Slider => vjoy_sys::HID_USAGE_SL0,
			Axis::Dial => vjoy_sys::HID_USAGE_SL1,

			Axis::Accelerator => vjoy_sys::HID_USAGE_ACCELERATOR,
			Axis::Aileron => vjoy_sys::HID_USAGE_AILERON,
			Axis::Brake => vjoy_sys::HID_USAGE_BRAKE,
			Axis::Clutch => vjoy_sys::HID_USAGE_CLUTCH,
			Axis::Rudder => vjoy_sys::HID_USAGE_RUDDER,
			Axis::Steering => vjoy_sys::HID_USAGE_STEERING,
			Axis::Throttle => vjoy_sys::HID_USAGE_THROTTLE,
			Axis::Wheel => vjoy_sys::HID_USAGE_WHL,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Status {
    Free,
	Acquired,
    Busy,
    Missing,
	Unknown, // TODO: Should this just result in an error instead?
}

impl From<Status> for vjoy_sys::VjdStat {
	fn from(status: Status) -> Self {
		match status {
			Status::Free => vjoy_sys::VjdStat_VJD_STAT_FREE,
			Status::Acquired => vjoy_sys::VjdStat_VJD_STAT_OWN,
			Status::Busy => vjoy_sys::VjdStat_VJD_STAT_BUSY,
			Status::Missing => vjoy_sys::VjdStat_VJD_STAT_MISS,
			Status::Unknown => vjoy_sys::VjdStat_VJD_STAT_UNKN,
		}
	}
}

impl TryFrom<vjoy_sys::VjdStat> for Status {
	type Error = ();

	fn try_from(status: vjoy_sys::VjdStat) -> Result<Self, Self::Error> {
		match status {
			vjoy_sys::VjdStat_VJD_STAT_FREE => Ok(Status::Free),
			vjoy_sys::VjdStat_VJD_STAT_OWN => Ok(Status::Acquired),
			vjoy_sys::VjdStat_VJD_STAT_BUSY => Ok(Status::Busy),
			vjoy_sys::VjdStat_VJD_STAT_MISS => Ok(Status::Missing),
			vjoy_sys::VjdStat_VJD_STAT_UNKN => Ok(Status::Unknown),
			_ => Err(()),
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum ApplyError {
	#[error("The vJoy interface return an error in sending the updated device state.")]
	Failed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum AxisRangeError {
	#[error("The vJoy Interface returned an invalid range (min >= max).")]
	Invalid,

	#[error("The vJoy Interface reported an error in retrieving the axis maximum.")]
	MaxFailure,

	#[error("The vJoy Interface reported an error in retrieving the axis minimum.")]
	MinFailure,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum DeviceIdFromIndexError {
	#[error("The index is too large to represent with a DeviceId.")]
	TooLarge,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum DeviceIdFromRawError {
	#[error("DeviceId may not be 0.")]
	Zero,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum GetAxisError {
	#[error(transparent)]
	GetRange(#[from] AxisRangeError),

	#[error("The value for the axis is outside of the allowed range.")]
	Value,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum NumButtonsError {
	#[error("The vJoy Interface returned an error in retrieving the number of buttons for the vJoy device.")]
	Failed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum NumContPovError {
	#[error("The vJoy Interface returned an error in retrieving the number of continuous POVs for the vJoy device.")]
	Failed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum NumDiscPovError {
	#[error("The vJoy Interface returned an error in retrieving the number of discrete POVs for the vJoy device.")]
	Failed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum SetAxisError {
	#[error(transparent)]
	GetRange(#[from] AxisRangeError),

	#[error("The value for the axis is outside of the allowed range.")]
	Value,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum SetButtonError {
	#[error("The vJoy device does not support the specified button.")]
	NoSuchButton,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum TryIntoDeviceIdError {
	#[error(transparent)]
	FromIndex(#[from] DeviceIdFromIndexError),

	#[error(transparent)]
	FromRaw(#[from] DeviceIdFromRawError),
}
