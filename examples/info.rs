use vjoy::{Interface};

fn main() -> Result<(), vjoy::Error> {
    let vjoy = Interface::new()?;
    let versions = vjoy.versions();

    println!("vJoy SDK: v{}", versions.sdk_version());
    println!("vJoy Interface: v{}", versions.interface_version()?);
    println!("vJoy Driver: v{}", versions.driver_version()?);

    println!("Manufacturer: {}", vjoy.device_manufacturer().as_deref().unwrap_or("(not valid UTF-16)"));
    println!("Product: {}", vjoy.device_product().as_deref().unwrap_or("(not valid UTF-16)"));
    println!("Serial Number: {}", vjoy.device_serial().as_deref().unwrap_or("(not valid UTF-16)"));

    let num_devices = vjoy.num_devices()?;
    let num_slots = vjoy.num_slots()?;
    println!("Devices: {}/{}", num_devices, num_slots);

    for device in vjoy.device_slots()? {
        if !device.is_available() {
            continue;
        }

        println!("vJoy Device #{}:", device.id());
        println!("  Status: {:?}", device.status());

        let num_buttons = device.num_buttons()?;
        println!("  Buttons: {}", num_buttons);

        let num_cpov = device.num_cont_pov()?;
        let num_dpov = device.num_disc_pov()?;
        println!("  POVs: {} discrete, {} continuous", num_dpov, num_cpov);

        for axis in device.axes() {
			let range = device.axis_range(axis)?;
            println!("  Axis #{} ({}): {} to {}", axis as u8, axis.name(), range.start(), range.end());
        }
    }

	Ok(())
}
