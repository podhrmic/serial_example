extern crate serial;
extern crate vectornav;

use std::env;
use std::error::Error;
use std::{thread, time};

use std::io::prelude::*;
use serial::prelude::*;
use std::process;
use vectornav::VNPacket;
use vectornav::VectornavData;




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