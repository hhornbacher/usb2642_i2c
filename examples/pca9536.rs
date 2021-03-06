#[macro_use]
extern crate bitflags;

use std::{thread::sleep, time::Duration};

use num_derive::ToPrimitive;
use num_traits::ToPrimitive;

use usb2642_i2c::USB2642I2C;

const I2C_ADDRESS: u8 = 0x41;

#[derive(ToPrimitive, Debug)]
pub enum Register {
    InputPort = 0,
    OutputPort = 1,
    Polarity = 2,
    Configuration = 3,
}

bitflags! {
    pub flags GpioPin: u8 {
        const GPIO_NONE = 0x00,
        const GPIO0 = 0x01,
        const GPIO1 = 0x02,
        const GPIO2 = 0x04,
        const GPIO3 = 0x08,
        const GPIO_ALL = 0x0f,
    }
}

#[derive(ToPrimitive, Debug)]
pub enum Direction {
    Output = 0,
    Input = 1,
}

pub struct PCA9536 {
    usb2642: USB2642I2C,
    direction_mask: u8,
}

impl PCA9536 {
    pub fn new(usb2642: USB2642I2C) -> Self {
        Self {
            usb2642,
            direction_mask: 0xff,
        }
    }

    fn write_register(&mut self, register: Register, value: u8) {
        let mut data = [register.to_u8().unwrap(), value];
        self.usb2642.write(I2C_ADDRESS, &mut data).unwrap();
    }

    pub fn read_register(&mut self, register: Register) -> u8 {
        let data = [register.to_u8().unwrap()];
        let data = self.usb2642.write_read(I2C_ADDRESS, &data, 1).unwrap();
        data[0]
    }

    pub fn set_pins_direction(&mut self, pins: GpioPin, direction: Direction) {
        match direction {
            Direction::Output => {
                self.direction_mask &= !pins.bits;
            }
            Direction::Input => {
                self.direction_mask &= pins.bits;
            }
        }
        self.write_register(Register::Configuration, self.direction_mask);
    }

    pub fn output_values(&mut self, pins: GpioPin) {
        self.write_register(Register::OutputPort, pins.bits);
    }
}

fn main() {
    let usb2642 = USB2642I2C::open("/dev/sg0").unwrap();

    let mut pca9536 = PCA9536::new(usb2642);

    pca9536.set_pins_direction(GPIO_ALL, Direction::Output);

    println!(
        "Output port register: {:#02x}",
        pca9536.read_register(Register::OutputPort)
    );
    pca9536.output_values(GPIO_NONE);
    println!(
        "Output port register: {:#02x}",
        pca9536.read_register(Register::OutputPort)
    );
    sleep(Duration::from_secs(2));
    pca9536.output_values(GPIO0 | GPIO2);
    println!(
        "Output port register: {:#02x}",
        pca9536.read_register(Register::OutputPort)
    );
    sleep(Duration::from_secs(2));
    pca9536.output_values(GPIO_ALL);
    println!(
        "Output port register: {:#02x}",
        pca9536.read_register(Register::OutputPort)
    );
}
