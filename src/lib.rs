extern crate serial;

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

#[allow(dead_code)]
const VN_GROUP_LENGTH: [[u8;16];6] = [
		[8, 8, 8, 12, 16, 12, 24, 12, 12, 24, 20, 28, 2, 4, 8, 0], // Group 1
		[8, 8, 8, 2, 8, 8, 8, 4, 0, 0, 0, 0, 0, 0, 0, 0], // Group 2
		[2, 12, 12, 12, 4, 4, 16, 12, 12, 12, 12, 2, 40, 0, 0, 0], // Group 3
		[8, 8, 2, 1, 1, 24, 24, 12, 12, 12, 4, 4, 32, 0, 0, 0], // Group 4
		[2, 12, 16, 36, 12, 12, 12, 12, 12, 12, 28, 24, 0, 0, 0, 0], // Group 5
		[2, 24, 24, 12, 12, 12, 12, 12, 12, 4, 4,68, 64, 0, 0, 0], // Group 6
		];

#[repr(C,packed)]
#[derive(Debug)]
pub struct VectornavData {
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
pub struct VNPacket {
	pub msg_available: bool,
	pub chksm_err: u32,
	pub hdr_err: u32,
	status: VNMsgStatus,
	calc_chk: u16,
	rec_chk: u16,
	pub counter: u16,
	pub vn_data: VectornavData,
	idx: u8,
	pub buf: Vec<u8>,
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
						// calculate payload size
						
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
