use bevy::prelude::*;
use bevy::sprite::{Wireframe2dConfig, Wireframe2dPlugin};
const BOARD_TILE_DIM: i32 = 8;
const DEFAULT_BOARD_TILE_SIZE: f32 = 100.0;
const DEFAULT_BOARD_HEIGHT: f32 = (BOARD_TILE_DIM) as f32 * DEFAULT_BOARD_TILE_SIZE;

const WHITE_TILE_COLOR: Color = Color::srgb_u8(254, 207, 159);

const BLACK_TILE_COLOR: Color = Color::srgb_u8(210, 140, 69);

const PIECES_FOLDER: &str = "pieces-basic-png";

#[derive(Debug, Clone, Copy)]
enum PieceColor {
    Black,
    White,
}

#[derive(Debug, Clone, Copy)]
enum PiecePerson {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Debug, Clone, Copy)]
struct Piece {
    color: PieceColor,
    piece_person: PiecePerson,
    id: Option<Entity>,
}

impl Piece {
    fn new(color: PieceColor, piece_person: PiecePerson) -> Option<Self> {
        Option::Some(Piece {
            color,
            piece_person,
            id: None,
        })
    }
    fn get_asset_path(&self) -> String {
        let name: &str = match self.piece_person {
            PiecePerson::Pawn => "pawn",
            PiecePerson::Rook => "rook",
            PiecePerson::Knight => "knight",
            PiecePerson::Bishop => "bishop",
            PiecePerson::Queen => "queen",
            PiecePerson::King => "king",
        };
        let color: &str = match self.color {
            PieceColor::Black => "black",
            PieceColor::White => "white",
        };
        format!("{}/{}-{}.png", PIECES_FOLDER, color, name)
    }
}

#[derive(Resource)]
struct Board {
    turn: PieceColor,
    pieces: [[Option<Piece>; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize],
}

// custom implementation for unusual values
impl Default for Board {
    fn default() -> Self {
        let mut pieces: [[Option<Piece>; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize] =
            [[None; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];
        for (idx, person) in [PiecePerson::Rook, PiecePerson::Knight, PiecePerson::Bishop]
            .iter()
            .enumerate()
        {
            pieces[idx] = [
                Piece::new(PieceColor::Black, *person),
                Piece::new(PieceColor::Black, PiecePerson::Pawn),
                None,
                None,
                None,
                None,
                Piece::new(PieceColor::White, PiecePerson::Pawn),
                Piece::new(PieceColor::White, *person),
            ];
        }

        pieces[3] = [
            Piece::new(PieceColor::Black, PiecePerson::King),
            Piece::new(PieceColor::Black, PiecePerson::Pawn),
            None,
            None,
            None,
            None,
            Piece::new(PieceColor::White, PiecePerson::Pawn),
            Piece::new(PieceColor::White, PiecePerson::Queen),
        ];
        pieces[4] = [
            Piece::new(PieceColor::Black, PiecePerson::Queen),
            Piece::new(PieceColor::Black, PiecePerson::Pawn),
            None,
            None,
            None,
            None,
            Piece::new(PieceColor::White, PiecePerson::Pawn),
            Piece::new(PieceColor::White, PiecePerson::King),
        ];

        for (idx, person) in [PiecePerson::Rook, PiecePerson::Knight, PiecePerson::Bishop]
            .iter()
            .rev()
            .enumerate()
        {
            pieces[idx + 5] = [
                Piece::new(PieceColor::Black, *person),
                Piece::new(PieceColor::Black, PiecePerson::Pawn),
                None,
                None,
                None,
                None,
                Piece::new(PieceColor::White, PiecePerson::Pawn),
                Piece::new(PieceColor::White, *person),
            ];
        }

        Board {
            turn: PieceColor::White,
            pieces,
        }
    }
}

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
        Wireframe2dPlugin::default(),
    ))
    .insert_resource(Board {
        ..Default::default()
    })
    .add_systems(Startup, setup);
    // #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, toggle_wireframe);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut board: ResMut<Board>,
    window: Single<&mut Window>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);

    let width = window.width();
    let height = window.height();

    let sizex = width / BOARD_TILE_DIM as f32;
    let sizey = height / BOARD_TILE_DIM as f32;

    for (idx, row) in board.pieces.iter_mut().enumerate() {
        for (idy, item) in row.iter_mut().enumerate() {
            // load the tile that the piece is on
            let color = if (idx + idy) % 2 == 0 {
                WHITE_TILE_COLOR
            } else {
                BLACK_TILE_COLOR
            };

            let transform = Transform::from_xyz(
                // Distribute shapes from -X_EXTENT/2 to +X_EXTENT/2.
                -(width / 2.) + (idx as f32 * sizex) + (sizex / 2.),
                (height / 2.) - (idy as f32 * sizey) - (sizey / 2.),
                0.0,
            );

            commands.spawn((
                Mesh2d(meshes.add(Rectangle::new(sizex, sizey))),
                MeshMaterial2d(materials.add(color)),
                transform,
            ));

            match item {
                Some(piece) => {
                    // load the piece sprite using commands and store the id of the piece in the board resource
                    piece.id = Option::Some(
                        commands
                            .spawn((
                                Sprite {
                                    image: asset_server.load(piece.get_asset_path()),
                                    custom_size: Some(Vec2::new(sizex, sizey)),
                                    ..default()
                                },
                                transform,
                            ))
                            .id(),
                    );
                }
                None => {
                    // Do nothing
                }
            }
        }
    }

    // #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        Text::new("Press space to toggle wireframes"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

// #[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<Wireframe2dConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}
