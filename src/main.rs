extern crate rand;
extern crate libusb;
extern crate crc;

use std::env;
use std::str;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs;
use std::fs::File;
use std::time::Duration;
use std::path::PathBuf;
use std::sync::Arc;
use std::u8;
use rand::Rng;
use crc::{crc16, Hasher16};
use std::thread;

struct UsbController<'a> {
    handle: libusb::DeviceHandle<'a>,
    interface: u8,
    read_address: u8,
    write_address: u8,
    print_messages: bool
}

impl<'a> UsbController<'a> {
    fn open(device: &'a libusb::Device, print_messages: bool) -> UsbController<'a> {

        let mut selected_interface = 0x00;
        let mut selected_read_address = 0x82;
        let mut selected_write_address = 0x02;

        let config = device.active_config_descriptor().unwrap();
        for interface in config.interfaces() {
            selected_interface = interface.number();
            for descriptor in interface.descriptors() {
                for endpoint in descriptor.endpoint_descriptors() {
                    if endpoint.direction() == libusb::Direction::In {
                        selected_read_address = endpoint.address();
                    }
                    else {
                        selected_write_address = endpoint.address();
                    }
                }
            }
        }

        //println!("Opening interface 0x{:02x}", selected_interface);
        //println!("Read address 0x{:02x}", selected_read_address);
        //println!("Write address 0x{:02x}", selected_write_address);

        return UsbController {
            handle: device.open().unwrap(),
            interface: selected_interface,
            read_address: selected_read_address,
            write_address: selected_write_address,
            print_messages: print_messages
        }
    }

    fn claim(&mut self) {
        self.handle.detach_kernel_driver(self.interface);
        self.handle.claim_interface(self.interface);
    }

    fn startup(&mut self) {
        self.handle.reset();
        let empty: [u8; 0] = [];
        self.handle.write_control(0x40, 0x02, 0x0002, 0, &empty, Duration::from_secs(1)).expect("Control failed");
    }

    fn setup_dongle(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        let msg1: [u8; 7] = [
            0x11, 0x02, 0x64, 0x00, 0x00, 0x00, 0x00,
        ];
        self.write_encoded(0, &msg1, &mut out);
    }

    fn print_name(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        let msg7: [u8; 1] = [
            0x02
        ];
        let newsize = self.write_encoded(0, &msg7, &mut out);
        let s = String::from_utf8_lossy(&out[0..newsize]);
        println!("Driver {}", s);
    }

    fn print_version(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        let msg7: [u8; 1] = [
            0x00
        ];
        self.write_encoded(0, &msg7, &mut out);
    }

    fn read_data_psu(&mut self, len: u8, reg: u8, mut out: &mut [u8]) -> usize {
        let msg2: [u8; 7] = [
            0x13, 0x03, 0x06, 0x01, 0x07, len, reg,
        ];
        self.write_encoded(0, &msg2, &mut out);
        let msg3: [u8; 1] = [
            0x12
        ];
        self.write_encoded(0, &msg3, &mut out);
        let msg4: [u8; 3] = [
            0x08, 0x07, len
        ];
        return self.write_encoded(0, &msg4, &mut out);
    }

    fn write_data_psu(&mut self, len: u8, reg: u8, data: &[u8], mut out: &mut [u8]) -> usize {
        let mut out: [u8; 4096] = [0; 4096];
        let header: [u8; 5] = [
            0x13, 0x01, 0x04, len, reg,
        ];
        let join: Vec<u8> = header.iter().chain(data.iter()).cloned().collect();
        self.write_encoded(0, join.as_slice(), &mut out);
        let msg4: [u8; 1] = [
            0x12
        ];
        return self.write_encoded(0, &msg4, &mut out);
    }

    fn print_device(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        let newsize = self.read_data_psu(0x07, 0x9a, &mut out);
        let s = String::from_utf8_lossy(&out[0..newsize]);
        println!("Device {}", s);
    }

    fn unknown_1(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        self.read_data_psu(0x02, 0x8c, &mut out);
    }

    fn print_temp(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        self.read_data_psu(0x02, 0x8e, &mut out);
        println!("Temp {}", UsbController::convert_byte_float(&out));
    }

    fn print_fan_mode(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        self.read_data_psu(0x01, 0xf0, &mut out);
        println!("Fan mode {:x}", out[0]);
    }

    fn set_fan_mode(&mut self, mode: u8) {
        println!("Setting fan mode");
        let mut out: [u8; 4096] = [0; 4096];
        let mode: [u8; 1] = [mode];
        self.write_data_psu(0x01, 0xf0, &mode, &mut out);
    }

    fn print_fan_speed(&mut self) {
        let mut out: [u8; 4096] = [0; 4096];
        self.read_data_psu(0x02, 0x90, &mut out);
        println!("Fan speed {}", UsbController::convert_byte_float(&out));
    }

    fn set_fan_speed_percent(&mut self, speed: u8) {
        let mut out: [u8; 4096] = [0; 4096];
        let percent: [u8; 1] = [speed];
        self.write_data_psu(0x01, 0xe7, &percent, &mut out); //0x3b
    }

    fn test(&mut self) {

        self.print_name();
        self.setup_dongle();
        self.print_device();

        // 0 is auto, 1 is manual
        self.set_fan_mode(1);
        self.set_fan_speed_percent(40);

        self.unknown_1();
        self.print_temp();
        self.print_fan_mode();
        self.print_fan_speed();

    }

    fn decode(msg: &[u8], size: usize, mut out: &mut [u8]) -> usize {

        let decode_table: [u8; 256] = [
            0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
            0x20, 0x21, 0x00, 0x12, 0x22, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x24,
            0x25, 0x00, 0x16, 0x26, 0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x28, 0x29, 0x00, 0x1a,
            0x2a, 0x2b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0x2c, 0x2d, 0x00, 0x1e, 0x2e,
            0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00
        ];

        let newsize = (size/2);
        if (((decode_table[msg[0] as usize] & 0xf) >> 1) != 7) {
            println!("decode_answer: wrong reply data: {} (data {})\n", ((decode_table[msg[0] as usize] & 0xf) >> 1), msg[0]);
            //return;
        }
        let mut i = 1;
        let mut j = 0;
        while i <= size {
            out[j] = (decode_table[msg[i] as usize] & 0xf) | ((decode_table[msg[i + 1] as usize] & 0xf) << 4);
            i += 2;
            j += 1;
        }
        return newsize;
    }

    fn print_array(buf: &[u8], size: usize) {
        print!("{:03} bytes: [", size);
        for b in 0..size {
            print!("{:x}, ", buf[b]);
        }
        println!("]");
    }

    fn write_encoded(&mut self, command: usize, msg: &[u8], mut out: &mut [u8]) -> usize {
        let size = msg.len();
        let encode_table: [u8; 16] = [
            0x55, 0x56, 0x59, 0x5a, 0x65, 0x66, 0x69, 0x6a, 0x95, 0x96, 0x99, 0x9a, 0xa5, 0xa6, 0xa9, 0xaa
        ];

        let mut ret: [u8; 4096] = [0; 4096];

        let newsize = (size * 2) + 2;

        ret[0] = encode_table[(command << 1) & 0xf] & 0xfc;
        ret[newsize - 1] = 0;

        let mut i = 1;
        let mut j = 1;
        while i <= size {
            ret[j] = encode_table[(msg[i - 1] & 0xf) as usize];
            j += 1;

            ret[j] = encode_table[(msg[i - 1] >> 4) as usize];
            j += 1;

            i += 1;
        }
        let result1 = self.handle.write_bulk(self.write_address, &ret[0..newsize], Duration::from_secs(4)).unwrap();
        //println!("write result {}", result1);
        //UsbController::print_array(&ret[0..newsize], newsize);
        return self.read_and_decode(&mut out);
    }

    fn read_and_decode(&mut self, mut out: &mut [u8]) -> usize {
        let mut resp: [u8; 4096] = [0; 4096];
        let mut result = self.handle.read_bulk(self.read_address, &mut resp, Duration::from_secs(1)).unwrap();
        if resp[result - 1] != 0 {
            result += self.handle.read_bulk(self.read_address, &mut resp[result..], Duration::from_secs(1)).unwrap();
        }
        return UsbController::decode(&resp, result, &mut out);
    }

    fn convert_byte_float(data: &[u8]) -> f64 {
        let mut p1 = ((data[1] as i32) >> 3) & 31;
        if (p1 > 15) {
            p1 -= 32;
        }
        let mut p2 = ((data[1] as i32) & 7) * 256 + (data[0] as i32);
        if (p2 > 1024) {
            p2 = -(65536 - (p2 | 63488));
        }
        let base = 2.0 as f64;
        return (p2 as f64) * base.powf(p1 as f64);
    }

    fn release(&mut self) {
        self.handle.release_interface(self.interface);
    }

}


fn print_endpoint(endpoint: libusb::EndpointDescriptor) {
    println!("Endpoint address {:02x}", endpoint.address());
    println!("Endpoint number {:02x}", endpoint.number());
    println!("Endpoint direction {:?}", endpoint.direction());
    println!("Endpoint transfer {:?}", endpoint.transfer_type());
    println!("Endpoint sync {:?}", endpoint.sync_type());
    println!("Endpoint usage {:?}", endpoint.usage_type());
    println!("Endpoint packet size {}", endpoint.max_packet_size());
}


fn print_device(device: &libusb::Device) {
    let device_desc = device.device_descriptor().unwrap();
    println!("Bus {:03} Device {:03} ID {:04x}:{:04x}",
             device.bus_number(),
             device.address(),
             device_desc.vendor_id(),
             device_desc.product_id());

    let config = device.active_config_descriptor().unwrap();
    println!("Number {}, Interfaces {}", config.number(), config.num_interfaces());

    for interface in config.interfaces() {
        println!("Interface {:04x}", interface.number());
        for descriptor in interface.descriptors() {
            println!("Endpoints {}", descriptor.num_endpoints());
            for endpoint in descriptor.endpoint_descriptors() {
                print_endpoint(endpoint);
            }
        }
    }
}

struct Config {
    vendor_id: u16,
    product_id: u16,
    print_endpoints: bool,
    print_status: bool
}

impl Config {
    pub fn default() -> Config {
        Config {
            vendor_id: 0x1b1c,
            product_id: 0x1c11,
            print_endpoints: false,
            print_status: false
        }
    }
}

/**
 * Set selected device to some mode.
 */
fn select_device(device: libusb::Device, config: &Config) {
    let mut controller = UsbController::open(&device, config.print_endpoints);

    if config.print_endpoints {
        print_device(&device);
    }

    controller.claim();

    controller.startup();
    controller.test();

    controller.release();
}


fn main() {
    let mut config = Config::default();
    let mut context = libusb::Context::new().unwrap();
    for mut device in context.devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        if device_desc.vendor_id() == config.vendor_id && device_desc.product_id() == config.product_id {
            select_device(device, &config);
        }
    }
}

