//! Run CI locally on your computer.
//!
//! Adapted from [Bevy](https://github.com/bevyengine/bevy) (MIT).

mod args;
mod ci;
mod commands;
mod prepare;

pub use self::{ci::*, prepare::*};

fn main() {
    argh::from_env::<CI>().run();
}
