use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::result::Result;
use serialport::{SerialPort};
use std::net::{TcpStream};
use std::io::{Read,Write};
use std::option::Option;
use std::fmt;

#[derive(Default, Clone)]
pub struct BindingConfig {
    pub com_port: String,
    pub baud_rate: u32,
    pub tcp_host: String
}

pub struct ActiveBinding {
    pub config : BindingConfig,
    active : Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Drop for ActiveBinding {
    fn drop(&mut self) {
        self.active.store(false, std::sync::atomic::Ordering::Release);
        let _ = self.worker.take().unwrap().join();
    }
}

impl fmt::Display for BindingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} <-> {})", self.tcp_host, self.com_port)
    }
}

fn start_tcp_read_thread(config: &BindingConfig, active_flag: Arc<AtomicBool>, tcp_listener: std::net::TcpListener) -> std::thread::JoinHandle<()> {
    let config = config.clone();

    std::thread::spawn(move || {
        while active_flag.load(std::sync::atomic::Ordering::Acquire) {
            let mut com = match serialport::new(&config.com_port, config.baud_rate).open_native() {
                Ok(port) => port,
                Err(err) => {
                    println!("{}: Failed to open com port: {}, retry in 2 seconds...", config, err);
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    continue
                },
            };
            let _ = com.set_timeout(std::time::Duration::from_secs(2));
        
            println!("{}: Waiting for a connection", config);
            let (tcp_stream_write, _) = match tcp_listener.accept() {
                Ok(stream) => stream,
                Err(err) => {
                    println!("{}: Error accepting TCP connection: {}", config, err);
                    continue
                }
            };
            
            println!("{}: Got a connection, beginning relay.", config);
            let mut tcp_stream_read =  match tcp_stream_write.try_clone() {
                Ok(stream) => stream,
                Err(err) => {
                    println!("Error cloning TCP stream for read: {}", err);
                    continue
                }
            };
            
            let writer_active_flag = Arc::new(AtomicBool::new(true));
            let tcp_write_thread = start_tcp_write_thread(&config, writer_active_flag.clone(), Box::new(com.try_clone_native().unwrap()), tcp_stream_write);

            let mut buf : [u8;1500] = [0;1500];
            let mut com_write = com.try_clone_native().unwrap();

            loop {
                match tcp_stream_read.read(&mut buf) {
                    Ok(0) => {},
                    Ok(n) => {
                        match com_write.write(&mut buf[..n]) {
                            Ok(_)=> {},
                            Err(err) => {
                                println!("{}: Error writing to COM port: {}", config, err);
                                break
                            }
                        }
                    },
                    Err(err) => {
                        println!("{}: Error reading from TCP stream: {}", config, err);
                        break
                    },
                };
            }

            std::mem::drop(com);
            std::mem::drop(tcp_stream_read);
            writer_active_flag.store(false, std::sync::atomic::Ordering::Release);
            let _ = tcp_write_thread.join();
        }
        
        println!("{}: exited", config);
        active_flag.store(false, std::sync::atomic::Ordering::Release);   
    })
}

fn start_tcp_write_thread(config: &BindingConfig, active_flag: Arc<AtomicBool>, com: Box<dyn serialport::SerialPort>, tcp: TcpStream) -> std::thread::JoinHandle<()> {
    let config_for_tcp_writer = config.clone();

    std::thread::spawn(move || {
        let mut buf : [u8;1500] = [0;1500];
        let mut com_read = com;
        let mut tcp_stream_write = tcp;
        let _ = tcp_stream_write.set_nodelay(true);
        
        while active_flag.load(std::sync::atomic::Ordering::Acquire) {
            match com_read.read(&mut buf) {
                Ok(0) => {},
                Ok(n_bytes) => {
                    match tcp_stream_write.write(&mut buf[..n_bytes]) {
                        Ok(_) => {},
                        Err(err) => {
                            println!("{}: Error writing from host: {}", config_for_tcp_writer, err);
                        }
                    }
                },
                Err(ec) => {
                    if ec.kind() != std::io::ErrorKind::TimedOut {
                        println!("{}: Error reading from port: {}", config_for_tcp_writer, ec);
                    }
                }
            };
        }
    })
}
pub fn start_workers(config: &BindingConfig) -> Result<ActiveBinding, serialport::Error> {
    println!("{}: Starting relay", config);

    let active_flag = Arc::new(AtomicBool::new(true));

    let tcp_listener = match std::net::TcpListener::bind(&config.tcp_host) {
        Ok(listener) => listener,
        Err(err) => return Err(serialport::Error::from(err))
    };

    let tcp_read_thread = start_tcp_read_thread(config, active_flag.clone(), tcp_listener);

    Ok(ActiveBinding{ 
        worker: Option::from(tcp_read_thread),
        active : active_flag,
        config : config.clone(),
    })
}