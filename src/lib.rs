#[macro_use]
extern crate nix;

use std::{
    fs::OpenOptions,
    io::Result,
    os::unix::io::{IntoRawFd, RawFd},
};

use nix::{libc::ioctl, sys::ioctl::ioctl_num_type};
use num_derive::ToPrimitive;
use num_traits::ToPrimitive;

pub type I2CAddress = u8;

pub const SG_INTERFACE_ID_ORIG: u8 = 'S' as u8;

pub const SG_IO: u32 = 0x2285;

// Use negative values to flag difference from original sg_header structure
#[derive(ToPrimitive)]
pub enum SgDxfer {
    ToDev = -2,
    FromDev = -3,
}

pub const USB2642_SCSI_OPCODE: u8 = 0xcf;
pub const USB2642_I2C_WRITE_STREAM: u8 = 0x23;
pub const USB2642_I2C_WRITE_READ_STREAM: u8 = 0x22;

pub trait USB2642I2CCommand {}

#[derive(Debug, Default)]
#[repr(C)]
pub struct USB2642I2CWriteReadCommand {
    scsi_vendor_command: u8,
    scsi_vendor_action_write_read_i2c: u8,
    i2c_write_slave_address: u8,
    i2c_read_slave_address: u8,
    i2c_read_data_phase_length_high: u8,
    i2c_read_data_phase_length_low: u8,
    i2c_write_phase_length: u8,
    i2c_write_phase_payload: [u8; 9],
}

impl USB2642I2CWriteReadCommand {
    pub fn new(i2c_addr: u8, write_data: &[u8], read_len: usize) -> Self {
        assert!(read_len < 512);
        assert!(write_data.len() < 9);

        let i2c_write_addr = i2c_addr << 1;
        let i2c_read_addr = i2c_write_addr + 1;

        let mut write_data_buffer = [0u8; 9];

        for (i, b) in write_data.iter().enumerate() {
            write_data_buffer[i] = *b;
        }

        Self {
            scsi_vendor_command: USB2642_SCSI_OPCODE,
            scsi_vendor_action_write_read_i2c: USB2642_I2C_WRITE_READ_STREAM,
            i2c_write_slave_address: i2c_write_addr,
            i2c_read_slave_address: i2c_read_addr,
            i2c_read_data_phase_length_high: ((read_len >> 8) & 0xff) as u8,
            i2c_read_data_phase_length_low: (read_len & 0xff) as u8,
            i2c_write_phase_length: write_data.len() as u8,
            i2c_write_phase_payload: write_data_buffer,
        }
    }
}

impl USB2642I2CCommand for USB2642I2CWriteReadCommand {}

#[derive(Debug, Default)]
#[repr(C)]
pub struct USB2642I2CWriteCommand {
    scsi_vendor_command: u8,
    scsi_vendor_action_write_i2c: u8,
    i2c_slave_address: u8,
    i2c_unused: u8,
    i2c_data_phase_length_high: u8,
    i2c_data_phase_length_low: u8,
    i2c_command_phase_length: u8,
    i2c_command_phase_payload: [u8; 9],
}

impl USB2642I2CWriteCommand {
    pub fn new(i2c_addr: u8, data_len: usize) -> Self {
        assert!(data_len < 512);

        let i2c_write_addr = i2c_addr << 1;

        Self {
            scsi_vendor_command: USB2642_SCSI_OPCODE,
            scsi_vendor_action_write_i2c: USB2642_I2C_WRITE_STREAM,
            i2c_slave_address: i2c_write_addr,
            i2c_unused: 0,
            i2c_data_phase_length_high: ((data_len >> 8) & 0xff) as u8,
            i2c_data_phase_length_low: (data_len & 0xff) as u8,
            i2c_command_phase_length: 0,
            i2c_command_phase_payload: Default::default(),
        }
    }
}
impl USB2642I2CCommand for USB2642I2CWriteCommand {}

#[derive(Debug)]
#[repr(C)]
pub struct SgIoHdr<CMD: USB2642I2CCommand> {
    // [i] 'S' for SCSI generic (required)
    interface_id: i32,
    // [i] data transfer direction
    dxfer_direction: i32,
    // [i] SCSI command length
    cmd_len: u8,
    // [i] max length to write to sbp
    mx_sb_len: u8,
    // [i] 0 implies no scatter gather
    iovec_count: u16,
    // [i] byte count of data transfer
    dxfer_len: u32,
    // [i], [*io] points to data transfer memory or scatter gather list
    dxferp: *mut u8,
    // [i], [*i] points to command to perform
    cmdp: *mut CMD,
    // [i], [*o] points to sense_buffer memory
    sbp: *mut u8,
    // [i] MAX_UINT->no timeout (unit: millisec)
    timeout: u32,
    // [i] 0 -> default, see SG_FLAG...
    flags: u32,
    // [i->o] unused internally (normally)
    pack_id: i32,
    // [i->o] unused internally
    usr_ptr: *const u8,
    // [o] scsi status
    status: u8,
    // [o] shifted, masked scsi status
    masked_status: u8,
    // [o] messaging level data (optional)
    msg_status: u8,
    // [o] byte count actually written to sbp
    sb_len_wr: u8,
    // [o] errors from host adapter
    host_status: u16,
    // [o] errors from software driver
    driver_status: u16,
    // [o] dxfer_len - actual_transferred
    resid: i32,
    // [o] time taken by cmd (unit: millisec)
    duration: u32,
    // [o] auxiliary information
    info: u32,
}

impl<CMD: USB2642I2CCommand> SgIoHdr<CMD> {
    pub fn new(mut command: CMD, sg_dxfer: SgDxfer, data_len: usize, data_buffer: *mut u8) -> Self {
        let mut sense = [0u8; 64];
        Self {
            interface_id: 'S' as i32,
            dxfer_direction: sg_dxfer.to_i32().unwrap(),
            cmd_len: std::mem::size_of::<CMD>() as u8,
            mx_sb_len: sense.len() as u8,
            iovec_count: 0,
            dxfer_len: data_len as u32,
            dxferp: data_buffer,
            cmdp: &mut command,
            sbp: sense.as_mut_ptr(),
            timeout: 3000,
            flags: 0,
            pack_id: 0,
            usr_ptr: std::ptr::null(),
            status: 0,
            masked_status: 0,
            msg_status: 0,
            sb_len_wr: 0,
            host_status: 0,
            driver_status: 0,
            resid: 0,
            duration: 0,
            info: 0,
        }
    }
}

pub fn sg_ioctl<CMD: USB2642I2CCommand>(sg_raw_fd: RawFd, sg_io_hdr: &SgIoHdr<CMD>) -> Result<()> {
    if let Err(e) =
        unsafe { convert_ioctl_res!(ioctl(sg_raw_fd, SG_IO as ioctl_num_type, sg_io_hdr)) }
    {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
    }
    Ok(())
}

pub struct USB2642I2C {
    sg_fd: RawFd,
}

impl USB2642I2C {
    pub fn open<S: Into<String>>(sg_dev: S) -> Result<Self> {
        let sg_fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open(sg_dev.into())?;
        Ok(Self {
            sg_fd: sg_fd.into_raw_fd(),
        })
    }

    pub fn write(&mut self, i2c_addr: I2CAddress, data: &mut [u8]) -> Result<()> {
        let command = USB2642I2CWriteCommand::new(i2c_addr, data.len());
        let sgio = SgIoHdr::new(command, SgDxfer::ToDev, data.len(), data.as_mut_ptr());
        sg_ioctl(self.sg_fd, &sgio)
    }

    pub fn write_read(
        &mut self,
        i2c_addr: I2CAddress,
        write_data: &mut [u8],
        read_len: usize,
    ) -> Result<()> {
        let read_data = [0u8; 512];
        let command = USB2642I2CWriteReadCommand::new(i2c_addr, write_data, read_len);
        let sgio = SgIoHdr::new(
            command,
            SgDxfer::FromDev,
            write_data.len(),
            write_data.as_mut_ptr(),
        );
        sg_ioctl(self.sg_fd, &sgio)
    }
}
