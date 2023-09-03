mod device;
mod psu;
mod encode;

extern crate rand;
extern crate libusb;
extern crate crc;

use std::env;
use std::u8;
use crate::psu::Psu;

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

#[derive(Clone, Copy)]
pub struct Config {
    vendor_id: u16,
    product_id: u16,
    print_endpoints: bool,
    print_status: bool,
    fan_percent: Option<u8>
}

impl Config {
    pub fn default() -> Config {
        Config {
            vendor_id: 0x1b1c,
            product_id: 0x1c11,
            print_endpoints: false,
            print_status: false,
            fan_percent: None
        }
    }
}

fn main() {
    let mut config = Config::default();

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        config.fan_percent = Some(args.get(1).unwrap().parse::<u8>().unwrap());
    }

    let context = libusb::Context::new().unwrap();
    let mut device = Psu::setup(&context, config.clone()).unwrap();
    device.setup_dongle();
    device.print_status();

   if config.fan_percent.is_some() {
       device.set_fan_mode(1);
       device.set_fan_speed_percent(config.fan_percent.unwrap());
   }
   else {
       device.set_fan_mode(0);
   }

    device.release();

}


