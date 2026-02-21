mod help;
mod loop_cmd;
mod nowplaying;
mod pause;
mod play;
mod queue;
mod remove;
mod resume;
mod shuffle;
mod skip;
mod stop;
pub mod tarkov;
mod volume;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    let mut cmds = vec![
        help::help(),
        play::play(),
        play::p(),
        skip::skip(),
        skip::s(),
        stop::stop(),
        stop::st(),
        queue::queue(),
        queue::q(),
        pause::pause(),
        pause::pa(),
        resume::resume(),
        resume::r(),
        nowplaying::nowplaying(),
        nowplaying::np(),
        loop_cmd::loop_cmd(),
        loop_cmd::l(),
        shuffle::shuffle(),
        shuffle::sh(),
        remove::remove(),
        remove::rm(),
        volume::volume(),
        volume::v(),
    ];
    cmds.extend(tarkov::all());
    cmds
}
