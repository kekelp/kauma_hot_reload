use hot_reload_macro::hot_reload;

pub struct State {
    pub counter: usize,
}

#[hot_reload]
pub fn do_stuff(state: &mut State) {
    state.counter += 1;
    println!("Doing stuff in iteration {}", state.counter);
}

fn main() {
    let mut state = State { counter: 0 };
    loop {
        do_stuff(&mut state);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
