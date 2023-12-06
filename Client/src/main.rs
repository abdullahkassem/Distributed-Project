//use std::io::{self, Read, Write, BufRead};
// use std::net::UdpSocket;
#![allow(warnings)]
use std::env;
use std::str;
use local_ip_address::local_ip;
use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use serde::{Serialize, Deserialize};
//use csv::Reader;
use std::error::Error;
use image::io::Reader as ImageReader;
use std::process::Command;
use std::fs;
use inquire::list_option::ListOption;
use inquire::Select;
use inquire::InquireError;
use viuer::{print_from_file, Config};

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Message {
    pub id: usize,
    pub reciver_id: usize,
    pub sender_ip: String,
    pub reciver_ip: String,
    pub request: bool,  
    pub text: String,
    pub election: bool, //to start an election
    pub cpu_load: f32, //contains the cpu load
    pub cpu_message: bool, //this means that this message is for sharing the cpu load with other servers
    pub image_buffer: Vec<u8>,
    pub num_image_bytes: usize,
    pub fail_msg: bool,    // if recived from server that means that it has failed.
    pub recoverey: bool,   // Means that server has recovered.
    pub online: bool,  // if client online -- gets added/removed from directory of service 
    pub dor_request:bool, 
    pub dor: bool,  // want to see directory of service 
    pub directory: Vec<String>,
    pub viewimg_request: bool,
    // pub sending_allimg: bool,  // new 
    // pub all_img_buffer: Vec<u8>, //idk
    // pub no_of_img: usize //help
}



fn main() -> std::io::Result<()> {

    
// working code that prints an image in terminal  
// let conf = Config {
//     // set offset
//     x: 20,
//     y: 4,
//     // set dimensions
//     width: Some(80),
//     height: Some(25),
//     ..Default::default()
// };

 
// print_from_file("image.jpg", &conf).expect("Image printing failed.");
/// rest of code 3ady 
    //let server_addresses: [&str; 3] = ["10.0.2.15:2000","10.0.2.15:2001","10.0.2.15:2002"];
    let server_addresses: [&str; 3] = ["10.7.57.107:2000", "10.7.57.107:2005", "10.7.57.107:2006"];
    let args: Vec<String> = env::args().collect();
    let portNum = &args[1];
    let my_local_ip = local_ip().unwrap().to_string();

    println!("This is my local IP address: {:?}", my_local_ip);

    let options: Vec<&str> = vec!["online", "offline",  "Directory of Service", "send img"];
    let ans: Result<&str, InquireError> = Select::new("What would you like to do?", options).prompt();
    let mut err: &str = "error";
    let mut choice = match ans {
       Ok(choice) => choice,
       Err(_) => err,
   };
    //for multicasting:
    // let multicast_group_address = Ipv4Addr::new(224, 0, 0, 1);
    // socket.join_multicast_v4(&multicast_group_address, &Ipv4Addr::UNSPECIFIED).unwrap();
   
    let buffer_size = 2 * 1024 * 1024; // 2 MB buffer


    let socket = UdpSocket::bind(my_local_ip.clone()+ &":" +portNum)?;  // for UDP4/6
    //let mut buf = [0; 2097152];
    let mut buf = vec![0u8; buffer_size];
    let mut dirofser:Vec<String> =  Vec::new(); 


    // Load the image from a file
    let image_data = fs::read("image.jpg")?; // Replace with the path 

    //if image_data.len() > buffer_size {
    //    return Err("Image size exceeds the buffer size.".into());
    //}
    if (choice == "offline")
    {  println!("offline !!");
    
        for j in 0..3{

            let parts: Vec<&str> = server_addresses[j].split(':').collect();
            if parts.len() != 2 {
                eprintln!("Invalid server address format: {}", server_addresses[j]);
                continue;
            }
            let hostname = parts[0];
            let port = parts[1];


            let msg = Message {  // offline message 
                id: 1,
                reciver_id: 2,
                request: false,
                text: "hello".to_string(),
                election: false,
                cpu_load: 0.0,
                cpu_message: false,
                sender_ip: format!("{}:{}",my_local_ip.clone(),portNum.clone()),
                reciver_ip: format!("{}:{}",hostname.clone(),port.clone()),
                image_buffer: image_data.clone(),
                num_image_bytes: 0,
                fail_msg: false,
                recoverey: false,
                online: false,  
                dor_request: false,
                dor:false ,
                directory: dirofser.clone(),
                viewimg_request:false
            };
       
            let serialized_object = serde_json::to_string(&msg).unwrap();
            socket.send_to(&serialized_object.as_bytes(), server_addresses[j])
                    .expect("Error on send");
            //println!("Image sent to server at {}", server_addresses[j]);
        }
    }
    if (choice == "online")
    {  println!("online !!");

        for j in 0..3{

            let parts: Vec<&str> = server_addresses[j].split(':').collect();
            if parts.len() != 2 {
                eprintln!("Invalid server address format: {}", server_addresses[j]);
                continue;
            }
            let hostname = parts[0];
            let port = parts[1];


            let msg = Message {  // online message 
                id: 1,
                reciver_id: 2,
                request: false,
                text: "hello".to_string(),
                election: false,
                cpu_load: 0.0,
                cpu_message: false,
                sender_ip: format!("{}:{}",my_local_ip.clone(),portNum.clone()),
                reciver_ip: format!("{}:{}",hostname.clone(),port.clone()),
                image_buffer: image_data.clone(),
                num_image_bytes: 0,
                fail_msg: false,
                recoverey: false,
                online: true,  
                dor_request: false,
                dor:false ,
                directory: dirofser.clone(),
                viewimg_request:false
            };
       
            let serialized_object = serde_json::to_string(&msg).unwrap();
            socket.send_to(&serialized_object.as_bytes(), server_addresses[j])
                    .expect("Error on send");
            //println!("Image sent to server at {}", server_addresses[j]);
            
             let Ok((amt, src)) = socket.recv_from(&mut buf) else {
                    todo!()
                };
                // println!("packet recived from {:?}", src.to_string());
        
                let msg: Message = serde_json::from_slice(&buf[..amt]).unwrap();
                
                if msg.viewimg_request == true{
                println!("Recieved image request! i will now send all my images ");
                
                }

        }
    }

    if (choice == "Directory of Service")
    {
        for j in 0..3{

                let parts: Vec<&str> = server_addresses[j].split(':').collect();
                if parts.len() != 2 {
                    eprintln!("Invalid server address format: {}", server_addresses[j]);
                    continue;
                }
                let hostname = parts[0];
                let port = parts[1];


                let msg = Message {  // request dor message 
                    id: 1,
                    reciver_id: 2,
                    request: false,
                    text: "hello".to_string(),
                    election: false,
                    cpu_load: 0.0,
                    cpu_message: false,
                    sender_ip: format!("{}:{}",my_local_ip.clone(),portNum.clone()),
                    reciver_ip: format!("{}:{}",hostname.clone(),port.clone()),
                    image_buffer: image_data.clone(),
                    num_image_bytes: 0,
                    fail_msg: false,
                    recoverey: false,
                    online: true,  
                    dor_request: true,
                    dor:false ,
                    directory: dirofser.clone(), 
                    viewimg_request:false
                };
        
                let serialized_object = serde_json::to_string(&msg).unwrap();
                socket.send_to(&serialized_object.as_bytes(), server_addresses[j])
                        .expect("Error on send");
                //println!("Image sent to server at {}", server_addresses[j]);

                let Ok((amt, src)) = socket.recv_from(&mut buf) else {
                    todo!()
                };
                // println!("packet recived from {:?}", src.to_string());
        
                let msg: Message = serde_json::from_slice(&buf[..amt]).unwrap();
                dirofser = msg.directory.clone();
                println!("Directory of service {:?}", dirofser);
//let options2: Vec<&str> = vec!["online", "offline",  "Directory of Service", "send img"];
		let v2: Vec<&str> = dirofser.iter().map(|s| s as &str).collect();
 		let ans2: Result<&str, InquireError> = Select::new("Whos pics do u want to see", v2).prompt();
		    let mut err: &str = "error";
		    let mut choice2 = match ans2 {
		       Ok(choice2) => choice2,
		       Err(_) => err,
		   };
                
                // choice 2 -- string that is the ip address of client picked to veiw their pics
                // send a request message to view images from other person 
                let msg = Message {  //  request to view message  
                    id: 1,
                    reciver_id: 2,
                    request: false,
                    text: "hello".to_string(),
                    election: false,
                    cpu_load: 0.0,
                    cpu_message: false,
                    sender_ip: format!("{}:{}",my_local_ip.clone(),portNum.clone()),
                    reciver_ip: format!("{}:{}",hostname.clone(),port.clone()),
                    image_buffer: image_data.clone(),
                    num_image_bytes: 0,
                    fail_msg: false,
                    recoverey: false,
                    online: true,  
                    dor_request: true,
                    dor:false,
                    directory: dirofser.clone(),
                    viewimg_request:true
                };
                
                let serialized_object = serde_json::to_string(&msg).unwrap();
                socket.send_to(&serialized_object.as_bytes(), choice2)
                        .expect("Error on send");
                
                
                
                
                


            }
        }
    if (choice == "send img") 
    {
        for i in 0..2 {

            for j in 0..3{

                let parts: Vec<&str> = server_addresses[j].split(':').collect();
                if parts.len() != 2 {
                    eprintln!("Invalid server address format: {}", server_addresses[j]);
                    continue;
                }
                let hostname = parts[0];
                let port = parts[1];


                let msg = Message {  // this needs to be edited  
                    id: 1,
                    reciver_id: 2,
                    request: false,
                    text: "hello".to_string(),
                    election: true,
                    cpu_load: 0.0,
                    cpu_message: false,
                    sender_ip: format!("{}:{}",my_local_ip.clone(),portNum.clone()),
                    reciver_ip: format!("{}:{}",hostname.clone(),port.clone()),
                    image_buffer: image_data.clone(),
                    num_image_bytes: image_data.len(),
                    fail_msg: false,
                    recoverey: false,
                    online: true,  
                    dor_request:false, 
                    dor:false,
                    directory: dirofser.clone(),
                    viewimg_request:true
                };
        
                let serialized_object = serde_json::to_string(&msg).unwrap();
                socket.send_to(&serialized_object.as_bytes(), server_addresses[j])
                        .expect("Error on send");
                println!("Image sent to server at {}", server_addresses[j]);
            }
        }
       

    }

    Ok(())
}
