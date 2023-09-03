use std::thread::sleep;
use std::time::Duration;
use libusb::Context;
use crate::Config;
use crate::device::ClaimedDevice;
use crate::encode::{decode, encode};

pub struct PsuStatus {

}

pub struct Psu<'a> {
    config: Config,
    claimed_device: ClaimedDevice<'a>
}

impl<'a> Psu<'a> {
    pub fn setup(context: &Context, config: Config) -> Option<Psu> {
        for device in context.devices().unwrap().iter() {
            let device_desc = device.device_descriptor().unwrap();
            if device_desc.vendor_id() == config.vendor_id && device_desc.product_id() == config.product_id {
                println!("Found device");
                println!("Device desc {:?}", device_desc);
                let configd = device.active_config_descriptor().unwrap();

                println!("Device config {:?}", configd);
                for interface in configd.interfaces() {
                    println!("Device interface {:?}", interface.number());
                }
                let claimed_device = ClaimedDevice::claim(device, 0x00).expect("Claiming device failed");
                claimed_device.write_control();
                return Some(Psu { config, claimed_device });
            }
        }
        None
    }

    fn write_encoded(&mut self, command: usize, msg: &[u8]) -> Vec<u8> {
        let encoded = encode(command, msg);
        let result1 = self.claimed_device.write_bulk(&encoded);
        if result1 != encoded.len() {
            println!("Failed to write msg");
        }
        return self.read_and_decode();
    }

    fn read_and_decode(&mut self) -> Vec<u8> {
        let mut result = self.claimed_device.read_bulk();
        while result[result.len() - 1] != 0 {
            result.append(&mut self.claimed_device.read_bulk());
        }
        return decode(&result);
    }

    pub fn setup_dongle(&mut self) {
        Self::expect_zero(self.write_encoded(0, &[
            0x11, 0x02, 0x64, 0x00, 0x00, 0x00, 0x00,
        ]));
    }

    fn print_firmware_string(&mut self) {
        let msg: [u8; 1] = [
            0x02
        ];
        let out = self.write_encoded(0, &msg);
        let s = String::from_utf8_lossy(&out);
        println!("Device firmware: {}", s);
    }

    fn print_version(&mut self) {
        let msg: [u8; 1] = [
            0x00
        ];
        let out = self.write_encoded(0, &msg);
        println!("Version: {:?}", out);
    }

    fn read_data_psu(&mut self, len: u8, reg: u8) -> Vec<u8> {
        let header: [u8; 7] = [
            0x13, 0x03, 0x06, 0x01, 0x07, len, reg,
        ];
        Self::expect_zero(self.write_encoded(0, &header));
        Self::expect_ok(self.write_encoded(0, &[0x12]));
        return self.write_encoded(0, &[
            0x08, 0x07, len
        ]);
    }

    fn write_data_psu(&mut self, reg: u8, data: &[u8]) -> Vec<u8> {
        let header: [u8; 5] = [
            0x13, 0x01, 0x04, data.len() as u8, reg,
        ];
        let join: Vec<u8> = header.iter().chain(data.iter()).cloned().collect();
        Self::expect_zero(self.write_encoded(0, join.as_slice()));

        let msg4: [u8; 1] = [
            0x12
        ];
        return self.write_encoded(0, &msg4);
    }

    fn print_device(&mut self) {
        let out = self.read_data_psu(0x07, 0x9a);
        let s = String::from_utf8_lossy(&out);
        println!("Device name: {}", s);
    }

    fn unknown_1(&mut self) {
        self.read_data_psu(0x02, 0x8c);
    }

    fn read_value_at(&mut self, address: u8) {
        for i in 0..16 {
            let out = self.read_data_psu(0x02, address + i);
            println!("Test {} = {:?}, {}", i, &out, convert_byte_float(&out));
        }

        let out = self.read_data_psu(0x07, address);
        println!("Test long = {:?}", &out);
    }

    fn get_f64_register(&mut self, register: u8) -> f64 {
        let out = self.read_data_psu(0x02, register);
        return convert_byte_float(&out)
    }


    fn print_uptime(&mut self) {
        let out = self.read_data_psu(0x02, 0xd2);
        let seconds = (out[0] as i64) + ((out[1] as i64) << 8);
        println!("Uptime {:.2} hours", seconds as f64 / (60.0 * 60.0));
    }

    fn print_temp(&mut self) {
        let out = self.get_f64_register(0x8e);
        println!("Temp 1 {} C", out);
    }

    fn print_temp2(&mut self) {
        let out = self.get_f64_register(0x8d);
        println!("Temp 2 {} C", out);
    }

    fn get_input_voltage(&mut self) -> f64 {
        return self.get_f64_register(0x88);
    }

    fn get_input_current(&mut self) -> f64 {
        return self.get_f64_register(0x89);
    }

    fn get_input_power(&mut self) -> f64 {
        return self.get_f64_register(0xee);
    }


    fn get_rail_voltage(&mut self) -> f64 {
        let out = self.read_data_psu(0x02, 0x8b);
        return convert_byte_float(&out);
    }

    fn get_rail_current(&mut self) -> f64 {
        let out = self.read_data_psu(0x02, 0x8c);
        return convert_byte_float(&out);
    }

    fn misc_rail_power(&mut self) -> f64 {
        let out = self.read_data_psu(0x02, 0x8f);
        return convert_byte_float(&out);
    }

    fn get_rail_watts(&mut self) -> f64 {
        let out = self.read_data_psu(0x02, 0x96);
        return convert_byte_float(&out);
    }

    fn get_12v_rail_current(&mut self) -> f64 {
        let out = self.read_data_psu(0x02, 0xe8);
        return convert_byte_float(&out)
    }

    fn get_12v_rail_power(&mut self) -> f64 {
        let out = self.read_data_psu(0x02, 0xe9);
        return convert_byte_float(&out);
    }

    fn get_12v_rail_ocp_limit(&mut self) -> f64 {
        let out = self.read_data_psu(0x02, 0xea);
        return convert_byte_float(&out);
    }

    fn get_fan_speed(&mut self) -> f64 {
        return self.get_f64_register(0x90);
    }

    fn print_fan_mode(&mut self) {
        let out = self.read_data_psu(0x01, 0xf0);
        println!("Fan mode {:x}", out[0]);
    }

    pub fn set_fan_mode(&mut self, mode: u8) {
        println!("Setting fan mode");
        let mode: [u8; 1] = [mode];
        self.write_data_psu(0xf0, &mode);
    }

    fn print_fan_speed(&mut self) {
        let out = self.read_data_psu(0x02, 0x90);
        println!("Fan speed {} RPM", convert_byte_float(&out));
    }

    pub fn set_fan_speed_percent(&mut self, speed: u8) {
        let percent: [u8; 1] = [speed];
        self.write_data_psu(0x3b, &percent);
    }

    fn set_12v_page(&mut self, page_number: u8) -> u8 {
        let page: [u8; 1] = [page_number];
        self.write_data_psu(0xe7, &page);
        sleep(Duration::from_millis(5));
        let r = self.read_data_psu(0x01, 0xe7);
        if r.len() >= 1 {
            return r[0];
        }
        else {
            return 0;
        }
    }

    fn set_rail(&mut self, page_number: u8) -> u8 {
        let page: [u8; 1] = [page_number];
        self.write_data_psu(0x00, &page);
        sleep(Duration::from_millis(5));
        let r = self.read_data_psu(0x01, 0x00);
        if r.len() >= 1 {
            return r[0];
        }
        else {
            return 0;
        }
    }

    pub fn print_status(&mut self) {

        self.print_device();
        self.print_version();
        self.print_firmware_string();

        // Input values
        let voltage = self.get_input_voltage();
        let current = self.get_input_current();
        let power = self.get_input_power();
        println!("Input: voltage = {} v, current = {} a, power = {} w", voltage, current, power);

        // other rails
        for i in 0..3 {
            self.set_rail(i);
            self.set_12v_page(0);
            let voltage = self.get_rail_voltage();
            let current = self.get_rail_current();
            let power = self.get_rail_watts();
            println!("Rail {}: voltage = {} v, current = {} a, power = {} w", i, voltage, current, power);
        }

        for i in 0..12 {
            self.set_rail(0);
            self.set_12v_page(i);
            let voltage = self.get_rail_voltage();
            let current = self.get_12v_rail_current();
            let power = self.get_12v_rail_power();
            let ocp_limit = self.get_12v_rail_ocp_limit();
            if power > 0.0 {
                println!("12v page {}: voltage = {} v, current = {} a, power = {}, ocp limit = {}", i, voltage, current, power, ocp_limit);
            }
        }

        self.set_rail(0);
        self.print_uptime();
        self.print_temp();
        self.print_temp2();
        let fan_speed = self.get_fan_speed();
        println!("Fan speed {} rpm", fan_speed);

    }

    fn expect_zero(response: Vec<u8>) {
        if response.len() != 1 {
            println!("Unexpected length {}", response.len());
        }
        else if *response.get(0).unwrap() != 0 {
            println!("Error reported {}", response.get(0).unwrap());
        }
    }

    fn expect_ok(response: Vec<u8>) {
        if response.len() != 2 {
            println!("Unexpected length {}", response.len());
        }
        else if *response.get(0).unwrap() != 0 {
            println!("Error reported {}", response.get(0).unwrap());
        }
    }

    pub fn release(&mut self) {
        self.claimed_device.release();
    }
}

fn convert_byte_float(data: &[u8]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }
    let mut p1 = ((data[1] as i32) >> 3) & 31;
    if p1 > 15 {
        p1 -= 32;
    }
    let mut p2 = ((data[1] as i32) & 7) * 256 + (data[0] as i32);
    if p2 > 1024 {
        p2 = -(65536 - (p2 | 63488));
    }
    let base = 2.0 as f64;
    return (p2 as f64) * base.powf(p1 as f64);
}