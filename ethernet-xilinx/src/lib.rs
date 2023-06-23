
/*!
 # ethernet_xilinx

 This crate provide a struct with many methods to operate ethernet in Xilinx's FPGA: Xxv ethernet

 */


#![no_std]

#[macro_use]
extern crate bitflags;

pub mod xxv_ethernet;

pub use xxv_ethernet::*;
 

