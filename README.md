# usb2642-i2c

[![API](https://docs.rs/usb2642-i2c/badge.svg)](https://docs.rs/usb2642-i2c)
[![Crate](https://img.shields.io/crates/v/usb2642-i2c.svg)](https://crates.io/crates/usb2642-i2c)

A Rust library for interfacing with the USB2642 I2C bus using the linux sg3 SCSI interface.


## Usage

```rust
let usb2642 = USB2642I2C::open("/dev/sg0").unwrap();

// Write-Only
let mut write_data = [0x01u8, 0x02u8];
usb2642.write(I2C_ADDRESS, &mut data).unwrap();

// Write-Read
let write_data = [register.to_u8().unwrap()];
let read_data = usb2642.write_read(I2C_ADDRESS, &data, 1).unwrap();
```
