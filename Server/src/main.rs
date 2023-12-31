#![allow(warnings)]

use image::io::Reader as ImageReader;
use local_ip_address::local_ip;
use queues::*;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::process::Command;
use std::str;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};
use systemstat::{saturating_sub_bytes, Platform, System};
// use polling::{Event, Poller};
use std::process::exit;
use regex::Regex;
use std::task::Poll;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: usize,
    pub reciver_id: usize,
    pub sender_ip: String,
    pub reciver_ip: String,
    pub request: bool,
    pub text: String,
    pub election: bool, //to start an election
    pub cpu_load: f32,  //contains the cpu load
    pub msgID: String,
    pub image_buffer: Vec<u8>,
    pub num_image_bytes: usize,
    pub fail_msg: bool,  // if recived from server that means that it has failed.
    pub recoverey: bool, // Means that server has recovered.
    pub online: bool,  // if client online -- gets added/removed from directory of service
    pub dor_request: bool,  // want to see directory of service
    pub dor:bool, 
    pub directory: Vec<String>,
    pub viewimg_request: bool,
    pub ClientToBeSentTo: String,

}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpuLoadMsg {
    pub value: f32,
    pub ownerIp: String,
    pub ElectionNum: String,
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

//thread to execute the workload
fn execute_load(
    socket: UdpSocket,
    workQ: &mut Arc<Mutex<queues::Queue<Message>>>,
    prtNum: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut EncryptionSuccessCnt = 0;
    let mut EncryptionFailCnt = 0;
    loop {
        // println!("Queue size {}",workQ.lock().unwrap().size());
        if workQ.lock().unwrap().size() > 0 {
            let curMsg = workQ.lock().unwrap().peek().unwrap();
            workQ.lock().unwrap().remove();
            let num_bytes = curMsg.num_image_bytes;
            let image_data = &curMsg.image_buffer[..num_bytes];
            let image = ImageReader::new(std::io::Cursor::new(image_data))
                .with_guessed_format()?
                .decode();

            
            
            let mut imgName = format!("./{}.jpg",curMsg.msgID.clone());

            let address = &imgName;
            let re = Regex::new(r"(\d+\.\d+\.\d+\.\d+):(\d+)-(\d+)").unwrap();
            let captures = re.captures(address).unwrap();
            let ip = captures.get(1).unwrap().as_str();
            let port = captures.get(2).unwrap().as_str();
            let instance = captures.get(3).unwrap().as_str();
            let converted_address = format!(r"{}_{}-{}.jpg", ip, port, instance);

            imgName = converted_address.clone();
            

            println!("Will start executing an image called {}",imgName.clone());

            // Save the image as a .jpg file
            image?
                .save(imgName.clone())
                .map_err(|e| format!("Error saving image: {}", e))?;
            
            println!("Received image saved as {}",imgName.clone());

            let prtNum_cloned = prtNum.clone();

            let encryptedImgName = format!("Enctypted_{}",imgName.clone());
            println!("encryptedImgName is {}",encryptedImgName);

            let command = "steghide";
            let args = [
                "embed",
                "-cf",
                "super.jpg",
                "-ef",
                &imgName,
                "-sf",
                &encryptedImgName,
                "-p",
                "123",
                "-f",
            ];
            let output = Command::new(command)
                .args(&args)
                .output()
                .expect("Failed to steghide");

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                EncryptionSuccessCnt += 1;
                println!("encrypted image successfully");
                let QueueLen = workQ.lock().unwrap().size();
                println!("Num of successful Encryption: {}, Failures: {} images remaining in queue is {}",EncryptionSuccessCnt,EncryptionFailCnt,QueueLen);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                EncryptionFailCnt += 1;
                println!("failed to encrypt image.");
            }

            let image_data = fs::read(format!("./{}",encryptedImgName))?;

            let returnImg = Message {  // request dor message 
                id: 1,
                reciver_id: 2,
                request: false,
                text: "".to_string(),
                election: false,
                cpu_load: 0.0,
                msgID: curMsg.msgID,
                sender_ip: curMsg.reciver_ip,
                reciver_ip: curMsg.sender_ip.clone(),
                image_buffer: image_data.clone(),
                num_image_bytes: image_data.len(),
                fail_msg: false,
                recoverey: false,
                online: false,  
                dor_request: false,
                dor:false ,
                directory: vec![], 
                viewimg_request:false,
                ClientToBeSentTo: curMsg.ClientToBeSentTo,
            };

            let serialized_object = serde_json::to_string(&returnImg).unwrap();

            socket.send_to(&serialized_object.as_bytes(),curMsg.sender_ip).expect("Error on send");



            //println!("finished handling request.");
        }
    }

    Ok(())
}

// thread that will always listen to requests and put them into queue:
fn handle_requests(
    socket: UdpSocket,
    workQ: &mut Arc<Mutex<queues::Queue<Message>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let buffer_size = 2 * 1024 * 1024; // 2 MB buffer
    let mut buffer = vec![0u8; buffer_size];

    loop {
        let Ok((amt, src)) = socket.recv_from(&mut buffer) else {
            todo!()
        };
        //println!("packet recived from {:?}", src.to_string());

        let msg: Message = serde_json::from_slice(&buffer[..amt]).unwrap();

        workQ.lock().unwrap().add(msg.clone());
    }

    Ok(())
}

//Worker thread that will handle messages in the workQ
//Currently handles messages of type election and cpuload
fn workerThread(
    socket: UdpSocket,
    finalIP: &String,
    workQ: &mut Arc<Mutex<queues::Queue<Message>>>,
    prtNum: &String,
    execQ: &mut Arc<Mutex<queues::Queue<Message>>>,
) {

    //Configurable Variables:
    let server_addresses: [&str; 3] = ["10.7.57.73:2000", "10.7.57.74:2000", "10.7.57.94:2000"];
    let allowFailures = true;
    let CpuWaitTimeoutPeriod = 5; //Timeout period in seconds
    




    //let client_addresses: [&str; 3] = ["10.0.2.7:3000", "192.168.1.3:3001", "192.168.91.128:3002"];
    let mut client_addresses:Vec<String> = Vec::new(); // change1
    let mut ServServ_addresses: Vec<String> = Vec::new();

    let mut otherServ1 = "empty";
    let mut otherServ2 = "empty";

    // Finidng the address of the other servers.
    for addr in server_addresses.iter() {
        //let addr_String = addr.to_string();
        if addr != finalIP && otherServ1 == "empty" {
            otherServ1 = addr;
        } else if addr != finalIP && otherServ2 == "empty" {
            otherServ2 = addr;
        }
    }

    //println!("srv1= {} srv2= {}", otherServ1, otherServ2);

    //Calculate the Server to server addresses of other servers.
    for i in 0..3 {
        let mut sIP = server_addresses[i].to_string();
        let sP = sIP.split(":").last().unwrap();
        let new_sP = sP.parse::<u32>().unwrap() + 100;
        let SSAdd = format!("{}:{}", sIP.split(":").next().unwrap(), new_sP);
        ServServ_addresses.push(SSAdd.clone());
    }

    

    // Get this server's servToServ IP
    let mut MyPort = finalIP.split(":").last().unwrap();
    let MyNewPort = MyPort.parse::<u32>().unwrap() + 100;
    let ServerCommunicationIP = format!("{}:{}", finalIP.split(":").next().unwrap(), MyNewPort);
    println!("ServerCommIP: {}.", ServerCommunicationIP);

    let S_to_S_Socket =
        UdpSocket::bind(ServerCommunicationIP.clone()).expect("Failed to bind socket");

    S_to_S_Socket.set_nonblocking(true).unwrap();


    let my_local_ip_wt = local_ip().unwrap().to_string();
    let mut sys = System::new();

    let mut cpuMsgCnt = 0;
    let mut cpuServ1: f32 = -1.0;
    let mut cpuServ2: f32 = -1.0;
    let mut idServ1 = 0;
    let mut idServ2 = 0;
    let mut ipServer1 = "".to_string();
    let mut ipServer2 = "".to_string();

    let mut minCpuIP = "".to_string(); // Will contain the ip of the min cpuload

    let mut start = Instant::now();

    let mut CurrentElection = -1; // if -1 could start a new election if not dont start a new election

    let mut someoneFailed = false;
    let mut IamDown = false;
    let mut failedIp = "".to_string();
    let mut onlineIP = "".to_string(); 
    let mut offlineIP = "".to_string();

    let mut end = Instant::now();
    let mut secPassed = 0;

    let mut CpuWaitTimeout = false;

    let mut imgMsg: Message = Message {
        id: 1,
        reciver_id: 2,
        request: false,
        text: "hello".to_string(),
        election: false,
        cpu_load: 0.0,
        msgID: "empty".to_string(),
        sender_ip: "0000".to_string(),
        reciver_ip: "0000".to_string(),
        image_buffer: vec![0u8; 2],
        num_image_bytes: 0,
        fail_msg: false,
        recoverey: false,
        online: true,
        dor_request: false, 
        dor:false,
        directory: vec![],
        viewimg_request:false,
        ClientToBeSentTo: "".to_string(),
    };

    loop {
        end = Instant::now();

        // if secPassed < end.duration_since(start).as_secs(){
        //     println!("current time since start {:?}",end.duration_since(start).as_secs());
        //     secPassed = end.duration_since(start).as_secs();
        //     println!("iamdown variable = {}",IamDown);
        //     println!("allowFailures {}",allowFailures);
        //     println!("someoneFailed {}",someoneFailed);
        // }

        // If server is down it will be up after duration

        if IamDown && end.duration_since(start) > Duration::from_secs(20) {
            println!("I am no longer Down!!");
            IamDown = false;
            failedIp = "".to_string();

            for addr in server_addresses.iter() {
                let addr_String = addr.to_string();

                if addr_String == *finalIP {
                    continue;
                }

                if addr_String != format!("{}:{}", my_local_ip_wt, prtNum) {
                    let recoverMsg = Message {
                        id: 1,
                        request: false,
                        reciver_id: 1, // To be changed
                        text: "RecoverMsg".to_string(),
                        election: false,
                        cpu_load: 0.0,
                        msgID: "empty".to_string(),
                        sender_ip: format!("{}:{}", my_local_ip_wt, prtNum),
                        reciver_ip: addr_String.clone(),
                        fail_msg: false,
                        recoverey: true,
                        image_buffer: vec![0u8; 2],
                        num_image_bytes: 0,
                        online: true,  
                        dor_request: false,
                        dor:false,
                        directory: vec![],
                        viewimg_request:false,
                        ClientToBeSentTo: "".to_string(),
                    };

                    let serialized_object = serde_json::to_string(&recoverMsg).unwrap();

                    socket
                        .send_to(&serialized_object.as_bytes(), addr_String)
                        .expect("Error on send");
                }
            }
            start = Instant::now();
            secPassed = 0;
        }
        //else if it is not down it will periodically check if it has the lowest CPU Load
        else if allowFailures
            && !someoneFailed
            && end.duration_since(start) > Duration::from_secs(10)
            && IamDown == false
        {
            //println!("my ip {}, mincpuIP {}",*finalIP,minCpuIP.clone());
            if *finalIP == minCpuIP && IamDown == false {
                println!("I will Fail!!!");
                IamDown = true;
                failedIp = (*finalIP).clone();

                for addr in server_addresses.iter() {
                    let addr_String = addr.to_string();

                    if addr_String == *finalIP {
                        continue;
                    }

                    if addr_String != format!("{}:{}", my_local_ip_wt, prtNum) {
                        let failMsg = Message {
                            id: 1,
                            request: false,
                            reciver_id: 1, // To be changed
                            text: "failMsg".to_string(),
                            election: false,
                            cpu_load: 0.0,
                            msgID: "empty".to_string(),
                            sender_ip: format!("{}:{}", my_local_ip_wt, prtNum),
                            reciver_ip: addr_String.clone(),
                            fail_msg: true,
                            recoverey: false,
                            image_buffer: vec![0u8; 2],
                            num_image_bytes: 0,
                            online: true,  
                            dor_request: false, 
                            dor:false,
                            directory: vec![],
                            viewimg_request:false,
                            ClientToBeSentTo: "".to_string(),
                        };

                        let serialized_object = serde_json::to_string(&failMsg).unwrap();

                        socket
                            .send_to(&serialized_object.as_bytes(), addr_String)
                            .expect("Error on send");
                    }
                }
            }
            start = Instant::now();
            secPassed = 0;
        }

        let mut cpuMsgsQ = queue![];

        // Handle The messages in the queue
        if workQ.lock().unwrap().size() > 0 {
            let curMsg = workQ.lock().unwrap().peek().unwrap();
            workQ.lock().unwrap().remove();

            // Check if the sender ip is a server
            if (server_addresses.contains(&curMsg.sender_ip.as_str())) {
                
                // This makes sure the server ignores and loses Any messages in queue after it fails
                // This applies to existing messages in queue when failing and messages recieved after failing.
                if failedIp == *finalIP {
                    //println!("I failed so I will ignore");
                    continue;
                }

                // If recieved a message notifing that a server has failed.
                if curMsg.fail_msg == true {
                    someoneFailed = true;
                    failedIp = curMsg.sender_ip.clone();
                }

                // If recieved a message notifing that the server that failed is up.
                if curMsg.recoverey == true {
                    someoneFailed = false;
                    failedIp = "".to_string();
                    start = Instant::now();
                    secPassed = 0;
                }
            } else   {
                // if I am down I will ignore client messages.
                if failedIp == *finalIP {
                    println!("I failed so I will ignore");
                    continue;
                }

                // This messages is so that the if there is an election happening we dont start another unitl the 1st finished.
                if curMsg.election == true && CurrentElection != -1 {
                    workQ.lock().unwrap().add(curMsg.clone());

                    continue;
                }

                if curMsg.election == true {

                    println!("Will start an election.");
                    
                    CurrentElection = 1;

                    // This will contain the message that will be passed to the other queue.
                    imgMsg = curMsg.clone();

                    let myCpuLoad = sys.load_average().unwrap().one;
                    let curMsgId = curMsg.msgID.clone();

                    let mut newFailedIP = ".".to_string();
                    if failedIp != ""
                    {
                        let failedPort = failedIp.split(":").last().unwrap();
                        let NewfailedPort = failedPort.parse::<u32>().unwrap() + 100;
                        newFailedIP = format!("{}:{}", failedIp.split(":").next().unwrap(), NewfailedPort);
                    }
                    
                    // Send My Cpu load to the other servers
                    for addr in ServServ_addresses.iter() {
                        let addr_String = addr.to_string();

                        if addr_String != ServerCommunicationIP && addr_String != newFailedIP {

                            let announcmentMsg = CpuLoadMsg {
                                value: myCpuLoad,
                                ownerIp: finalIP.to_string(),
                                ElectionNum: curMsg.msgID.clone(),
                            };

                            let serialized_object = serde_json::to_string(&announcmentMsg).unwrap();
                            let result =
                                S_to_S_Socket.send_to(&serialized_object.as_bytes(), addr_String);
                        }
                    }

                    //Wait for the Cpu Load of the other servers.
                    let mut waitFor = 2;
                    if someoneFailed == true {
                        waitFor -= 1;
                    }


                    let mut cpuElNum1 = "-1".to_string();
                    let mut cpuElNum2 = "-1".to_string();

                    let Cpu_buffer_size = 2 * 1024;
                    let mut Cpu_buffer = vec![0u8; Cpu_buffer_size];

                    
                    let mut finished_waiting = false;
                    
                    loop {

                        match S_to_S_Socket.recv_from(&mut Cpu_buffer)
                        {
                            Ok((number_of_bytes, src_addr)) => {
                                let cpuLoadRecieved: CpuLoadMsg =
                                serde_json::from_slice(&Cpu_buffer[..number_of_bytes]).unwrap();
                            
                                cpuMsgsQ.add(cpuLoadRecieved);
                            }

                            Err(err) => {
                                // Handle the error
                                ;
                            }
                        }

                        let cpuWaitStart = Instant::now();
                        CpuWaitTimeout = false;

                        while cpuMsgsQ.size() != 0 
                        {
                            let cpuWaitCurr = Instant::now();
                            if cpuWaitCurr.duration_since(cpuWaitStart)>Duration::from_secs(CpuWaitTimeoutPeriod)
                            {
                                
                                finished_waiting = true;
                                CpuWaitTimeout = true;
                            }

                            let currentCpuMsg = cpuMsgsQ.peek().unwrap();
                            cpuMsgsQ.remove();
                            if currentCpuMsg.ElectionNum.clone() == curMsgId.clone()
                                && ( currentCpuMsg.ownerIp.clone() == otherServ1
                                    || currentCpuMsg.ownerIp.clone() == otherServ2 )
                            {

                                if currentCpuMsg.ownerIp.clone() == otherServ1 {
                                    cpuServ1 = currentCpuMsg.value.clone();
                                    ipServer1 = currentCpuMsg.ownerIp.clone();
                                    cpuElNum1 = currentCpuMsg.ElectionNum.clone();
                                }
                                if currentCpuMsg.ownerIp.clone() == otherServ2 {
                                    cpuServ2 = currentCpuMsg.value.clone();
                                    ipServer2 = currentCpuMsg.ownerIp.clone();
                                    cpuElNum2 = currentCpuMsg.ElectionNum.clone();
                                }

                                if (cpuServ1 != -1.0 && cpuServ2 != -1.0) || (someoneFailed && (cpuServ1 != -1.0 || cpuServ2 != -1.0) ) {
                                    finished_waiting = true;
                                    break;
                                }
                            } else {
                                cpuMsgsQ.add(currentCpuMsg.clone());
                            }
                        }
                        

                        if finished_waiting {
                            finished_waiting = false;
                            break;
                        }
                    }

                    if CpuWaitTimeout 
                    {
                        println!("CPU wait timeout, will fail to handle message ✖✖✖✖✖✖✖✖✖✖✖✖✖✖✖✖✖✖");
                        ipServer1 = "empty".to_string();
                        ipServer2 = "empty".to_string();
                        cpuServ1 = -1.0;
                        cpuServ2 = -1.0;
                        CurrentElection = -1;
                        continue;
                    }


                    if waitFor == 1 {
                        if cpuServ1 == -1.0
                        {
                            cpuServ1 = 999.0;
                        }
                        else if cpuServ2 == -1.0
                        {
                            cpuServ2 = 999.0;
                        }
                        else
                        {panic!("This shouldnt happed")}
                    }


                    // println!(
                    //     "I have \n0. Ip: {} with load {} for no {}",
                    //     *finalIP,
                    //     myCpuLoad,
                    //     curMsgId.clone()
                    // );
                    // println!(
                    //     "1. Ip: {} with load {} for no {}",
                    //     ipServer1, cpuServ1, cpuElNum1
                    // );
                    // println!(
                    //     "2. Ip: {} with load {} for no {}",
                    //     ipServer2, cpuServ2, cpuElNum2
                    // );

                    

                    if cpuServ1 == cpuServ2 && cpuServ1 == myCpuLoad {
                        if *finalIP < ipServer1 && *finalIP < ipServer2 {
                            minCpuIP = (*finalIP).clone();
                        } else if ipServer1 < *finalIP && ipServer1 < ipServer2 {
                            minCpuIP = ipServer1.clone();
                        } else if ipServer2 < *finalIP && ipServer2 < ipServer1 {
                            minCpuIP = ipServer2.clone();
                        }
                    } else if myCpuLoad > cpuServ1 && cpuServ1 == cpuServ2 {
                        if ipServer1 < ipServer2 {
                            minCpuIP = ipServer1.clone();
                        } else {
                            minCpuIP = ipServer2.clone();
                        }
                    } else if cpuServ1 > myCpuLoad && myCpuLoad == cpuServ2 {
                        if *finalIP < ipServer2 {
                            minCpuIP = (*finalIP).clone();
                        } else {
                            minCpuIP = ipServer2.clone();
                        }
                    } else if cpuServ2 > myCpuLoad && myCpuLoad == cpuServ1 {
                        if *finalIP < ipServer1 {
                            minCpuIP = (*finalIP).clone();
                        } else {
                            minCpuIP = ipServer1.clone();
                        }
                    } else {
                        let mut min = myCpuLoad.min(cpuServ1);
                        min = min.min(cpuServ2);

                        if min == myCpuLoad {
                            minCpuIP = (*finalIP).clone();
                        } else if min == cpuServ1 {
                            minCpuIP = ipServer1.clone();
                        } else if min == cpuServ2 {
                            minCpuIP = ipServer2.clone();
                        }
                    }

                    // Finding if I will execute this image or no.
                    println!("Election Message number {}", curMsgId.clone());

                    if minCpuIP == *finalIP {
                        println!("I will execute this image ✔✔✔✔✔✔✔✔✔✔✔✔✔✔✔✔");
                        
                        ipServer1 = "empty".to_string();
                        ipServer2 = "empty".to_string();
                        cpuServ1 = -1.0;
                        cpuServ2 = -1.0;
                        CurrentElection = -1;
                        execQ.lock().unwrap().add(imgMsg.clone());
                        continue;
                    } else {
                        println!("I will not execute this image xxxxxxxxxxxxxxxxx");
                        
                        ipServer1 = "empty".to_string();
                        ipServer2 = "empty".to_string();
                        cpuServ1 = -1.0;
                        cpuServ2 = -1.0;
                        CurrentElection = -1;
                        continue;
                    }
                }
            
                //println!("Message from unknown sender recived.")
                if curMsg.online == true 
                {            
                    onlineIP = curMsg.sender_ip.clone(); 
                    println!("new client");                          		              
                 client_addresses.push(onlineIP); 
                }

                if curMsg.online ==  false {
                    // deltes the element equal to senderip
                    offlineIP = curMsg.sender_ip.clone();
                    client_addresses.retain(|x| x != &offlineIP);

                }

                if curMsg.dor_request == true 
                {
                    println!("sending-  Directory of Service:   {:?}", client_addresses); // we dont want to print hena we want to print this at the client side
                    
                    let addr_String = curMsg.sender_ip.to_string();
                    let DoRMessge = Message {
                        id: 1,
                        request: false,
                        reciver_id: 1, // To be changed
                        text: "failMsg".to_string(),
                        election: false,
                        cpu_load: 0.0,
                        msgID: "empty".to_string(),
                        sender_ip: format!("{}:{}", my_local_ip_wt, prtNum),
                        reciver_ip: addr_String.clone(),
                        fail_msg: true,
                        recoverey: false,
                        image_buffer: vec![0u8; 2],
                        num_image_bytes: 0,
                        online: true,  
                        dor_request: false, 
                        dor:false,
                        directory: client_addresses.clone(),
                        viewimg_request:false,
                        ClientToBeSentTo: "".to_string(),
                    };
                    let serialized_object = serde_json::to_string(&DoRMessge).unwrap();

                    socket
                        .send_to(&serialized_object.as_bytes(), addr_String)
                        .expect("Error on send");
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let portNum = &args[1];
    let portNum_cloned = portNum.clone();
    let portNum_cloned_2 = portNum.clone();
    let my_local_ip = local_ip().unwrap().to_string();

    let sys = System::new();

    println!("This is my local IP address: {:?}", my_local_ip);

    let fIP = my_local_ip.clone() + &":" + portNum;
    let socket = UdpSocket::bind(my_local_ip + &":" + portNum)?; // for UDP4/6

    //create 3 threads one for receving and one for sending and one for handling the encryption.

    let socket = socket.try_clone().expect("Failed to clone socket");
    let socket_cloned = socket.try_clone().expect("Failed to clone socket");
    let socket_cloned2 = socket.try_clone().expect("Failed to clone socket");

    let mut workQueue = Arc::new(Mutex::new(queue![]));
    let mut executeQueue = Arc::new(Mutex::new(queue![]));

    let mut workQueue_cloned = workQueue.clone();
    let mut workQueue_cloned2 = workQueue.clone();

    let mut executeQueueCopy = executeQueue.clone();

    thread::spawn(move || {
        handle_requests(socket, &mut workQueue);
    });

    thread::spawn(move || {
        execute_load(socket_cloned2,&mut executeQueue.clone(), &portNum_cloned_2);
    });

    thread::spawn(move || {
        workerThread(
            socket_cloned,
            &fIP,
            &mut workQueue_cloned,
            &portNum_cloned,
            &mut executeQueueCopy,
        );
    });

    loop {}
}
