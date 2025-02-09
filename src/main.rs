use kauma_hot_reload::hot_reload;

pub struct State {
    pub counter: usize,
}

impl State {
    #[hot_reload]
    pub fn do_stuff(&mut self) {
        self.counter += 1;
        println!("Doing stuff in iteration {}", self.counter);
    }
}

fn main() {
    let mut state = State { counter: 0 };
    loop {
        state.do_stuff();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
