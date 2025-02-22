## Zero Setup Hot Reload

When people tell you how to do hot reloading in Rust, the method usually involves manually creating a separate crate for the functions you want to hot-reload, so that you can build it separately.
This gets even more tedious when you have to share many types across both crates.

The point of this experiment is to find out if all this setup can be done automatically and transparently by a proc macro. The results are:

- Yes, see the example: go into `examples/basic` and run `cargo run`. You can edit `do_stuff` in `examples/basic/main.rs` and see the changes getting hot-reloaded.

  Within the example crate, you can just add `#[hot_reload]` on top of any regular function, and it Just Works™.

- However, because of poor `cargo` support for things like this, this relies on a few dumb tricks that can probably break in more complicated project setups. It also relies on symlinks, so this is all unix-only.
  
- However, the dylib-based hot reload strategy has many inherent problems and pitfalls. When I tried using this method on more complex functions calling `egui` and my own GUI library, what I got was mostly thread-local-storage related bugs and segfaults.
  
  I think someone who knows what they're doing could solve many of these issues, but probably not all of them.
  The true future of hot reloading probably doesn't look like this at all, but rather more like [In Place Binary Patching](https://github.com/jkelleyrtp/ipbp). 
