use bevy::prelude::*;
use bevy::sprite::{Wireframe2dConfig, Wireframe2dPlugin};
use std::ops;

const BOARD_TILE_DIM: i32 = 8;
const DEFAULT_BOARD_TILE_SIZE: f32 = 100.0;
const DEFAULT_BOARD_HEIGHT: f32 = BOARD_TILE_DIM as f32 * DEFAULT_BOARD_TILE_SIZE;

const WHITE_TILE_COLOR: Color = Color::srgb_u8(254, 207, 159);

const BLACK_TILE_COLOR: Color = Color::srgb_u8(210, 140, 69);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PieceColor {
    Black,
    White,
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
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Piece {
    color: PieceColor,
    piece_person: PiecePerson,
    id: Option<Entity>,
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
        let name: &str = match self.piece_person {
            PiecePerson::Pawn { first_move: _ } => "pawn",
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

#[derive(Resource, Clone)]
struct Board {
    player_1_color: PieceColor,
    turn: PieceColor,
    squares: [[Square; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize],
    move_number: i32,
}

#[derive(Clone, Copy)]
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

enum Side {
    Left,
    Right,
}

enum Move {
    Regular {
        position: Coordinate,
        move_type: MoveType,
    },
    Promote {
        position: Coordinate,
        move_type: MoveType,
        piece: Piece,
    },
    Castle {
        side: Side,
    },
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Square {
    Filled(Piece),
    Empty,
    Boundary,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum MoveType {
    Jump,
    Take,
}

// custom implementation for unusual values
impl Board {
    fn get_square(&self, position: &Coordinate) -> Square {
        let x = position.x;
        let y = position.y;
        if (x < 0 || y < 0)
            || (x >= self.squares.len() as isize - 1 || y >= self.squares[0].len() as isize - 1)
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
                    position: sight,
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
                    position: sight,
                    move_type: move_type,
                });
            }
        }
        output
    }

    fn get_possible_moves(&self, position: Coordinate) -> Option<Vec<Move>> {
        if let Square::Filled(piece) = self.get_square(&position) {
            if piece.color != self.turn {
                return None;
            }

            let mut output = Vec::new();

            Some(self.filter_legal_moves(
                position,
                match piece.piece_person {
                    PiecePerson::Pawn { first_move } => {
                        let going_up = self.turn == self.player_1_color;
                        let direction: isize = if going_up { -1 } else { 1 };

                        // TODO: implement pawn promotion
                        // Check if the pawn is on the last row of it's direction, these become 3 separate moves, Knight, Rook, and Queen
                        if going_up {}

                        let front = position + (0, 1 * direction);
                        if self.possible_jump(&front) {
                            output.push(Move::Regular {
                                position: front,
                                move_type: MoveType::Jump,
                            });
                        }

                        for i in [-1, 1] {
                            let front_lr = position + (i, 1 * direction);
                            if self.possible_take(&front_lr) {
                                output.push(Move::Regular {
                                    position: front_lr,
                                    move_type: MoveType::Take,
                                });
                            }
                        }

                        if first_move.is_none() {
                            let front = position + (0, 2 * direction);
                            if self.possible_jump(&front) {
                                output.push(Move::Regular {
                                    position: front,
                                    move_type: MoveType::Jump,
                                });
                            }
                        }

                        output
                    }
                    PiecePerson::Rook => self.cast_ray(&position, &ROOK_SEARCH_OFFSETS),
                    PiecePerson::Bishop => self.cast_ray(&position, &BISHOP_SEARCH_OFFSETS),
                    PiecePerson::Queen => self.cast_ray(
                        &position,
                        &[ROOK_SEARCH_OFFSETS, BISHOP_SEARCH_OFFSETS].concat(),
                    ),
                    PiecePerson::King => self.check_squares(&position, &KING_SEARCH_OFFSETS),
                    PiecePerson::Knight => self.check_squares(&position, &KNIGHT_SEARCH_OFFSETS),
                },
            ))
        } else {
            None
        }
    }

    fn filter_legal_moves(&self, position: Coordinate, moves: Vec<Move>) -> Vec<Move> {
        // let mut output = Vec::new();
        // for piece_move in moves {
        //     let mut test_board = self.clone();
        //     test_board.apply_move(position, &piece_move);
        //     if !test_board.check_check() {
        //         output.push(piece_move);
        //     }
        // }
        // output

        moves
            .into_iter()
            .filter(|piece_move| {
                let mut test_board = self.clone();
                test_board.apply_move(position, &piece_move);
                !test_board.check_check()
            })
            .collect()
    }

    fn get_all_moves(&self) -> Vec<Move> {
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

    fn locate_king(&self) -> Coordinate {
        for (idx, row) in self.squares.iter().enumerate() {
            for (idy, square) in row.iter().enumerate() {
                if let Square::Filled(Piece {
                    color,
                    id,
                    piece_person: PiecePerson::King,
                }) = square
                {
                    return Coordinate {
                        x: idx as isize,
                        y: idy as isize,
                    };
                }
            }
        }
        panic!("NO KING ON BOARD")
    }

    fn check_check(&self) -> bool {
        let king_location = self.locate_king();

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
                            if piece.color == self.turn {
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
                    if piece.color != self.turn && piece.piece_person == piece_person {
                        return true;
                    }
                }
            }
        }

        let going_up = self.turn == self.player_1_color;
        let threatening_pawn_y_offset: isize = if going_up { -1 } else { 1 };
        let offsets = [
            (-1, threatening_pawn_y_offset),
            (0, threatening_pawn_y_offset),
        ];

        for offset in offsets {
            if let Square::Filled(piece) = self.get_square(&(king_location + offset)) {
                if let PiecePerson::Pawn { .. } = piece.piece_person {
                    if piece.color != self.turn {
                        return true;
                    }
                }
            }
        }

        // TODO: check for en passant
        false
    }

    fn apply_move(&mut self, initial_position: Coordinate, instuction: &Move) -> Option<Piece> {
        match instuction {
            Move::Regular {
                position: final_position,
                move_type,
            } => {
                let final_square =
                    self.squares[final_position.x as usize][final_position.y as usize];
                self.squares[final_position.x as usize][final_position.y as usize] =
                    self.squares[initial_position.x as usize][initial_position.y as usize];

                if let Square::Filled(piece) = final_square {
                    Some(piece)
                } else {
                    None
                }
            }
            Move::Promote { .. } => {
                todo!("Promote move")
            }
            Move::Castle { .. } => {
                todo!("Castle")
            }
        }
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
        let player_2_color = match player_1_color {
            PieceColor::Black => PieceColor::White,
            PieceColor::White => PieceColor::Black,
        };

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
            Piece::new(player_2_color, PiecePerson::King),
            Piece::new(player_1_color, PiecePerson::Queen),
        );
        squares[4] = Board::new_row(
            Piece::new(player_2_color, PiecePerson::Queen),
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
    .insert_resource(Board::new(PieceColor::White))
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

            let transform = Transform::from_xyz(
                // Distribute shapes from -X_EXTENT/2 to +X_EXTENT/2.
                -(width / 2.) + (idx as f32 * size_x) + (size_x / 2.),
                (height / 2.) - (idy as f32 * size_y) - (size_y / 2.),
                0.0,
            );

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
