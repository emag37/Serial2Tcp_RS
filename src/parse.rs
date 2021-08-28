use clap::{Arg, App};
use std::vec::Vec;
use std::string::String;
use ini::*;

use crate::binding::BindingConfig;

pub fn parse_cli() -> Result<Vec<BindingConfig>, ini::Error> {
    let arg_matches = App::new("Serial2Tcp")
    .version("0.1")
    .author("Eric M.")
    .about("Relays Serial data to/from a TCP socket")
    .arg(Arg::with_name("config")
         .short("c")
         .long("config")
         .value_name("INI file")
         .conflicts_with_all(&["baudrate","host", "comport"])
         .help("Load binding configs from an INI file")
         .takes_value(true))
    .arg(Arg::with_name("baudrate")
         .short("b")
         .long("baudrate")
         .value_name("Baudrate")
         .conflicts_with("config")
         .default_value_if("host", None, "115200")
         .help("Baudrate for COM port")
         .takes_value(true))
    .arg(Arg::with_name("host")
         .short("h")
         .long("host")
         .value_name("TCP Host")
         .required_unless("config")
         .help("TCP host address and port, format : x.x.x.x:<port>")
         .takes_value(true))
    .arg(Arg::with_name("comport")
         .short("p")
         .long("comport")
         .value_name("COM Port")
         .required_unless("config")
         .help("COM port, i.e: COMx")
         .takes_value(true))
    .get_matches();
    
    if arg_matches.is_present("config") {
        return parse_ini(arg_matches.value_of("config").unwrap());
    }


    Ok(vec!(BindingConfig {
        baud_rate: arg_matches.value_of("baudrate").unwrap_or("115200").parse::<u32>().unwrap_or(115200),
        com_port: String::from(arg_matches.value_of("comport").unwrap()),
        tcp_host: String::from(arg_matches.value_of("host").unwrap())

    }))
}

fn parse_ini(path: &str) -> Result<Vec<BindingConfig>, ini::Error> {
    let loaded_ini = match Ini::load_from_file(path) {
        Ok(ini) => ini,
        Err(err) => return Err(err),
    };

    let mut ret_vec = Vec::new();

    for (sec, prop) in loaded_ini.iter() {
        match sec {
            Some("relay") => { 
                let mut new_config = BindingConfig::default();
                for (k, v) in prop.iter() {
                    match k.to_lowercase().as_str() {
                        "com" => {new_config.com_port = String::from(v);}
                        "baud" => {new_config.baud_rate = v.parse::<u32>().unwrap_or(0);}
                        "host" => {new_config.tcp_host = String::from(v);}
                        &_ => {}
                    }
                }
                if new_config.baud_rate == 0 {
                    new_config.baud_rate = 115200;
                }
                ret_vec.push(new_config);
            }
            Some(arg) => println!("Unknown parameter: {}", arg),
            None => {}
        }
    }

    Ok(ret_vec)
}