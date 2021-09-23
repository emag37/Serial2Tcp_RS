#[allow(non_snake_case)]
mod binding;
mod parse;

fn main() {
    let active_bindings : Vec<binding::ActiveBinding> = parse::parse_cli().unwrap()
    .into_iter()
    .map(|c| {
        binding::start_workers(&c).unwrap()
    }).collect();

    println!("[MAIN]: Started {} workers. Press 'q' to exit.", active_bindings.len());
    let getch = getch::Getch::new();

    loop {
        match getch.getch() {
            Ok(b'q') => {
                println!("bye!");
                std::process::exit(0);
            }
            Ok(_) => {}
            Err(_)=> {println!("Error reading character...")}
        }
    }
}
