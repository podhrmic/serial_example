extern crate serial;

use std::env;
use std::error::Error;
use std::{thread, time};

use std::io::prelude::*;
use serial::prelude::*;
use std::process;
use std::slice;
use std::mem;

const VN_SYNC: u8 = 0xFA;
const VN_OUTPUT_GROUP: u8 = 0x39;
const VN_GROUP_BYTES: u8 = 8;
const VN_HEADER_SIZE: u8 = 10;
const VN_PAYLOAD_SIZE: u8 = 144;
const VN_CRC_SIZE: u8 = 2;
const VN_GROUP_FIELD_1: u16 = 0x01E9;
const VN_GROUP_FIELD_2: u16 = 0x061A;
const VN_GROUP_FIELD_3: u16 = 0x0140;
const VN_GROUP_FIELD_4: u16 = 0x0009;

#[repr(C,packed)]
#[derive(Debug)]
struct VectornavData {
	timestamp: u64,
	ypr: [f32;3],
	angular_rates: [f32;3],
	position: [f64;3],
	velocity: [f32;3],
	accel: [f32;3],
	tow: u64,
	num_sats: u8,
	fix: u8,
	pos_u: [f32;3],
	vel_u: f32,
	lin_accel: [f32;3],
	ypr_u: [f32;3],
	ins_status: u16,
	vel_body: [f32;3],
}


impl VectornavData {
	pub fn new() -> VectornavData {
		VectornavData {
			timestamp: 100_000,
			ypr: [-3.0, -2.0, -1.0],
			angular_rates: [1.0, 2.0, 3.0],
			position: [4.0, 5.0, 6.0],
			velocity: [7.0, 8.0, 9.0],
			accel: [10.0, 11.0, 12.0],
			tow: 12_345_678,
			num_sats: 42,
			fix: 1,
			pos_u: [13.0, 14.0, 15.0],
			vel_u: 16.0,
			lin_accel: [17.0, 18.0, 19.0],
			ypr_u: [20.0, 21.0, 22.0],
			ins_status: 6969,
			vel_body: [23.0, 24.0, 25.0],
		}
	}
	
	pub fn clean() -> VectornavData {
		VectornavData {
			timestamp: 0,
			ypr: [0.0;3],
			angular_rates: [0.0;3],
			position: [0.0;3],
			velocity: [0.0;3],
			accel: [0.0;3],
			tow: 0,
			num_sats: 0,
			fix: 0,
			pos_u: [0.0;3],
			vel_u: 0.0,
			lin_accel: [0.0;3],
			ypr_u: [0.0;3],
			ins_status: 0,
			vel_body: [0.0;3],
		}
	}
	
	pub fn from_slice(slice: &[u8]) -> VectornavData {
	
		let mut arr = [0; VN_PAYLOAD_SIZE as usize];
		arr.copy_from_slice(&slice[(VN_HEADER_SIZE as usize)..(VN_HEADER_SIZE as usize + VN_PAYLOAD_SIZE as usize)]);
		let data = unsafe {
			mem::transmute::<[u8; 144], VectornavData>(arr)
		};

		data
	}
	
	pub fn get_as_ref_u8(&self) -> &[u8] {
		let p: *const VectornavData = self;
		let p: *const u8 = p as *const u8;
		unsafe {
			slice::from_raw_parts(p, mem::size_of::<VectornavData>())
		}
	}
}



#[repr(C)]
#[derive(Debug)]
enum VNMsgStatus {
  VNMsgSync,
  VNMsgHeader,
  VNMsgGroup,
  VNMsgData,
}

#[repr(C)]
struct VNPacket {
	msg_available: bool,
	chksm_err: u32,
	hdr_err: u32,
	status: VNMsgStatus,
	calc_chk: u16,
	rec_chk: u16,
	counter: u16,
	vn_data: VectornavData,
	idx: u8,
	buf: Vec<u8>,
}

impl VNPacket {
	pub fn new() -> VNPacket {
		VNPacket {
			msg_available: false,
			chksm_err: 0,
			hdr_err: 0,
			status: VNMsgStatus::VNMsgSync,
			calc_chk: 0,
			rec_chk: 0,
			counter: 0,
			vn_data: VectornavData::new(),
			idx: 0,
			buf: vec![],
		}
	}
	
	pub fn fill_header(&mut self) {
		self.buf.push(VN_SYNC);
		self.buf.push(VN_OUTPUT_GROUP);
		self.buf.push((VN_GROUP_FIELD_1 >> 8) as u8);
		self.buf.push((VN_GROUP_FIELD_1) as u8);
		self.buf.push((VN_GROUP_FIELD_2 >> 8) as u8);
		self.buf.push((VN_GROUP_FIELD_2) as u8);
		self.buf.push((VN_GROUP_FIELD_3 >> 8) as u8);
		self.buf.push((VN_GROUP_FIELD_3) as u8);
		self.buf.push((VN_GROUP_FIELD_4 >> 8) as u8);
		self.buf.push((VN_GROUP_FIELD_4) as u8);
	}
	
	pub fn fill_data(&mut self) {
		// push VN data to buffer
		for data in self.vn_data.get_as_ref_u8() {
			self.buf.push(*data);
		}
	}
	
	pub fn fill_crc(&mut self) {
		// push crc to buffer
		let (crc0,crc1) = self.calculate_crc();
		self.buf.push(crc0);
		self.buf.push(crc1);
	}
	
	#[allow(dead_code)]
	pub fn print_buffer(&self) {
		println!("buf=[");
		for (index,byte) in self.buf.iter().enumerate() {
			println!("{}: {:X},",index,byte);
		}
		println!("];");
	}
	
	fn verify_checksum(&mut self) -> bool {
		// pop received crc first
		let crc1 = self.buf.pop().unwrap();
		let crc0 = self.buf.pop().unwrap();
		let rec_crc = (crc0 as u16) << 8 | crc1 as u16;
		
		// calculate crc
		let (crc0, crc1) = self.calculate_crc();
		let calc_crc = (crc0 as u16) << 8 | crc1 as u16;
		
		if calc_crc == rec_crc {
			true
		} else {
			false
		}
	}
	
	pub fn parse_data(&mut self, data: &[u8]) {
		for &byte in data {
			match self.status {
				VNMsgStatus::VNMsgSync => {
					if byte == VN_SYNC {
						self.buf.push(byte);
						self.status = VNMsgStatus::VNMsgHeader;
					} else {
						self.hdr_err += 1;
					}
				},
				VNMsgStatus::VNMsgHeader => {
					if byte == VN_OUTPUT_GROUP {
						self.status = VNMsgStatus::VNMsgGroup;
						self.buf.push(byte);
						
					} else {
						self.hdr_err += 1;
						self.status = VNMsgStatus::VNMsgSync;
					}
				},
				VNMsgStatus::VNMsgGroup => {
					self.buf.push(byte);
					if self.buf.len() as u8 == VN_GROUP_BYTES {
						self.status = VNMsgStatus::VNMsgData;
					}
				},
				VNMsgStatus::VNMsgData => {
					self.buf.push(byte);
					if self.buf.len() as u8 == (VN_PAYLOAD_SIZE + VN_HEADER_SIZE + VN_CRC_SIZE) {
						if self.verify_checksum() {
							self.msg_available = true;
							self.counter += 1;
							
						    self.vn_data = VectornavData::from_slice(&self.buf); // create the struct			    						    
							self.buf = vec![]; // erase buffer
							
							
						} else {
							self.msg_available = false;
							self.chksm_err += 1;
						}
						self.status = VNMsgStatus::VNMsgSync;
					}
				},
			}
		}
	}
	
	
	// CRC over buffer content
	pub fn calculate_crc(&self) -> (u8,u8) {
		let mut crc: u16 = 0;

		// get the byte array representation
		for byte in self.buf.iter().skip(1) { // skip first byte (VN_SYNC)
			crc =  (((crc >> 8) as u8) as u16) | (((crc << 8) as u8) as u16);
			crc ^= *byte as u16;
			crc ^= (crc & 0xFF) >> 4 as u8;
			crc ^= crc << 12;
			crc ^= (crc & 0x00ff) << 5;	
		}
		
		((crc >> 8) as u8, crc as u8)
	}
}



fn configure_port(mut port: serial::SystemPort) -> Result<serial::SystemPort, Box<Error>> {
	let settings = serial::core::PortSettings{
		baud_rate: serial::BaudOther(921_600),
		char_size: serial::Bits8,
		parity: serial::ParityNone,
		stop_bits: serial::Stop1,
		flow_control: serial::FlowNone,
	};
	
	port.configure(&settings)?;
	
	Ok(port)
}


// Sender thread
fn thread1(dev: &std::ffi::OsString) -> Result<(), Box<Error>> {
	let port = serial::open(dev)?;
	let mut port = match configure_port(port) {
		Ok(port) => port,
		Err(e) => return Err(e),
	};
	
	let mut packet = VNPacket::new();
	packet.fill_header();
	packet.fill_data();
	packet.fill_crc();
	
	loop {
		port.write(&packet.buf[..])?;
		thread::sleep(time::Duration::from_millis(500));
	}
}



// Receiver thread
fn thread2(dev: &std::ffi::OsString) -> Result<(), Box<Error>> {
let port = serial::open(dev)?;
	let mut port = match configure_port(port) {
		Ok(port) => port,
		Err(e) => return Err(e),
	};
	
	// arbitrary timeout
	port.set_timeout(time::Duration::from_millis(1000))?;
	
	let mut packet = VNPacket::new();
	packet.vn_data = VectornavData::clean(); // clean data
	
	loop {
		// initialize an emty buffer
		let mut buf = [0;255]; // still have to manually allocate an array
		
		// read data
		let len = match port.read(&mut buf[..]) {
			Ok(len) =>len,
			Err(e) => {
				println!("Skipping because {}",e);
				continue
			}, 
		};
		
		// parse data
		packet.parse_data(&buf[0..len]); // use a slice!
		
		if packet.msg_available {
			// print some stats
			println!("Received {} packets", packet.counter);
			println!("Checksum errors: {}", packet.chksm_err);
			println!("Hdr errors: {}", packet.hdr_err);
			//println!("Data {:?}", packet.vn_data);
			
			packet.msg_available = false;	
		}
		// sleep not needed
	}
}


fn main() {
	let port1 = match env::args_os().nth(1) {
		Some(port) => port,
		None => {
			println!("Port1 name not provided, exiting.");
			process::exit(1);
		},
	};
	
	let port2 = match env::args_os().nth(2) {
		Some(port) => port,
		None => {
			println!("Port2 name not provided, exiting.");
			process::exit(1);
		},
	};
	
	
	
	
	let t1 = thread::spawn(move || {
		if let Err(e) =  thread1(&port1) {
			println!("Error using {:?}: {}", port1, e);
		} else {
			println!("Thread 1 finished");	
		}
	});
	
	
	let t2 = thread::spawn(move || {
		if let Err(e) =  thread2(&port2) {
			println!("Error using {:?}: {}", port2, e);
		}
		else {
			println!("Thread 2 finished");	
		}
	});
	
	
	t1.join().expect("Error waiting for t1 to finish");
	t2.join().expect("Error waiting for t2 to finish");	
}