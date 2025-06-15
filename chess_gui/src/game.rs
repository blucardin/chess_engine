use bevy::input::common_conditions::*;
use bevy::prelude::*;

use chess_engine::*;
use chess_engine::Move;
use board_evaluator::ChessEngine;

pub const WHITE_TILE_COLOR: Color = Color::srgb_u8(254, 207, 159);

pub const BLACK_TILE_COLOR: Color = Color::srgb_u8(210, 140, 69);

pub const POSSIBLE_MOVE_HIGHLIGHT_COLOR: Color = Color::srgba_u8(32, 194, 29, 255 / 4);
pub const PROMOTION_BACKGROUND_COLOR: Color = Color::srgb_u8(45, 45, 45);

struct ChessGameResource {
    board : Board, 
    engine: ChessEngine
}

pub struct Game;

impl Plugin for Game {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameSettings {
            computer_player: true,
        });
        app.insert_non_send_resource(ChessGameResource { board : Board::new(PieceColor::White), engine: ChessEngine::new() });
        app.insert_state(ComputerTurnState::Player);
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            (
                mouse_button_input
                .run_if(input_just_pressed(MouseButton::Left))
                .run_if(in_state(ComputerTurnState::Player)),
            computer_move
                .run_if(in_state(ComputerTurnState::Computer))
            )
        );
    }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum ComputerTurnState {
    Computer,
    Player
}

#[derive(Resource, Clone)]
struct GameSettings {
    computer_player: bool,
}


fn generate_transform_for_board_gui(
    window_width: f32,
    window_height: f32,
    tile_size_x: f32,
    tile_size_y: f32,
    x: usize,
    y: usize,
) -> Transform {
    Transform::from_xyz(
        -(window_width / 2.) + (x as f32 * tile_size_x) + (tile_size_x / 2.),
        (window_height / 2.) - (y as f32 * tile_size_y) - (tile_size_y / 2.),
        0.0,
    )
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut board_resource: NonSendMut<ChessGameResource>,
    window: Single<&mut Window>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);

    let width = window.width();
    let height = window.height();

    let size_x = width / BOARD_TILE_DIM as f32;
    let size_y = height / BOARD_TILE_DIM as f32;

    for (idx, row) in board_resource.board.squares.iter_mut().enumerate() {
        for (idy, _) in row.iter_mut().enumerate() {
            // load the tile that the piece is on
            let color = if (idx + idy) % 2 == 0 {
                WHITE_TILE_COLOR
            } else {
                BLACK_TILE_COLOR
            };

            let transform =
                generate_transform_for_board_gui(width, height, size_x, size_y, idx, idy);

            commands.spawn((
                Mesh2d(meshes.add(Rectangle::new(size_x, size_y))),
                MeshMaterial2d(materials.add(color)),
                transform,
            ));
        }
    }

    draw_pieces(&mut commands, &board_resource, &window, &asset_server);
}

fn draw_pieces(
    commands: &mut Commands,
    board_resource: &NonSendMut<ChessGameResource>,
    window: &Single<&mut Window>,
    asset_server: &Res<AssetServer>,
) {
    let width = window.width();
    let height = window.height();

    let size_x = width / BOARD_TILE_DIM as f32;
    let size_y = height / BOARD_TILE_DIM as f32;

    for (idx, row) in board_resource.board.squares.iter().enumerate() {
        for (idy, square) in row.iter().enumerate() {
            let transform =
                generate_transform_for_board_gui(width, height, size_x, size_y, idx, idy);

            if let Square::Filled(piece) = square {
                commands.spawn((
                    PieceMarker,
                    Sprite {
                        image: asset_server.load(piece.get_asset_path()),
                        custom_size: Some(Vec2::new(size_x, size_y)),
                        ..default()
                    },
                    transform,
                ));
            }
        }
    }
}

#[derive(Component)]
struct PieceMarker;

#[derive(Component)]
struct PossibleMove {
    highlight_square: Coordinate,
    piece_move: Move,
}

#[derive(Component)]
struct Highlight;

#[derive(Component)]
struct PromotionPicker {
    piece_move: Move,
    board_position: Coordinate,
}

fn mouse_button_input(
    // buttons: Res<ButtonInput<MouseButton>>,
    mut board_resource: NonSendMut<ChessGameResource>,
    game_settings: Res<GameSettings>,
    window: Single<&mut Window>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    current_highlights: Query<Entity, With<Highlight>>,
    possible_moves: Query<&PossibleMove>,
    gui_pieces: Query<Entity, With<PieceMarker>>,
    promotions: Query<&PromotionPicker>,
    promotion_pickers: Query<Entity, With<PromotionPicker>>,
    asset_server: Res<AssetServer>,
    mut next_computer_turn_state: ResMut<NextState<ComputerTurnState>>,
) {
    let width = window.width();
    let height = window.height();

    let size_x = width / BOARD_TILE_DIM as f32;
    let size_y = height / BOARD_TILE_DIM as f32;

    if let Some(click_position) = window.cursor_position() {
        // convert click_position to board position
        let board_click_position = Coordinate {
            x: (click_position.x / size_x) as isize,
            y: (click_position.y / size_y) as isize,
        };

        let mut moved = false;

        if !promotions.is_empty() {
            for promotion in promotions.iter() {
                if promotion.board_position == board_click_position {
                    board_resource.board.apply_move(
                        &promotion.piece_move,
                    );

                    // despawn all the move picker elements
                    for promotion_picker in promotion_pickers.iter() {
                        commands.entity(promotion_picker).despawn();
                    }

                    moved = true;
                    break;
                }
            }

            if !moved {
                return;
            }
        }

        for possible_move in possible_moves.iter() {
            if board_click_position == possible_move.highlight_square {
                if let Move::Promote {
                    initial_position,
                    final_position,
                    move_type,
                    ..
                } = possible_move.piece_move
                {
                    // remove all the highlights
                    current_highlights
                        .iter()
                        .for_each(|current_highlight| commands.entity(current_highlight).despawn());

                    let going_up = board_resource.board.pawn_going_up();
                    let direction: isize = if going_up { -1 } else { 1 };

                    for (idx, piece_person) in POSSIBLE_PAWN_PROMOTES.iter().enumerate() {
                        let picker_position_x = final_position.x as usize;
                        let picker_position_y =
                            (final_position.y + ((-1 * direction) * idx as isize)) as usize;

                        let transform = generate_transform_for_board_gui(
                            width,
                            height,
                            size_x,
                            size_y,
                            picker_position_x,
                            picker_position_y,
                        );

                        // don't use highlight, use another marker component so things don't get lost

                        commands.spawn((
                            Highlight,
                            Mesh2d(meshes.add(Rectangle::new(size_x, size_y))),
                            MeshMaterial2d(materials.add(PROMOTION_BACKGROUND_COLOR)),
                            transform,
                        ));

                        commands.spawn((
                            PromotionPicker {
                                piece_move: Move::Promote {
                                    initial_position,
                                    final_position,
                                    move_type,
                                    piece_person: *piece_person,
                                },
                                board_position: Coordinate {
                                    x: picker_position_x as isize,
                                    y: picker_position_y as isize,
                                },
                            },
                            Sprite {
                                image: asset_server.load(format_piece_filename(
                                    board_resource.board.turn.file_string(),
                                    piece_person.file_string(),
                                )),
                                custom_size: Some(Vec2::new(size_x, size_y)),
                                ..default()
                            },
                            transform,
                        ));
                    }
                    return;
                }

                // // remove all the highlights
                // current_highlights
                //     .iter()
                //     .for_each(|current_highlight| commands.entity(current_highlight).despawn());

                board_resource.board.apply_move(
                    &possible_move.piece_move,
                );

                moved = true;
                break;
            }
        }

        // remove all the highlights
        current_highlights
            .iter()
            .for_each(|current_highlight| commands.entity(current_highlight).despawn());

        if moved {
            let game_state = board_resource.board.outcome();

            // remove all the pieces
            gui_pieces
                .iter()
                .for_each(|gui_piece| commands.entity(gui_piece).despawn());

            // redraw all the pieces
            draw_pieces(&mut commands, &board_resource, &window, &asset_server);

            match game_state {
                GameState::Playing => {}
                GameState::Checkmate { winner } => {
                    let text = match winner {
                        PieceColor::Black => "Checkmate, winner is Black",
                        PieceColor::White => "Checkmate, winner is White",
                    };
                    commands.spawn((
                        Text::new(text),
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(12.0),
                            left: Val::Px(12.0),
                            ..default()
                        },
                    ));
                    return;
                }
                GameState::Draw => {
                    commands.spawn((
                        Text::new("DRAW"),
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(12.0),
                            left: Val::Px(12.0),
                            ..default()
                        },
                    ));
                    return;
                }
            }


            if game_settings.computer_player {
                next_computer_turn_state.set(ComputerTurnState::Computer);
            }

            return;
        }

        if let Some(moves) = board_resource.board.get_possible_moves(board_click_position) {
            commands.spawn((
                Highlight,
                Mesh2d(meshes.add(Rectangle::new(size_x, size_y))),
                MeshMaterial2d(materials.add(POSSIBLE_MOVE_HIGHLIGHT_COLOR)),
                generate_transform_for_board_gui(
                    width,
                    height,
                    size_x,
                    size_y,
                    board_click_position.x as usize,
                    board_click_position.y as usize,
                ),
            ));

            let mut promotion_coordinates: Vec<Coordinate> = vec![];

            for piece_move in moves {
                let highlight_coordinate = match piece_move {
                    Move::Regular { final_position, .. }
                    | Move::EnPassant { final_position, .. } => final_position,
                    Move::Promote { final_position, .. } => {
                        if !promotion_coordinates.contains(&final_position) {
                            promotion_coordinates.push(final_position);
                            final_position
                        } else {
                            continue;
                        }
                    }
                    Move::Castle { ref side } => {
                        let final_y = if board_resource.board.pawn_going_up() {
                            BOARD_TILE_DIM - 1
                        } else {
                            0
                        };
                        let final_x = match side {
                            Side::QueenSide => 2,
                            Side::KingsSide => BOARD_TILE_DIM - 2,
                        };

                        Coordinate {
                            x: final_x,
                            y: final_y,
                        }
                    }
                };

                commands.spawn((
                    Highlight,
                    PossibleMove {
                        piece_move,
                        highlight_square: highlight_coordinate,
                    },
                    Mesh2d(meshes.add(Rectangle::new(size_x, size_y))),
                    MeshMaterial2d(materials.add(POSSIBLE_MOVE_HIGHLIGHT_COLOR)),
                    generate_transform_for_board_gui(
                        width,
                        height,
                        size_x,
                        size_y,
                        highlight_coordinate.x as usize,
                        highlight_coordinate.y as usize,
                    ),
                ));
            }
        }
    } else {
        println!("Cursor is not in the game window.");
    }
}

fn computer_move(
    // buttons: Res<ButtonInput<MouseButton>>,
    mut board_resource: NonSendMut<ChessGameResource>,
    game_settings: Res<GameSettings>,
    window: Single<&mut Window>,
    mut commands: Commands,
    gui_pieces: Query<Entity, With<PieceMarker>>,
    asset_server: Res<AssetServer>,
    mut next_computer_turn_state: ResMut<NextState<ComputerTurnState>>,
) {
    // let computer_move = board_resource.board.find_computer_move(board_resource.board.get_all_moves_for_turn());
    
    let computer_move = board_resource.engine.next_best_move_minimax(&board_resource.board, 1);
    
    // println!("computer_move: {:?}", computer_move);
    board_resource.board.apply_move(
        &computer_move,
    );

    // remove all the pieces
    gui_pieces
        .iter()
        .for_each(|gui_piece| commands.entity(gui_piece).despawn());

    // redraw all the pieces
    draw_pieces(&mut commands, &board_resource, &window, &asset_server);

    let game_state = board_resource.board.outcome();

    // todo: extract this as a function
    match game_state {
        GameState::Playing => {}
        GameState::Checkmate { winner } => {
            let text = match winner {
                PieceColor::Black => "Checkmate, winner is Black",
                PieceColor::White => "Checkmate, winner is White",
            };
            commands.spawn((
                Text::new(text),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(12.0),
                    left: Val::Px(12.0),
                    ..default()
                },
            ));
            return;
        }
        GameState::Draw => {
            commands.spawn((
                Text::new("DRAW"),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(12.0),
                    left: Val::Px(12.0),
                    ..default()
                },
            ));
            return;
        }
    }

    next_computer_turn_state.set(ComputerTurnState::Player);
}