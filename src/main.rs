use process::device::get_default_device;

fn main() {
    if let Ok(d) = get_default_device() {
        println!("{:#?}", unsafe { d.GetId().unwrap() });
    }
}
