# Linux driver for ax1600i psu


# Build and run:

cargo build

sudo ./target/debug/ax1600i

# Example output

```
Device desc DeviceDescriptor { bLength: 18, bDescriptorType: 1, bcdUSB: 272, bDeviceClass: 0, bDeviceSubClass: 0, bDeviceProtocol: 0, bMaxPacketSize: 64, idVendor: 6940, idProduct: 7185, bcdDevice: 256, iManufacturer: 1, iProduct: 2, iSerialNumber: 3, bNumConfigurations: 1 }
Device config ConfigDescriptor { bLength: 9, bDescriptorType: 2, wTotalLength: 32, bNumInterfaces: 1, bConfigurationValue: 1, iConfiguration: 0, bmAttributes: 128, bMaxPower	: 15 }
Device interface 0
Device name: AX1600i
Version: [0, 9, 0]
Device firmware: USB to SMB Bridge (Firmware by Ross Fosler)
Input: voltage = 239 v, current = 0.46875 a, power = 126 w
Rail 0: voltage = 12.03125 v, current = 7.875 a, power = 88 w
Rail 1: voltage = 4.953125 v, current = 5.125 a, power = 25.5 w
Rail 2: voltage = 3.296875 v, current = 0 a, power = 0 w
12v page 6: voltage = 12.03125 v, current = 2.875 a, power = 35.5, ocp limit = 40.5
12v page 11: voltage = 12.03125 v, current = 4.1875 a, power = 53.5, ocp limit = 40.4375
Uptime 9.76 hours
Temp 1 22.5 C
Temp 2 23.75 C
Fan speed 540 rpm
Setting fan mode
```