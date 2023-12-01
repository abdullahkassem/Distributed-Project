
use std::io;
use std::env;
use std::str;
use regex::Regex;
use std::time::{Instant, Duration};
use std::thread;
use std::net::UdpSocket;
use std::net::{Ipv4Addr, SocketAddrV4};

fn calculate_average_duration(duration_array: &[Duration]) -> Duration {
    let total_duration: Duration = duration_array.iter().sum();
    let average_duration = total_duration / duration_array.len() as u32;
    average_duration
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} server1:port1 server2:port2 server3:port3", args[0]);
        std::process::exit(1);
    }

    let server_addresses: Vec<&str> = args[1..].iter().map(|arg| &**arg).collect();
    //let server_addresses: Vec<&str> = ["10.65.192.73:2000","10.65.192.73:2001"];

    let mut array = Vec::new();
    let mut stime = vec![Instant::now(); 1000];
    let mut elapsed_time = vec![Duration::from_secs(0); 1000];

    for i in 0..1000 {
        array.push(format!("message{}", i));
    }

    //println!("{:?}", array);

    let re = Regex::new(r"\d+").unwrap();

    let multicast_group_address = Ipv4Addr::new(224, 0, 0, 1);
    // let socket1 = UdpSocket::bind("[::]:3000").unwrap();
    // let data = b"Hello, world!";
    // socket1.send_to(data, SocketAddrV4::new(multicast_group_address, 2000)).unwrap();

    // for server_address in &server_addresses {
        // let parts: Vec<&str> = server_address.split(':').collect();
        // if parts.len() != 2 {
        //     eprintln!("Invalid server address format: {}", server_address);
        //     continue;
        // }
        // let hostname = parts[0];
        // let port = parts[1];

        let socket = UdpSocket::bind("[::]:0")?; // for UDP4/6

        for i in 0..1000 {
            socket
                .send_to(array[i].as_bytes(), SocketAddrV4::new(multicast_group_address, 2000))
                .expect("Error on send");
            stime[i] = Instant::now();

            let mut buf = [0; 2048];
            socket.set_read_timeout(Some(Duration::from_secs(5)))?; // Set a 5-second timeout
            match socket.recv_from(&mut buf) {
                Ok((amt, _src)) => {
                    let end = Instant::now();
                    let echo = str::from_utf8(&buf[..amt]).unwrap();
                    elapsed_time[i] = end.duration_since(stime[i]);
                    let number = re.find(echo).unwrap().as_str().parse::<i32>().unwrap();
                    // println!("Echo {}, {}", echo, number);
                }
                Err(_) => {
                    println!("No response received. Timeout reached.");
                    elapsed_time[i] = Duration::from_secs(5); // Set elapsed time to timeout duration
                }
            }
        }
    // }

    let average_duration: Duration = calculate_average_duration(&elapsed_time);
    println!("Average duration: {:?}", average_duration);
    let count = elapsed_time.iter().filter(|&&x| x == Duration::from_secs(5)).count();
    println!("Timeout count: {}", count);

    Ok(())
}