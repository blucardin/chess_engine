use bevy::color::palettes::basic::RED;
use bevy::input::common_conditions::*;
use bevy::prelude::*;
use bevy::ui::widget::ImageNodeSize;
use board_evaluator::ChessEngine;
use chess_engine::Move;
use chess_engine::*;

pub const WHITE_TILE_COLOR: Color = Color::srgb_u8(254, 207, 159);

pub const BLACK_TILE_COLOR: Color = Color::srgb_u8(210, 140, 69);

pub const POSSIBLE_MOVE_HIGHLIGHT_COLOR: Color = Color::srgba_u8(32, 194, 29, 255 / 4);
pub const PROMOTION_BACKGROUND_COLOR: Color = Color::srgb_u8(45, 45, 45);

struct ChessGameResource {
    board: Board,
    engine: ChessEngine,
}

pub struct Game;

impl Plugin for Game {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameSettings {
            computer_player: true,
        });
        app.insert_non_send_resource(ChessGameResource {
            board: Board::new(PieceColor::White),
            engine: ChessEngine::new(100_000),
        });
        app.insert_state(ScreenState::HomeScreen);
        app.insert_state(TurnState::Player1Turn);
        app.insert_state(GameMode::PlayerVsComputer);
        app.add_systems(Startup, setup_camera);
        app.add_systems(
            Update,
            (
                button_system.run_if(in_state(ScreenState::HomeScreen)),
                mouse_button_input
                    .run_if(input_just_pressed(MouseButton::Left))
                    .run_if(not(in_state(TurnState::ComputerTurn)))
                    .run_if(in_state(ScreenState::GameScreen)),
                computer_move.run_if(in_state(TurnState::ComputerTurn)),
            ),
        );

        // setup_home_screen
        app.add_systems(OnEnter(ScreenState::HomeScreen), setup_home_screen);
        app.add_systems(OnExit(ScreenState::HomeScreen), tear_down_home_screen);

        app.add_systems(
            OnEnter(ScreenState::GameScreen),
            (setup_board, re_draw_pieces),
        );
        // app.add_systems(OnEnter(TurnState::ComputerTurn), re_draw_pieces);
        // app.add_systems(OnEnter(TurnState::Player1Turn), re_draw_pieces);
        // app.add_systems(OnEnter(TurnState::Player2Turn), re_draw_pieces);
        app.add_systems(OnExit(TurnState::Player1Turn), check_if_game_over);
        app.add_systems(OnExit(TurnState::ComputerTurn), check_if_game_over);
        app.add_systems(OnExit(TurnState::Player2Turn), check_if_game_over);
    }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum TurnState {
    ComputerTurn,
    Player1Turn,
    Player2Turn,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum ScreenState {
    HomeScreen,
    GameScreen,
    PausedScreen,
    GameTerminationScreen,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameMode {
    PlayerVsComputer,
    PlayerVsPlayer,
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

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Component, Debug)]
struct BoardSetupSettings {
    main_player: PieceColor,
    game_mode: GameMode,
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
            &BoardSetupSettings,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_screen_state: ResMut<NextState<ScreenState>>,
    mut next_turn_state: ResMut<NextState<TurnState>>,
    mut next_game_mode_state: ResMut<NextState<GameMode>>,
    mut board_resource: NonSendMut<ChessGameResource>,
) {
    for (interaction, mut color, mut border_color, children, board_setup_settings) in
        &mut interaction_query
    {
        // let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                // **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
                // println!("{:?} clicked", board_setup_settings);
                next_screen_state.set(ScreenState::GameScreen);

                next_game_mode_state.set(board_setup_settings.game_mode.clone());

                board_resource.board = Board::new(board_setup_settings.main_player);

                if board_setup_settings.game_mode == GameMode::PlayerVsComputer
                    && board_setup_settings.main_player == PieceColor::Black
                {
                    next_turn_state.set(TurnState::ComputerTurn);
                } else {
                    next_turn_state.set(TurnState::Player1Turn);
                }
                break;
            }
            Interaction::Hovered => {
                // **text = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                // **text = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

#[derive(Component)]
struct HomeScreenMarker;

fn tear_down_home_screen(
    mut commands: Commands,
    home_screen_items: Query<Entity, With<HomeScreenMarker>>,
) {
    home_screen_items
        .iter()
        .for_each(|home_screen_item| commands.entity(home_screen_item).despawn());
}

fn setup_home_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            // justify_content: JustifyContent::SpaceAround,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        HomeScreenMarker,
        children![
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                children![
                    (
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            align_items: AlignItems::End,
                            justify_content: JustifyContent::SpaceAround,
                            ..default()
                        },
                        children![Text::new("Play Against Computer")],
                    ),
                    (
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            align_items: AlignItems::Start,
                            justify_content: JustifyContent::SpaceAround,
                            ..default()
                        },
                        children![
                            button(
                                &asset_server,
                                BoardSetupSettings {
                                    main_player: PieceColor::White,
                                    game_mode: GameMode::PlayerVsComputer,
                                },
                                PieceColor::White
                            ),
                            button(
                                &asset_server,
                                BoardSetupSettings {
                                    main_player: PieceColor::Black,
                                    game_mode: GameMode::PlayerVsComputer,
                                },
                                PieceColor::Black
                            )
                        ]
                    )
                ]
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                children![
                    (
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            align_items: AlignItems::End,
                            justify_content: JustifyContent::SpaceAround,
                            ..default()
                        },
                        children![Text::new("Play Against Human (Local Multiplayer)")],
                    ),
                    (
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            align_items: AlignItems::Start,
                            justify_content: JustifyContent::SpaceAround,
                            ..default()
                        },
                        children![
                            button(
                                &asset_server,
                                BoardSetupSettings {
                                    main_player: PieceColor::White,
                                    game_mode: GameMode::PlayerVsPlayer,
                                },
                                PieceColor::White
                            ),
                            button(
                                &asset_server,
                                BoardSetupSettings {
                                    main_player: PieceColor::Black,
                                    game_mode: GameMode::PlayerVsPlayer,
                                },
                                PieceColor::Black
                            )
                        ]
                    )
                ],
            )
        ],
    ));
}

fn button(
    asset_server: &AssetServer,
    board_setup_settings: BoardSetupSettings,
    color: PieceColor,
) -> impl Bundle + use<> {
    let height = 65.;
    (
        Button,
        Node {
            width: Val::Px(150.0),
            height: Val::Px(height),
            border: UiRect::all(Val::Px(5.0)),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            ..default()
        },
        board_setup_settings,
        BorderColor(Color::BLACK),
        BorderRadius::MAX,
        BackgroundColor(NORMAL_BUTTON),
        // Sprite {
        //     image: asset_server.load(format_piece_filename(
        //         PieceColor::White.file_string(),
        //         PiecePerson::King {moved : false}.file_string(),
        //     )),
        //     // custom_size: Some(Vec2::new(size_x, size_y)),
        //     ..default()
        // },
        children![(
            // Text::new("Button"),
            // TextColor(Color::srgb(0.9, 0.9, 0.9)),
            // TextShadow::default(),
            ImageNode::new(asset_server.load(format_piece_filename(
                color.file_string(),
                PiecePerson::King { moved: false }.file_string(),
            ))),
            Node {
                width: Val::Px(height),
                height: Val::Px(height),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                // margin: UiRect::all(Val::Px(20.0)),
                ..default()
            }
        )],
    )
}

fn setup_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut board_resource: NonSendMut<ChessGameResource>,
    window: Single<&mut Window>,
) {
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
}

fn re_draw_pieces(
    mut commands: Commands,
    mut board_resource: NonSendMut<ChessGameResource>,
    window: Single<&mut Window>,
    asset_server: Res<AssetServer>,
    gui_pieces: Query<Entity, With<PieceMarker>>,
) {
    gui_pieces
        .iter()
        .for_each(|gui_piece| commands.entity(gui_piece).despawn());

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
    mut next_computer_turn_state: ResMut<NextState<TurnState>>,
    turn_state: Res<State<TurnState>>,
    game_mode: Res<State<GameMode>>,
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
                    board_resource.board.apply_move(&promotion.piece_move);

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

                board_resource.board.apply_move(&possible_move.piece_move);

                moved = true;
                break;
            }
        }

        // remove all the highlights
        current_highlights
            .iter()
            .for_each(|current_highlight| commands.entity(current_highlight).despawn());

        if moved {
            // let game_state = board_resource.board.outcome();

            // remove all the pieces

            // redraw all the pieces
            re_draw_pieces(commands, board_resource, window, asset_server, gui_pieces);

            // for x in board_resource.board.generate_transposition() {
            //     println!("{:?}", x);
            // }

            match game_mode.get() {
                GameMode::PlayerVsComputer => {
                    next_computer_turn_state.set(TurnState::ComputerTurn);
                }
                GameMode::PlayerVsPlayer => {
                    let current_turn_state = turn_state.get();
                    match turn_state.get() {
                        TurnState::Player1Turn => {
                            next_computer_turn_state.set(TurnState::Player2Turn);
                        }
                        TurnState::Player2Turn => {
                            next_computer_turn_state.set(TurnState::Player1Turn);
                        }
                        TurnState::ComputerTurn => {
                            panic!(
                                "In wrong state for mouse movement {:?}, ",
                                current_turn_state
                            )
                        }
                    }
                }
            }

            return;
        }

        if let Some(moves) = board_resource
            .board
            .get_possible_moves(board_click_position)
        {
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
    mut next_computer_turn_state: ResMut<NextState<TurnState>>,
) {
    // let computer_move = board_resource.board.find_computer_move(board_resource.board.get_all_moves_for_turn());

    // let board = board_resource.board.clone();
    // println!("Probability of white winning: {}", board_resource.engine.infer_probability_of_white_winning_cached(&board));

    let board = board_resource.board.clone();

    let computer_move = board_resource
        .engine
        .next_best_move_natural_minimax_ab_no_eval_sort_id(&board, 5);

    // println!("computer_move: {:?}", computer_move);
    board_resource.board.apply_move(&computer_move);

    // // remove all the pieces
    // gui_pieces
    //     .iter()
    //     .for_each(|gui_piece| commands.entity(gui_piece).despawn());

    re_draw_pieces(commands, board_resource, window, asset_server, gui_pieces);

    // redraw all the pieces
    next_computer_turn_state.set(TurnState::Player1Turn);
}

fn check_if_game_over(
    board_resource: NonSendMut<ChessGameResource>,
    mut commands: Commands,
    mut next_screen_state: ResMut<NextState<ScreenState>>,
    // mut next_turn_state: ResMut<NextState<TurnState>>,
) {
    let game_state = board_resource.board.outcome();

    match game_state {
        Outcome::Playing => {
            return;
        }
        Outcome::Checkmate { winner } => {
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
        }
        Outcome::Draw => {
            commands.spawn((
                Text::new("DRAW"),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(12.0),
                    left: Val::Px(12.0),
                    ..default()
                },
            ));
        }
    }
    next_screen_state.set(ScreenState::GameTerminationScreen);
    // next_turn_state.set(TurnState::None);
}
