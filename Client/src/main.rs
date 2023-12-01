//use std::io::{self, Read, Write, BufRead};
// use std::net::UdpSocket;
#![allow(warnings)]
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::str;
//use csv::Reader;
use image::io::Reader as ImageReader;
use std::error::Error;
use std::fs;
use std::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: usize,
    pub reciver_id: usize,
    pub sender_ip: String,
    pub reciver_ip: String,
    pub request: bool,
    pub text: String,
    pub election: bool,    //to start an election
    pub cpu_load: f32,     //contains the cpu load
    pub cpu_message: bool, //this means that this message is for sharing the cpu load with other servers
    pub image_buffer: Vec<u8>,
    pub num_image_bytes: usize,
    pub fail_msg: bool,  // if recived from server that means that it has failed.
    pub recoverey: bool, // Means that server has recovered.
}

fn main() -> std::io::Result<()> {
    //let server_addresses: [&str; 3] = ["10.0.2.15:2000","10.0.2.15:2001","10.0.2.15:2002"];
    let server_addresses: [&str; 3] = ["192.168.1.3:2000", "192.168.1.3:2001", "192.168.1.3:2002"];
    let args: Vec<String> = env::args().collect();
    let portNum = &args[1];
    let my_local_ip = local_ip().unwrap().to_string();

    println!("This is my local IP address: {:?}", my_local_ip);

    //for multicasting:
    // let multicast_group_address = Ipv4Addr::new(224, 0, 0, 1);
    // socket.join_multicast_v4(&multicast_group_address, &Ipv4Addr::UNSPECIFIED).unwrap();

    let buffer_size = 2 * 1024 * 1024; // 2 MB buffer

    let socket = UdpSocket::bind(my_local_ip.clone() + &":" + portNum)?; // for UDP4/6
                                                                         //let mut buf = [0; 2097152];
    let mut buf = vec![0u8; buffer_size];

    // Load the image from a file
    let image_data = fs::read("image.jpg")?; // Replace with the path

    //if image_data.len() > buffer_size {
    //    return Err("Image size exceeds the buffer size.".into());
    //}
    for i in 0..2 {
        for j in 0..3 {
            let parts: Vec<&str> = server_addresses[j].split(':').collect();
            if parts.len() != 2 {
                eprintln!("Invalid server address format: {}", server_addresses[j]);
                continue;
            }
            let hostname = parts[0];
            let port = parts[1];

            let msg = Message {
                id: 1,
                reciver_id: 2,
                request: false,
                text: "hello".to_string(),
                election: true,
                cpu_load: 0.0,
                cpu_message: false,
                sender_ip: format!("{}:{}", my_local_ip.clone(), portNum.clone()),
                reciver_ip: format!("{}:{}", hostname.clone(), port.clone()),
                image_buffer: image_data.clone(),
                num_image_bytes: image_data.len(),
                fail_msg: false,
                recoverey: false,
            };

            let serialized_object = serde_json::to_string(&msg).unwrap();
            socket
                .send_to(&serialized_object.as_bytes(), server_addresses[j])
                .expect("Error on send");
            println!("Image sent to server at {}", server_addresses[j]);
        }
    }

    Ok(())
}
