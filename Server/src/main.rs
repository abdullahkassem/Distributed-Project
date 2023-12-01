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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: usize,
    pub reciver_id: usize,  // Not used till now
    pub sender_ip: String,  //source ip
    pub reciver_ip: String, //destination ip
    pub request: bool,      // if message is of type request
    pub text: String,       // text inside message
    pub election: bool,     //to start an election
    pub cpu_load: f32,      //contains the cpu load
    pub cpu_message: bool, //this means that this message is for sharing the cpu load with other servers
    pub fail_msg: bool,    // if recived from server that means that it has failed.
    pub recoverey: bool,   // Means that server has recovered.
    pub image_buffer: Vec<u8>,
    pub num_image_bytes: usize,
}


//thread to execute the workload
fn execute_load(
    workQ: &mut Arc<Mutex<queues::Queue<Message>>>,
    prtNum: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut EncryptionSuccessCnt = 0; let mut EncryptionFailCnt=0;
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

                // Save the image as a .jpg file
            image?
                .save("./received_image.jpg")
                .map_err(|e| format!("Error saving image: {}", e))?;
            println!("Received image saved as 'received_image.jpg'");

            let prtNum_cloned = prtNum.clone();

            let command = "steghide";
            let args = [
                "embed",
                "-cf",
                "super.jpg",
                "-ef",
                "received_image.jpg",
                "-sf",
                "output.jpg",
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
                EncryptionSuccessCnt+=1;
                println!("encrypted image successfully");
                println!("Num of successful Encryption: {}, Failures: {}",EncryptionSuccessCnt,EncryptionFailCnt);
                
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                EncryptionFailCnt+=1;
                println!("failed to encrypt image.");
            }
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
    let server_addresses: [&str; 3] = [
        "192.168.1.3:2000",
        "192.168.1.3:2001",
        "192.168.1.3:2002",
    ];
    let client_addresses: [&str; 3] = [
        "192.168.1.3:3000",
        "192.168.1.3:3001",
        "192.168.91.128:3002",
    ];

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

    let mut end = Instant::now();
    let mut secPassed = 0;

    let mut imgMsg: Message = Message {
        id: 1,
        reciver_id: 2,
        request: false,
        text: "hello".to_string(),
        election: false,
        cpu_load: 0.0,
        cpu_message: false,
        sender_ip: "0000".to_string(),
        reciver_ip: "0000".to_string(),
        image_buffer: vec![0u8; 2],
        num_image_bytes: 0,
        fail_msg: false,
        recoverey: false,
    };

    loop {
        end = Instant::now();

        // if secPassed < end.duration_since(start).as_secs(){
        //     println!("current time since start {:?}",end.duration_since(start).as_secs());
        //     secPassed = end.duration_since(start).as_secs();
        //     println!("iamdown variable = {}",IamDown);
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
                        cpu_message: false,
                        sender_ip: format!("{}:{}", my_local_ip_wt, prtNum),
                        reciver_ip: addr_String.clone(),
                        fail_msg: false,
                        recoverey: true,
                        image_buffer: vec![0u8; 2],
                        num_image_bytes: 0,
                    };

                    let serialized_object = serde_json::to_string(&recoverMsg).unwrap();

                    socket
                        .send_to(&serialized_object.as_bytes(), addr_String)
                        .expect("Error on send");
                }
            }
            start = Instant::now();
            secPassed = 0;
        } //else if it is not down it will periodically check if it has the lowest CPU Load
        else if !someoneFailed && end.duration_since(start) > Duration::from_secs(10) && IamDown == false {
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
                            cpu_message: false,
                            sender_ip: format!("{}:{}", my_local_ip_wt, prtNum),
                            reciver_ip: addr_String.clone(),
                            fail_msg: true,
                            recoverey: false,
                            image_buffer: vec![0u8; 2],
                            num_image_bytes: 0,
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

        let myCpuLoad = sys.load_average().unwrap().one;

        // Handle The messages in the queue
        if workQ.lock().unwrap().size() > 0 {
            let curMsg = workQ.lock().unwrap().peek().unwrap();
            workQ.lock().unwrap().remove();

            // Check if the sender ip is a server or a client
            if (server_addresses.contains(&curMsg.sender_ip.as_str())) {
                // This makes sure the server ignores and loses Any messages in queue after it fails
                // This applies to existing messages in queue when failing and messages recieved after failing.
                if failedIp == *finalIP {
                    println!("I failed so I will ignore");
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
                }

                if curMsg.cpu_message == true && someoneFailed == false {
                    cpuMsgCnt += 1;
                    if cpuMsgCnt == 1 {
                        cpuServ1 = curMsg.cpu_load;
                        idServ1 = curMsg.id;
                        ipServer1 = curMsg.sender_ip;
                    } else if cpuMsgCnt == 2 {
                        cpuServ2 = curMsg.cpu_load;
                        idServ2 = curMsg.id;
                        ipServer2 = curMsg.sender_ip;
                        cpuMsgCnt = 0;

                        // Finding the Min CPU IP - 1st by min CPU then by min IP+Port

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
                            // let min = min(min(myCpuLoad, cpuServ1), cpuServ2);
                            let mut min = myCpuLoad.min(cpuServ1);
                            min = min.min(cpuServ2);
                            // println!("min is {:.3}",min);

                            if min == myCpuLoad {
                                minCpuIP = (*finalIP).clone();
                            } else if min == cpuServ1 {
                                minCpuIP = ipServer1.clone();
                            } else if min == cpuServ2 {
                                minCpuIP = ipServer2.clone();
                            }
                        }

                        //println!("minCpuIP = {}", minCpuIP);

                        // Finding if I will execute this image or no.

                        if minCpuIP == *finalIP
                        {
                            println!("I will execute this image <=========");
                            cpuServ1 = -1.0;
                            cpuServ2 = -1.0;
                            CurrentElection = -1;
                            execQ.lock().unwrap().add(imgMsg.clone());
                            continue;
                        }else
                        {
                            println!("I will not execute this image <=========");
                            cpuServ1 = -1.0;
                            cpuServ2 = -1.0;
                            CurrentElection = -1;
                            continue;
                        }
                        
                    /*
                        if cpuServ1 < myCpuLoad {
                            println!("I will not execute this image <=========");
                            cpuServ1 = -1.0;
                            cpuServ2 = -1.0;
                            CurrentElection = -1;
                            continue;
                        } else if cpuServ2 < myCpuLoad {
                            println!("I will not execute this image <=========");
                            cpuServ1 = -1.0;
                            cpuServ2 = -1.0;
                            CurrentElection = -1;
                            continue;
                        } else {
                            if cpuServ1 == myCpuLoad && myCpuLoad == cpuServ2 {
                                if finalIP < &ipServer1 && finalIP < &ipServer2 {
                                    println!("I will execute this image <=========");
                                    cpuServ1 = -1.0;
                                    cpuServ2 = -1.0;
                                    CurrentElection = -1;
                                    execQ.lock().unwrap().add(imgMsg.clone());
                                    continue;
                                } else {
                                    println!("I will not execute this image <=========");
                                    cpuServ1 = -1.0;
                                    cpuServ2 = -1.0;
                                    CurrentElection = -1;
                                    continue;
                                }
                            }

                            if cpuServ1 == myCpuLoad {
                                //compare ip and port if mine is less i will execute
                                if finalIP < &ipServer1 {
                                    println!("I will execute this image <=========");
                                    cpuServ1 = -1.0;
                                    cpuServ2 = -1.0;
                                    CurrentElection = -1;
                                    execQ.lock().unwrap().add(imgMsg.clone());
                                    continue;
                                } else {
                                    println!("I will not execute this image <=========");
                                    cpuServ1 = -1.0;
                                    cpuServ2 = -1.0;
                                    CurrentElection = -1;
                                    continue;
                                }
                            }

                            if cpuServ2 == myCpuLoad {
                                //compare ip and port if mine is less i will execute
                                if finalIP < &ipServer2 {
                                    println!("I will execute this image <=========");
                                    cpuServ1 = -1.0;
                                    cpuServ2 = -1.0;
                                    CurrentElection = -1;
                                    execQ.lock().unwrap().add(imgMsg.clone());
                                    continue;
                                } else {
                                    println!("I will not execute this image <=========");
                                    cpuServ1 = -1.0;
                                    cpuServ2 = -1.0;
                                    CurrentElection = -1;
                                    continue;
                                }
                            }

                            println!("I will execute this image <=========");
                            cpuServ1 = -1.0;
                            cpuServ2 = -1.0;
                            CurrentElection = -1;
                            execQ.lock().unwrap().add(imgMsg.clone());
                            continue;
                        }
                    */

                    }
                } // If a server is down we will only consider 1 other server, in our election. 
                else if curMsg.cpu_message == true && someoneFailed == true {
                    println!("Some one failed.");

                    cpuServ1 = curMsg.cpu_load;
                    idServ1 = curMsg.id;
                    ipServer1 = curMsg.sender_ip;

                    if myCpuLoad > cpuServ1 {
                        println!("I will not execute this image <-----------");
                        cpuServ1 = -1.0;
                        cpuServ2 = -1.0;
                        CurrentElection = -1;
                        continue;
                    } else if myCpuLoad == cpuServ1 {
                        if finalIP < &ipServer1 {
                            println!("I will execute this image <-------------");
                            cpuServ1 = -1.0;
                            cpuServ2 = -1.0;
                            CurrentElection = -1;
                            execQ.lock().unwrap().add(imgMsg.clone());
                            continue;
                        } else {
                            println!("I will not execute this image <-------------");
                            cpuServ1 = -1.0;
                            cpuServ2 = -1.0;
                            CurrentElection = -1;

                            continue;
                        }
                    } else {
                        println!("I will execute this image <-------------");
                        cpuServ1 = -1.0;
                        cpuServ2 = -1.0;
                        CurrentElection = -1;
                        execQ.lock().unwrap().add(imgMsg.clone());
                        continue;
                    }
                }
            } else if (client_addresses.contains(&curMsg.sender_ip.as_str())) {

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
                    CurrentElection = 1;
                    imgMsg = curMsg.clone();


                    for addr in server_addresses.iter() {
                        let addr_String = addr.to_string();

                        if addr_String != format!("{}:{}", my_local_ip_wt, prtNum) {
                            let ElectMessage = Message {
                                id: 1,
                                request: false,
                                reciver_id: 1, // To be changed
                                text: "hello".to_string(),
                                election: false,
                                cpu_load: myCpuLoad,
                                cpu_message: true,
                                sender_ip: format!("{}:{}", my_local_ip_wt, prtNum),
                                reciver_ip: addr_String.clone(),
                                fail_msg: false,
                                recoverey: false,
                                image_buffer: vec![0u8; 2],
                                num_image_bytes: 0,
                            };

                            let serialized_object = serde_json::to_string(&ElectMessage).unwrap();

                            socket
                                .send_to(&serialized_object.as_bytes(), addr_String)
                                .expect("Error on send");
                        }
                    }
                }
            } else {
                println!("Message from unknown sender recived.")
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

    let mut workQueue = Arc::new(Mutex::new(queue![]));
    let mut executeQueue = Arc::new(Mutex::new(queue![]));

    let mut workQueue_cloned = workQueue.clone();
    let mut workQueue_cloned2 = workQueue.clone();

    let mut executeQueueCopy = executeQueue.clone();

    thread::spawn(move || {
        handle_requests(socket, &mut workQueue);
    });

    thread::spawn(move || {
        execute_load(&mut executeQueue.clone(), &portNum_cloned_2);
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
