use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::result::Result;
use serialport::{SerialPort, COMPort};
use std::net::{TcpListener, TcpStream};
use std::io::{Read,Write};

#[derive(Default, Clone)]
pub struct BindingConfig {
    pub com_port: String,
    pub baud_rate: u32,
    pub tcp_host: String
}

pub struct ActiveBinding {
    pub config : BindingConfig,
    active : Arc<AtomicBool>,
    worker: std::thread::JoinHandle<()>,
}


impl ActiveBinding {
    pub fn destroy(mut self) {
        self.active.store(false, std::sync::atomic::Ordering::Release);
        let _ = self.worker.join();
    }
}

fn start_tcp_read_thread(config: &BindingConfig, active_flag: Arc<AtomicBool>, com: serialport::COMPort, tcp_listener: std::net::TcpListener) -> std::thread::JoinHandle<()> {
    let config = config.clone();
    std::thread::spawn(move || {
        println!("Waiting for a connection on {}", config.tcp_host);
        let (tcp_stream_write, _) = match tcp_listener.accept() {
            Ok(stream) => stream,
            Err(err) => {
                active_flag.store(false, std::sync::atomic::Ordering::Release);
                return
            }
        };
        
        println!("Got a connection, beginning relay {} <-> {}", config.tcp_host, config.com_port);
        let tcp_stream_read =  match tcp_stream_write.try_clone() {
            Ok(stream) => stream,
            Err(err) => {
                active_flag.store(false, std::sync::atomic::Ordering::Release);
                return
            }
        };
        
        let tcp_write_thread = start_tcp_write_thread(&config, active_flag.clone(), com.try_clone_native().unwrap(), tcp_stream_write);

        let mut buf : [u8;1500] = [0;1500];
        
        let mut tcp_stream_read = tcp_stream_read;
        let mut com_write = com;

        while active_flag.load(std::sync::atomic::Ordering::Acquire) {
            let n_read = match tcp_stream_read.read(&mut buf) {
                Ok(n) => n,
                Err(err) => {
                    println!("Error reading from TCP stream @ {}: {}", config.tcp_host, err);
                    break
                },
            };
            if n_read > 0 {
                match com_write.write(&mut buf[..n_read]) {
                    Ok(_) => {},
                    Err(err) => {
                        println!("Error writing to COM port {}: {}", config.com_port, err);
                        break
                    },
                }
            }
        }
        println!("relay {} <-> {} exited", config.tcp_host, config.com_port);
        active_flag.store(false, std::sync::atomic::Ordering::Release);
        let _ = tcp_write_thread.join();
    })
}

fn start_tcp_write_thread(config: &BindingConfig, active_flag: Arc<AtomicBool>, com: serialport::COMPort, tcp: TcpStream) -> std::thread::JoinHandle<()> {
    let config_for_tcp_writer = config.clone();
    let tcp_writer_flag_ref = active_flag.clone();
    std::thread::spawn(move || {
        let mut buf : [u8;1500] = [0;1500];

        let mut com_read = com;
        let mut tcp_stream_write = tcp;

        while tcp_writer_flag_ref.load(std::sync::atomic::Ordering::Acquire) {
            let n_read = match com_read.read(&mut buf){
                Ok(n_bytes) => n_bytes,
                Err(ec) => {println!("Error reading from port {}! {}", config_for_tcp_writer.com_port, ec); break}
            };
            match tcp_stream_write.write(&mut buf[..n_read]) {
                Ok(_) => {},
                Err(err) => {
                    println!("Error writing from host {}: {}", config_for_tcp_writer.tcp_host, err);
                }
            }
        }
        active_flag.store(false, std::sync::atomic::Ordering::Release);
    })
}
pub fn start_workers(config: &BindingConfig) -> Result<ActiveBinding, serialport::Error> {
    println!("Starting relay {} <-> {}", config.tcp_host, config.com_port);

    let active_flag = Arc::new(AtomicBool::new(true));

    let mut com_read = match serialport::new(&config.com_port, config.baud_rate).open_native() {
        Ok(port) => port,
        Err(err) => return Err(err),
    };
    let _ = com_read.set_timeout(std::time::Duration::from_secs(2));

    let tcp_listener = match std::net::TcpListener::bind(&config.tcp_host) {
        Ok(listener) => listener,
        Err(err) => return Err(serialport::Error::from(err))
    };

    let tcp_read_thread = start_tcp_read_thread(config, active_flag.clone(), com_read, tcp_listener);

    Ok(ActiveBinding{ 
        worker: tcp_read_thread,
        active : active_flag,
        config : config.clone(),
    })
}