mod game;

use game::chess_board::DEFAULT_BOARD_HEIGHT;
use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Chess"),
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    resolution: Vec2::new(DEFAULT_BOARD_HEIGHT, DEFAULT_BOARD_HEIGHT).into(),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(ImagePlugin::default_linear()), // default_nearest for pixel art
        game::Game
    ));

    app.run();
}
