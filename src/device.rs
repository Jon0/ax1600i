use std::sync::{Mutex};
use std::thread::sleep;
use std::time::Duration;
use libusb::{Device, DeviceHandle, Error};

pub struct ClaimedDevice<'a> {
    handle: Mutex<DeviceHandle<'a>>,
    interface: u8,
    read_address: u8,
    write_address: u8,
    debug: bool
}

impl<'a> ClaimedDevice<'a> {
    pub fn claim(device: Device<'a>, interface: u8) -> Result<ClaimedDevice<'a>, Error> {
        let mut handle = device.open().expect("Failed open");
        let read_address = 0x82;
        let write_address = 0x02;
        let result = handle.detach_kernel_driver(interface).is_ok();
        if result {
            println!("Kernel driver detached");
        }
        handle.claim_interface(interface).expect("Claim interface failed");
        handle.reset().expect("Reset device failed");
        Ok(ClaimedDevice { handle: Mutex::new(handle), interface, read_address, write_address, debug: false })
    }

    pub fn write_control(&self) {
        let locked_handle = self.handle.lock().unwrap();
        locked_handle.write_control(0x40, 0x02, 0x0002, 0, &[], Self::get_timeout()).expect("Control failed");
    }


    pub fn read_bulk(&self) -> Vec<u8> {
        let mut resp: [u8; 4096] = [0; 4096];
        let locked_handle = self.handle.lock().unwrap();
        let result = locked_handle.read_bulk(self.read_address, &mut resp, Self::get_timeout()).unwrap();
        return Vec::from(&resp[0..result]);
    }

    pub fn write_bulk(&self, data: &[u8]) -> usize {
        let locked_handle = self.handle.lock().unwrap();
        sleep(Duration::from_millis(5));
        return locked_handle.write_bulk(self.write_address, &data, Self::get_timeout()).unwrap();
    }

    fn get_timeout() -> Duration {
        return Duration::from_secs(4);
    }

    pub fn release(&mut self) {
        let mut locked_handle = self.handle.lock().unwrap();
        locked_handle.release_interface(self.interface).expect("Release interface failed");
    }
}