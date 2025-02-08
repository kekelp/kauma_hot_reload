use libloading::{Library, Symbol};

pub struct State {
    pub counter: usize,
}

// fn do_stuff(state: &mut State) {
//     // Load the shared library
//     let lib = unsafe {
//         Library::new("hot_stuff/target/debug/libhot_test2.so").unwrap()
//     };

//     // Load the function symbol
//     unsafe {
//         let func: Symbol<unsafe extern "C" fn(&mut State)> = lib.get(b"do_stuff").unwrap();
//         func(state); // Call the function
//     }
// }
#[no_mangle]
pub fn do_stuff(state: &mut State) {
    state.counter += 1;
    println!("doing stuff in iteration {}", state.counter);
}

fn main() {
    let mut state = State { counter: 0 };
    loop {
        do_stuff(&mut state);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
