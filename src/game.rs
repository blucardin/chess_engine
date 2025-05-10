use bevy::input::common_conditions::*;
use bevy::prelude::*;
use std::cmp::PartialEq;
use std::ops;

const BOARD_TILE_DIM: isize = 8;
const DEFAULT_BOARD_TILE_SIZE: f32 = 100.0;
pub(crate) const DEFAULT_BOARD_HEIGHT: f32 = BOARD_TILE_DIM as f32 * DEFAULT_BOARD_TILE_SIZE;

const WHITE_TILE_COLOR: Color = Color::srgb_u8(254, 207, 159);

const BLACK_TILE_COLOR: Color = Color::srgb_u8(210, 140, 69);

const POSSIBLE_MOVE_HIGHLIGHT_COLOR: Color = Color::srgba_u8(32, 194, 29, 255 / 4);
const PROMOTION_BACKGROUND_COLOR: Color = Color::srgb_u8(45, 45, 45);
const PIECES_FOLDER: &str = "pieces-basic-png";

const ROOK_SEARCH_OFFSETS: [(isize, isize); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_SEARCH_OFFSETS: [(isize, isize); 4] = [(1, 1), (-1, 1), (-1, -1), (1, -1)];

const KING_SEARCH_OFFSETS: [(isize, isize); 8] = [
    (1, -1),
    (1, 0),
    (1, 1),
    (0, -1),
    (0, 1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];

const KNIGHT_SEARCH_OFFSETS: [(isize, isize); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

pub struct Game;

impl Plugin for Game {
    fn build(&self, app: &mut App) {
        app.insert_resource(Board::new(PieceColor::White));
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            mouse_button_input.run_if(input_just_pressed(MouseButton::Left)),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PieceColor {
    Black,
    White,
}

impl PieceColor {
    fn opposite(&self) -> Self {
        match self {
            PieceColor::Black => PieceColor::White,
            PieceColor::White => PieceColor::Black,
        }
    }

    fn file_string(&self) -> String {
        String::from(match self {
            PieceColor::Black => "black",
            PieceColor::White => "white",
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PiecePerson {
    Pawn { first_move: Option<i32> },
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

impl PiecePerson {
    fn new_pawn() -> Self {
        PiecePerson::Pawn { first_move: None }
    }

    fn file_string(&self) -> String {
        String::from(match self {
            PiecePerson::Pawn { first_move: _ } => "pawn",
            PiecePerson::Rook => "rook",
            PiecePerson::Knight => "knight",
            PiecePerson::Bishop => "bishop",
            PiecePerson::Queen => "queen",
            PiecePerson::King => "king",
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Piece {
    color: PieceColor,
    piece_person: PiecePerson,
    id: Option<Entity>,
}

fn format_piece_filename(color_file_string: String, name_file_string: String) -> String {
    format!(
        "{}/{}-{}.png",
        PIECES_FOLDER, color_file_string, name_file_string
    )
}

impl Piece {
    fn new(color: PieceColor, piece_person: PiecePerson) -> Self {
        Piece {
            color,
            piece_person,
            id: None,
        }
    }
    fn get_asset_path(&self) -> String {
        let name_file_string = self.piece_person.file_string();
        let color_file_string = self.color.file_string();
        format_piece_filename(color_file_string, name_file_string)
    }
}

#[derive(Resource, Clone)]
pub(crate) struct Board {
    player_1_color: PieceColor,
    turn: PieceColor,
    squares: [[Square; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize],
    move_number: i32,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
struct Coordinate {
    x: isize,
    y: isize,
}

impl ops::Add<(isize, isize)> for Coordinate {
    type Output = Coordinate;
    fn add(self, rhs: (isize, isize)) -> Self::Output {
        Self::Output {
            x: self.x + rhs.0,
            y: self.y + rhs.1,
        }
    }
}

#[derive(Debug)]
enum Side {
    Left,
    Right,
}

#[derive(Debug)]
enum Move {
    Regular {
        initial_position: Coordinate,
        final_position: Coordinate,
        move_type: MoveType,
    },
    Promote {
        initial_position: Coordinate,
        final_position: Coordinate,
        move_type: MoveType,
        piece_person: PiecePerson,
    },
    Castle {
        initial_position: Coordinate,
        side: Side,
    },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Square {
    Filled(Piece),
    Empty,
    Boundary,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum MoveType {
    Jump,
    Take,
}

enum GameState {
    Playing,
    Checkmate { winner: PieceColor },
    Draw,
}

const POSSIBLE_PAWN_PROMOTES: [PiecePerson; 4] = [
    PiecePerson::Queen,
    PiecePerson::Rook,
    PiecePerson::Bishop,
    PiecePerson::Knight,
];

// custom implementation for unusual values
impl Board {
    fn get_square(&self, position: &Coordinate) -> Square {
        let x = position.x;
        let y = position.y;
        if (x < 0 || y < 0)
            || (x >= self.squares.len() as isize || y >= self.squares[0].len() as isize)
        {
            Square::Boundary
        } else {
            self.squares[x as usize][y as usize]
        }
    }

    fn possible_jump(&self, position: &Coordinate) -> bool {
        self.get_square(position) == Square::Empty
    }

    fn possible_take(&self, position: &Coordinate) -> bool {
        if let Square::Filled(piece) = self.get_square(position) {
            piece.color != self.turn
        } else {
            false
        }
    }

    fn possible_move(&self, position: &Coordinate) -> Option<MoveType> {
        match self.get_square(position) {
            Square::Empty => Some(MoveType::Jump),
            Square::Filled(piece) => {
                if piece.color != self.turn {
                    Some(MoveType::Take)
                } else {
                    None
                }
            }
            Square::Boundary => None,
        }
    }

    fn cast_ray(&self, position: &Coordinate, offsets: &[(isize, isize)]) -> Vec<Move> {
        let mut output = Vec::new();
        for offset in offsets {
            let mut sight = *position + *offset;

            while let Some(move_type) = self.possible_move(&sight) {
                output.push(Move::Regular {
                    initial_position: *position,
                    final_position: sight,
                    move_type: move_type,
                });

                if move_type == MoveType::Take {
                    break;
                }

                sight = sight + *offset;
            }
        }
        output
    }

    fn check_squares(&self, position: &Coordinate, offsets: &[(isize, isize)]) -> Vec<Move> {
        let mut output = Vec::new();
        for offset in offsets {
            let sight = *position + *offset;
            if let Some(move_type) = self.possible_move(&sight) {
                output.push(Move::Regular {
                    initial_position: *position,
                    final_position: sight,
                    move_type: move_type,
                });
            }
        }
        output
    }

    fn pawn_going_up(&self) -> bool {
        self.turn == self.player_1_color
    }

    fn get_possible_moves(&self, initial_position: Coordinate) -> Option<Vec<Move>> {
        if let Square::Filled(piece) = self.get_square(&initial_position) {
            if piece.color != self.turn {
                return None;
            }

            let mut output = Vec::new();

            Some(self.filter_legal_moves(match piece.piece_person {
                PiecePerson::Pawn { first_move } => {
                    let going_up = self.pawn_going_up();
                    let direction: isize = if going_up { -1 } else { 1 };

                    // Check if the pawn is on the last row of its direction, these become 3 separate moves, Knight, Rook, and Queen

                    let mut promote = false;

                    if going_up {
                        if initial_position.y == 1 {
                            promote = true;
                        }
                    } else {
                        if initial_position.y == (BOARD_TILE_DIM - 2) {
                            promote = true;
                        }
                    }

                    let front = initial_position + (0, 1 * direction);
                    if self.possible_jump(&front) {
                        let move_type = MoveType::Jump;
                        if promote {
                            for piece_person in POSSIBLE_PAWN_PROMOTES {
                                output.push(Move::Promote {
                                    initial_position,
                                    final_position: front,
                                    move_type,
                                    piece_person,
                                });
                            }
                        } else {
                            output.push(Move::Regular {
                                initial_position,
                                final_position: front,
                                move_type,
                            });
                        }

                        if first_move.is_none() {
                            let front = initial_position + (0, 2 * direction);
                            if self.possible_jump(&front) {
                                output.push(Move::Regular {
                                    initial_position,
                                    final_position: front,
                                    move_type,
                                });
                            }
                        }
                    }

                    for i in [-1, 1] {
                        let front_lr = initial_position + (i, 1 * direction);
                        if self.possible_take(&front_lr) {
                            let move_type = MoveType::Take;
                            if promote {
                                for piece_person in POSSIBLE_PAWN_PROMOTES {
                                    output.push(Move::Promote {
                                        initial_position,
                                        final_position: front_lr,
                                        move_type,
                                        piece_person,
                                    });
                                }
                            } else {
                                output.push(Move::Regular {
                                    initial_position,
                                    final_position: front_lr,
                                    move_type,
                                });
                            }
                        }
                    }

                    // TODO: implement en passant

                    output
                }
                PiecePerson::Rook => self.cast_ray(&initial_position, &ROOK_SEARCH_OFFSETS),
                PiecePerson::Bishop => self.cast_ray(&initial_position, &BISHOP_SEARCH_OFFSETS),
                PiecePerson::Queen => self.cast_ray(
                    &initial_position,
                    &[ROOK_SEARCH_OFFSETS, BISHOP_SEARCH_OFFSETS].concat(),
                ),
                PiecePerson::King => self.check_squares(&initial_position, &KING_SEARCH_OFFSETS),
                PiecePerson::Knight => {
                    self.check_squares(&initial_position, &KNIGHT_SEARCH_OFFSETS)
                }
            }))
        } else {
            None
        }
    }

    fn filter_legal_moves(&self, moves: Vec<Move>) -> Vec<Move> {
        moves
            .into_iter()
            .filter(|piece_move| {
                let mut test_board = self.clone();
                test_board.apply_move(&piece_move);
                !test_board.check_check(self.turn)
            })
            .collect()
    }

    fn get_all_moves_for_turn(&self) -> Vec<Move> {
        let mut output = Vec::new();
        for (idx, row) in self.squares.iter().enumerate() {
            for (idy, square) in row.iter().enumerate() {
                if let Some(piece_moves) = self.get_possible_moves(Coordinate {
                    x: idx as isize,
                    y: idy as isize,
                }) {
                    output.extend(piece_moves);
                }
            }
        }
        output
    }

    fn locate_king(&self, search_color: PieceColor) -> Coordinate {
        for (idx, row) in self.squares.iter().enumerate() {
            for (idy, square) in row.iter().enumerate() {
                if let Square::Filled(Piece {
                    color,
                    id,
                    piece_person: PiecePerson::King,
                }) = square
                {
                    if *color == search_color {
                        return Coordinate {
                            x: idx as isize,
                            y: idy as isize,
                        };
                    }
                }
            }
        }
        panic!("NO KING ON BOARD")
    }

    fn check_check(&self, color: PieceColor) -> bool {
        let king_location = self.locate_king(color);

        for (piece_person, offsets) in [
            (PiecePerson::Rook, ROOK_SEARCH_OFFSETS),
            (PiecePerson::Bishop, BISHOP_SEARCH_OFFSETS),
        ] {
            'rays: for offset in offsets {
                let mut sight = king_location;

                'squares: loop {
                    sight = sight + offset;
                    match self.get_square(&sight) {
                        Square::Empty => {
                            continue 'squares;
                        }

                        Square::Filled(piece) => {
                            if piece.color == color {
                                continue 'rays;
                            }

                            if piece.piece_person == PiecePerson::Queen
                                || piece.piece_person == piece_person
                            {
                                return true;
                            }
                        }
                        Square::Boundary => {
                            continue 'rays;
                        }
                    }
                }
            }
        }

        for (piece_person, offsets) in [
            (PiecePerson::King, KING_SEARCH_OFFSETS),
            (PiecePerson::Knight, KNIGHT_SEARCH_OFFSETS),
        ] {
            for offset in offsets {
                if let Square::Filled(piece) = self.get_square(&(king_location + offset)) {
                    if piece.color != color && piece.piece_person == piece_person {
                        return true;
                    }
                }
            }
        }

        let going_up = color == self.player_1_color;
        let threatening_pawn_y_offset: isize = if going_up { -1 } else { 1 };
        let offsets = [
            (-1, threatening_pawn_y_offset),
            (1, threatening_pawn_y_offset),
        ];

        for offset in offsets {
            if let Square::Filled(piece) = self.get_square(&(king_location + offset)) {
                if let PiecePerson::Pawn { .. } = piece.piece_person {
                    if piece.color != color {
                        return true;
                    }
                }
            }
        }

        // TODO: check for en passant
        false
    }

    fn apply_move(&mut self, instruction: &Move) -> Vec<Piece> {

        let pieces_to_update = match instruction {
            Move::Regular {
                initial_position,
                final_position,
                move_type,
            } => {
                let replacement_piece: Piece =
                    match self.squares[initial_position.x as usize][initial_position.y as usize] {
                        Square::Filled(Piece {
                            color,
                            piece_person:
                                PiecePerson::Pawn {
                                    first_move: Option::None,
                                },
                            id,
                        }) => Piece {
                            color,
                            piece_person: PiecePerson::Pawn {
                                first_move: Some(self.move_number),
                            },
                            id,
                        },
                        Square::Filled(piece) => piece,
                        _ => {
                            panic!("Inital Piece not filled")
                        }
                    };

                self.replace_piece(initial_position, final_position, replacement_piece)
            }
            Move::Promote {
                initial_position,
                final_position,
                move_type,
                piece_person,
            } => {
                let old_pawn = self.squares[initial_position.x as usize][initial_position.y as usize];
                if let Square::Filled(piece) = old_pawn {
                    self.replace_piece(
                        initial_position,
                        final_position,
                        Piece {
                            piece_person: *piece_person, 
                            ..piece
                        },
                    )
                } else { 
                    panic!("Original Piece for promotion not filled")
                }
            },
            Move::Castle { .. } => {
                todo!("Castle")
            }
        };

        self.turn = self.turn.opposite();
        self.move_number += 1;

        pieces_to_update
    }

    fn replace_piece(
        &mut self,
        initial_position: &Coordinate,
        final_position: &Coordinate,
        promotion_piece: Piece,
    ) -> Vec<Piece> {
        let fx = final_position.x as usize;
        let fy = final_position.y as usize;
        let ix = initial_position.x as usize;
        let iy = initial_position.y as usize;

        let final_square = self.squares[fx][fy];

        self.squares[fx][fy] = Square::Filled(promotion_piece);

        self.squares[ix][iy] = Square::Empty;

        if let Square::Filled(piece) = final_square {
            vec![piece]
        } else {
            vec![]
        }
    }

    fn outcome(&self) -> (GameState, Vec<Move>) {
        let possible_moves = self.get_all_moves_for_turn();

        let game_state = if possible_moves.is_empty() {
            if self.check_check(self.turn) {
                GameState::Checkmate {
                    winner: self.turn.opposite(),
                }
            } else {
                GameState::Draw
            }
        } else {
            GameState::Playing
        };

        (game_state, possible_moves)
    }

    fn new_row(right_piece: Piece, left_piece: Piece) -> [Square; BOARD_TILE_DIM as usize] {
        [
            Square::Filled(right_piece),
            Square::Filled(Piece::new(right_piece.color, PiecePerson::new_pawn())),
            Square::Empty,
            Square::Empty,
            Square::Empty,
            Square::Empty,
            Square::Filled(Piece::new(left_piece.color, PiecePerson::new_pawn())),
            Square::Filled(left_piece),
        ]
    }

    fn new(player_1_color: PieceColor) -> Self {
        let player_2_color = player_1_color.opposite();

        let mut squares: [[Square; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize] =
            [[Square::Empty; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

        for (idx, person) in [PiecePerson::Rook, PiecePerson::Knight, PiecePerson::Bishop]
            .iter()
            .enumerate()
        {
            squares[idx] = Board::new_row(
                Piece::new(player_2_color, *person),
                Piece::new(player_1_color, *person),
            );
        }

        squares[3] = Board::new_row(
            Piece::new(player_2_color, PiecePerson::Queen),
            Piece::new(player_1_color, PiecePerson::Queen),
        );
        squares[4] = Board::new_row(
            Piece::new(player_2_color, PiecePerson::King),
            Piece::new(player_1_color, PiecePerson::King),
        );

        for (idx, person) in [PiecePerson::Rook, PiecePerson::Knight, PiecePerson::Bishop]
            .iter()
            .rev()
            .enumerate()
        {
            squares[idx + 5] = Board::new_row(
                Piece::new(PieceColor::Black, *person),
                Piece::new(PieceColor::White, *person),
            );
        }

        Board {
            player_1_color,
            turn: PieceColor::White,
            squares,
            move_number: 0,
        }
    }
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
    mut board: ResMut<Board>,
    window: Single<&mut Window>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);

    let width = window.width();
    let height = window.height();

    let size_x = width / BOARD_TILE_DIM as f32;
    let size_y = height / BOARD_TILE_DIM as f32;

    for (idx, row) in board.squares.iter_mut().enumerate() {
        for (idy, square) in row.iter_mut().enumerate() {
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

            if let Square::Filled(piece) = square {
                // load the piece sprite using commands and store the id of the piece in the board resource
                piece.id = Some(
                    commands
                        .spawn((
                            Sprite {
                                image: asset_server.load(piece.get_asset_path()),
                                custom_size: Some(Vec2::new(size_x, size_y)),
                                ..default()
                            },
                            transform,
                        ))
                        .id(),
                );
            }
        }
    }
}

#[derive(Component)]
struct PossibleMove {
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
    mut board: ResMut<Board>,
    window: Single<&mut Window>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    current_highlights: Query<Entity, With<Highlight>>,
    possible_moves: Query<&PossibleMove>,
    promotions: Query<&PromotionPicker>,
    promotion_pickers: Query<Entity, With<PromotionPicker>>,
    asset_server: Res<AssetServer>,
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
                    if let Move::Promote {
                        initial_position,
                        final_position,
                        move_type,
                        piece_person,
                    } = promotion.piece_move
                    {
                        if let Square::Filled(piece) = board.get_square(&initial_position) {
                            // don't despawn and respawn, just change the sprite and the transform

                            // despawn the og pawn
                            commands
                                .entity(piece.id.unwrap())
                                .remove::<Transform>()
                                .remove::<Sprite>()
                                .insert((
                                    Sprite {
                                        image: asset_server.load(format_piece_filename(
                                            board.turn.file_string(),
                                            piece_person.file_string(),
                                        )),
                                        custom_size: Some(Vec2::new(size_x, size_y)),
                                        ..default()
                                    },
                                    generate_transform_for_board_gui(
                                        width,
                                        height,
                                        size_x,
                                        size_y,
                                        final_position.x as usize,
                                        final_position.y as usize,
                                    ),
                                ));

                            let pieces_to_despawn = board.apply_move(&promotion.piece_move);

                            for piece in pieces_to_despawn.iter() {
                                commands.entity(piece.id.unwrap()).despawn();
                            }

                            // despawn all the move picker elements
                            for promotion_picker in promotion_pickers.iter() {
                                commands.entity(promotion_picker).despawn();
                            }

                            moved = true;
                            break;
                        }
                    }
                }
            }
            
            if !moved {
                return; 
            }
        }

        for possible_move in possible_moves.iter() {
            match possible_move.piece_move {
                Move::Regular {
                    initial_position,
                    final_position,
                    move_type,
                } => {
                    if board_click_position == final_position {
                        if let Square::Filled(piece) = board.get_square(&initial_position) {
                            let pieces_to_despawn = board.apply_move(&possible_move.piece_move);

                            commands
                                .entity(piece.id.unwrap())
                                .remove::<Transform>()
                                .insert(generate_transform_for_board_gui(
                                    width,
                                    height,
                                    size_x,
                                    size_y,
                                    final_position.x as usize,
                                    final_position.y as usize,
                                ));

                            for piece in pieces_to_despawn.iter() {
                                commands.entity(piece.id.unwrap()).despawn();
                            }

                            moved = true;
                            break;
                        }
                    }
                }
                Move::Promote {
                    initial_position,
                    final_position,
                    move_type,
                    piece_person,
                } => {
                    // remove all the highlights
                    current_highlights
                        .iter()
                        .for_each(|current_highlight| commands.entity(current_highlight).despawn());

                    let going_up = board.pawn_going_up();
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
                                    board.turn.file_string(),
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
                Move::Castle { .. } => {}
            }
        }

        // remove all the highlights
        current_highlights
            .iter()
            .for_each(|current_highlight| commands.entity(current_highlight).despawn());

        if moved {
            let (game_state, moves) = board.outcome();

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
                }
                GameState::Draw => {}
            }
            return;
        }

        if let Some(moves) = board.get_possible_moves(board_click_position) {
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
                match piece_move {
                    Move::Regular {
                        initial_position,
                        final_position,
                        move_type,
                    } => {
                        commands.spawn((
                            Highlight,
                            PossibleMove { piece_move },
                            Mesh2d(meshes.add(Rectangle::new(size_x, size_y))),
                            MeshMaterial2d(materials.add(POSSIBLE_MOVE_HIGHLIGHT_COLOR)),
                            Transform::from_xyz(
                                -(width / 2.) + (final_position.x as f32 * size_x) + (size_x / 2.),
                                (height / 2.) - (final_position.y as f32 * size_y) - (size_y / 2.),
                                0.0,
                            ),
                        ));
                    }
                    Move::Promote {
                        initial_position,
                        final_position,
                        move_type,
                        piece_person,
                    } => {
                        if !promotion_coordinates.contains(&final_position) {
                            promotion_coordinates.push(final_position);
                            commands.spawn((
                                Highlight,
                                PossibleMove { piece_move },
                                Mesh2d(meshes.add(Rectangle::new(size_x, size_y))),
                                MeshMaterial2d(materials.add(POSSIBLE_MOVE_HIGHLIGHT_COLOR)),
                                generate_transform_for_board_gui(
                                    width,
                                    height,
                                    size_x,
                                    size_y,
                                    final_position.x as usize,
                                    final_position.y as usize,
                                ),
                            ));
                        }
                    }
                    Move::Castle { .. } => {}
                }
            }
        }
    } else {
        println!("Cursor is not in the game window.");
    }
}
