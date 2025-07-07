
mod game;

use chess_engine::DEFAULT_BOARD_HEIGHT;
use bevy::prelude::*;
use bevy::window::PresentMode;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Chess"),
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    resolution: Vec2::new(DEFAULT_BOARD_HEIGHT, DEFAULT_BOARD_HEIGHT).into(),
                    present_mode: PresentMode::AutoNoVsync, // Reduces input lag.
                    fit_canvas_to_parent: true,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(ImagePlugin::default_linear()), // default_nearest for pixel art
        game::Game
    ));

    app.run();
}
