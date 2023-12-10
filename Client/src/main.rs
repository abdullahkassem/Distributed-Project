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
    pub election: bool,    //to start an election
    pub cpu_load: f32,     //contains the cpu load
    pub msgID: String,
    pub image_buffer: Vec<u8>,
    pub num_image_bytes: usize,
    pub fail_msg: bool,    // if recived from server that means that it has failed.
    pub recoverey: bool,   // Means that server has recovered.
    pub online: bool,  // if client online -- gets added/removed from directory of service 
    pub dor_request:bool, 
    pub dor: bool,  // want to see directory of service 
    pub directory: Vec<String>,
    pub viewimg_request: bool,
    pub ClientToBeSentTo: String,
    // pub sending_allimg: bool,  // new 
    // pub all_img_buffer: Vec<u8>, //idk
    // pub no_of_img: usize //help
}

// usage -> displayImage(&["img1.jpeg","img2.png"]);
fn displayImage (strings: &[&str]) {
    let command = "feh";
    let args = strings;
    let output = Command::new(command)
        .args(args)
        .output()
        .expect("Failed to display image");

    // if output.status.success() {
    //     ;//println!();
    // } else {
    //     ;
    // }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {

    
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
    let server_addresses: [&str; 3] = ["10.0.2.7:2000", "10.0.2.7:2001", "10.0.2.7:2002"];
    let args: Vec<String> = env::args().collect();
    let portNum = &args[1];
    let my_local_ip = local_ip().unwrap().to_string();
    let mut msgCount = 0;
    let imageNames = ["image1.jpg","image2.jpg"];
    let imageCount = imageNames.len();

    println!("This is my local IP address: {:?}", my_local_ip);

    let mut offline_chosen = false;
    loop{

    let options: Vec<&str> = if offline_chosen{vec!("online")}
    else{
        vec!["online", "offline",  "Directory of Service", "send img"]
    };

    
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
    //let image_data = fs::read("image.jpg")?; // Replace with the path 

    //if image_data.len() > buffer_size {
    //    return Err("Image size exceeds the buffer size.".into());
    //}
    if (choice == "offline")
    {  println!("offline !!");
        offline_chosen = true;
    
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
                msgID: format!("{}:{}-{}", my_local_ip.clone(), portNum.clone(),msgCount),
                sender_ip: format!("{}:{}", my_local_ip.clone(), portNum.clone()),
                reciver_ip: format!("{}:{}", hostname.clone(), port.clone()),
                image_buffer: vec![0u8; 2],
                num_image_bytes: 0,
                fail_msg: false,
                recoverey: false,
                online: false,  
                dor_request: false,
                dor:false ,
                directory: dirofser.clone(),
                viewimg_request:false,
                ClientToBeSentTo: "".to_string(),
            };
       
            let serialized_object = serde_json::to_string(&msg).unwrap();
            socket.send_to(&serialized_object.as_bytes(), server_addresses[j])
                    .expect("Error on send");
            //println!("Image sent to server at {}", server_addresses[j]);
        }
    }
    if (choice == "online")
    {  println!("online !!");
        offline_chosen = false;
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
                msgID: format!("{}:{}-{}", my_local_ip.clone(), portNum.clone(),msgCount),
                sender_ip: format!("{}:{}", my_local_ip.clone(), portNum.clone()),
                reciver_ip: format!("{}:{}", hostname.clone(), port.clone()),
                image_buffer: vec![0u8; 2],
                num_image_bytes: 0,
                fail_msg: false,
                recoverey: false,
                online: true,  
                dor_request: false,
                dor:false ,
                directory: dirofser.clone(),
                viewimg_request:false,
                ClientToBeSentTo: "".to_string(),
            };
       
            let serialized_object = serde_json::to_string(&msg).unwrap();
            socket.send_to(&serialized_object.as_bytes(), server_addresses[j])
                    .expect("Error on send");
            
            //println!("Image sent to server at {}", server_addresses[j]);            

        }
        let Ok((amt, src)) = socket.recv_from(&mut buf) else {
            todo!()
        };
        // println!("packet recived from {:?}", src.to_string());

        let msg: Message = serde_json::from_slice(&buf[..amt]).unwrap();
        
        if msg.viewimg_request == true{
            println!("Recieved image request! i will now send all my images, count = {}",imageCount.clone());
            
            for i in 0..imageCount{
                let image_path = format!("./LowRes/{}",imageNames[i].clone());
                println!("reading image {}",image_path.clone());
                
                let image_data = fs::read(image_path)?;
                let imgMessage = Message {  // request dor message 
                    id: 1,
                    reciver_id: 2,
                    request: false,
                    text: imageNames[i].to_string(),
                    election: false,
                    cpu_load: 0.0,
                    msgID: "empty".to_string(),
                    sender_ip: "empty".to_string(),
                    reciver_ip: "empty".to_string(),
                    image_buffer: image_data.clone(),
                    num_image_bytes: image_data.len(),
                    fail_msg: false,
                    recoverey: false,
                    online: true,  
                    dor_request: false,
                    dor:false ,
                    directory: vec![], 
                    viewimg_request:false,
                    ClientToBeSentTo: "empty".to_string(),
                };
                let serialized_object = serde_json::to_string(&imgMessage).unwrap();
                 socket.send_to(&serialized_object.as_bytes(), src)
                .expect("Error on send");
              }
        
        }else{
            println!("I was waiting for all img request, got smth else");
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
                    msgID: format!("{}:{}-{}", my_local_ip.clone(), portNum.clone(),msgCount),
                    sender_ip: format!("{}:{}", my_local_ip.clone(), portNum.clone()),
                    reciver_ip: format!("{}:{}", hostname.clone(), port.clone()),
                    image_buffer: vec![0u8; 2],
                    num_image_bytes: 0,
                    fail_msg: false,
                    recoverey: false,
                    online: true,  
                    dor_request: true,
                    dor:false ,
                    directory: dirofser.clone(), 
                    viewimg_request:false,
                    ClientToBeSentTo: "".to_string(),
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
                    msgID: format!("{}:{}-{}", my_local_ip.clone(), portNum.clone(),msgCount),
                    sender_ip: format!("{}:{}", my_local_ip.clone(), portNum.clone()),
                    reciver_ip: format!("{}:{}", hostname.clone(), port.clone()),
                    image_buffer: vec![0u8; 2],
                    num_image_bytes: 0,
                    fail_msg: false,
                    recoverey: false,
                    online: true,  
                    dor_request: true,
                    dor:false,
                    directory: dirofser.clone(),
                    viewimg_request:true,
                    ClientToBeSentTo: "".to_string(),
                };
                
                let serialized_object = serde_json::to_string(&msg).unwrap();
                socket.send_to(&serialized_object.as_bytes(), choice2)
                        .expect("Error on send");

                // recieving low res images.
                for i in 0..2{

                    let Ok((amt, src)) = socket.recv_from(&mut buf) else {
                        todo!()
                    };

                    let msg: Message = serde_json::from_slice(&buf[..amt]).unwrap();

                    let num_bytes = msg.num_image_bytes;
                    let image_data = &msg.image_buffer[..num_bytes];
                    let image = ImageReader::new(std::io::Cursor::new(image_data))
                        .with_guessed_format()?
                        .decode();
                    

                    // Save the image as a .jpg file

                    println!("Recieving low res image and saving it as {}",format!("./Recieved/{}",msg.text));
                    
                    image?
                        .save(format!("./Recieved/{}",msg.text))
                        .map_err(|e| format!("Error saving image: {}", e))?;
                    println!("Recieving low res image and saved it");

                }

                displayImage(&["./LowRes/image1.jpg","./LowRes/image2.jpg"]);
                let options3: Vec<&str> = vec!["image1.jpg","image2.jpg"];
                let ans: Result<&str, InquireError> = Select::new("Choose 1 image to request", options3).prompt();
                let mut err: &str = "error";
                let mut choice3 = match ans {
                Ok(choice3) => choice3,
                Err(_) => err,};

                msgCount+=1;
                let mut image_data = fs::read("./LowRes/image1.jpg")?; 
                if choice3 == "image1.jpg"
                {
                    image_data = fs::read("./LowRes/image1.jpg")?; 
                }
                else if choice3 == "image2.jpg"
                {
                    image_data = fs::read("./LowRes/image2.jpg")?; 
                }

                
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
                        msgID: format!("{}:{}-{}", my_local_ip.clone(), portNum.clone(),msgCount),
                        sender_ip: format!("{}:{}", my_local_ip.clone(), portNum.clone()),
                        reciver_ip: format!("{}:{}", hostname.clone(), port.clone()),
                        image_buffer: image_data.clone(),
                        num_image_bytes: image_data.len(),
                        fail_msg: false,
                        recoverey: false,
                        online: true,  
                        dor_request:false, 
                        dor:false,
                        directory: dirofser.clone(),
                        viewimg_request:true,
                        ClientToBeSentTo: "idk".to_string(),
                    };
        
                    let serialized_object = serde_json::to_string(&msg).unwrap();
                    socket
                        .send_to(&serialized_object.as_bytes(), server_addresses[j])
                        .expect("Error on send");
                    println!("Image sent to server at {}", server_addresses[j]);
                }

                    let Ok((amt, src)) = socket.recv_from(&mut buf) else {
                        todo!()
                    };

                    let msg: Message = serde_json::from_slice(&buf[..amt]).unwrap();

                    let num_bytes = msg.num_image_bytes;
                    let image_data = &msg.image_buffer[..num_bytes];
                    let image = ImageReader::new(std::io::Cursor::new(image_data))
                        .with_guessed_format()?
                        .decode();
                    

                    // Save the image as a .jpg file
                    image?
                        .save(format!("./Enctypted/{}",msg.text))
                        .map_err(|e| format!("Error saving image: {}", e))?;



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
                    msgID: format!("{}:{}-{}", my_local_ip.clone(), portNum.clone(),msgCount),
                    sender_ip: format!("{}:{}", my_local_ip.clone(), portNum.clone()),
                    reciver_ip: format!("{}:{}", hostname.clone(), port.clone()),
                    image_buffer: vec![0u8; 2],
                    num_image_bytes: 0,
                    fail_msg: false,
                    recoverey: false,
                    online: true,  
                    dor_request:false, 
                    dor:false,
                    directory: dirofser.clone(),
                    viewimg_request:true,
                    ClientToBeSentTo: "".to_string(),
                };
        
                let serialized_object = serde_json::to_string(&msg).unwrap();
                socket.send_to(&serialized_object.as_bytes(), server_addresses[j])
                        .expect("Error on send");
                println!("Image sent to server at {}", server_addresses[j]);
            }
        }
    }

    }

    Ok(())
}
