#[allow(non_snake_case)]
mod binding;
mod parse;

fn main() {
    let active_bindings : Vec<binding::ActiveBinding> = parse::parse_cli().unwrap()
    .into_iter()
    .map(|c| {
        binding::start_workers(&c).unwrap()
    }).collect();

    println!("Started {} workers...", active_bindings.len());
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
