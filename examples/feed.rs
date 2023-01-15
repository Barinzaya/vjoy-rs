use std::time::{Instant, Duration};

use anyhow::{Context as _, Result as AnyResult};
use vjoy::{Interface};

fn main() -> AnyResult<()> {
    let vjoy = Interface::new()?;

	let device = vjoy.device_slots()?
		.filter_map(|d| d.acquire().ok())
		.next()
		.context("Failed to acquire a vJoy device!")?;

	println!("Acquired vJoy device #{}.", device.id());
	let num_buttons = device.num_buttons()?;

	let start = Instant::now();

	let mut next = start;
	let period = Duration::from_nanos(1_000_000_000 / 125);

	let mut time_samples = 0;
	let mut time_total = Duration::ZERO;

	loop {
		let before = Instant::now();
		let t = before.duration_since(start).as_secs_f64();

		let mut speed = 0.5 * std::f64::consts::TAU;
		for axis in device.axes() {
			let value = 0.5 * f64::sin(speed * t) + 0.5;
			speed /= 1.2;

			device.set_axis_f32(axis, value as f32)?;
		}

		let mut interval = 0.1;
		for button in 0..num_buttons {
			let phase = f64::fract(t / interval);
			interval *= 1.1;

			device.set_button(button, phase < 0.5)?;
		}

		time_total += before.elapsed();
		time_samples += 1;

		if time_samples % (125*15) == 0 {
			let time_avg = time_total / time_samples;
			println!("Average update time over {} samples: {:#?}", time_samples, time_avg);
		}

		device.apply()?;
		next += period;

		let now = Instant::now();
		if let Some(pause) = next.checked_duration_since(now) {
			std::thread::sleep(pause);
		}
	}
}
