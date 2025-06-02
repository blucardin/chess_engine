use bevy::prelude::{Color, Resource};
use rand::seq::IndexedRandom;
use std::cmp::PartialEq;
use std::ops;

extern crate either;

use either::Either;
use std::iter;
// 0.9.0

pub const BOARD_TILE_DIM: isize = 8;
const DEFAULT_BOARD_TILE_SIZE: f32 = 100.0;
pub const DEFAULT_BOARD_HEIGHT: f32 = BOARD_TILE_DIM as f32 * DEFAULT_BOARD_TILE_SIZE;

pub const WHITE_TILE_COLOR: Color = Color::srgb_u8(254, 207, 159);

pub const BLACK_TILE_COLOR: Color = Color::srgb_u8(210, 140, 69);

pub const POSSIBLE_MOVE_HIGHLIGHT_COLOR: Color = Color::srgba_u8(32, 194, 29, 255 / 4);
pub const PROMOTION_BACKGROUND_COLOR: Color = Color::srgb_u8(45, 45, 45);
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
pub enum PieceColor {
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

    pub fn file_string(&self) -> String {
        String::from(match self {
            PieceColor::Black => "black",
            PieceColor::White => "white",
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PiecePerson {
    Pawn { first_move: Option<i32> },
    Rook { moved: bool },
    Knight,
    Bishop,
    Queen,
    King { moved: bool },
}

impl PiecePerson {
    fn new_pawn() -> Self {
        PiecePerson::Pawn { first_move: None }
    }

    pub fn file_string(&self) -> String {
        String::from(match self {
            PiecePerson::Pawn { first_move: _ } => "pawn",
            PiecePerson::Rook { .. } => "rook",
            PiecePerson::Knight => "knight",
            PiecePerson::Bishop => "bishop",
            PiecePerson::Queen => "queen",
            PiecePerson::King { .. } => "king",
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Piece {
    color: PieceColor,
    piece_person: PiecePerson,
}

pub fn format_piece_filename(color_file_string: String, name_file_string: String) -> String {
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
        }
    }
    pub fn get_asset_path(&self) -> String {
        let name_file_string = self.piece_person.file_string();
        let color_file_string = self.color.file_string();
        format_piece_filename(color_file_string, name_file_string)
    }
}

#[derive(Resource, Clone)]
pub struct Board {
    player_1_color: PieceColor,
    pub turn: PieceColor,
    pub squares: [[Square; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize],
    move_number: i32,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Coordinate {
    pub x: isize,
    pub y: isize,
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

#[derive(Debug, Clone)]
pub enum Side {
    QueenSide,
    KingsSide,
}

#[derive(Debug, Clone)]
pub enum Move {
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
        side: Side,
    },
    EnPassant {
        initial_position: Coordinate,
        final_position: Coordinate,
    },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Square {
    Filled(Piece),
    Empty,
    Boundary,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MoveType {
    Jump,
    Take,
}

pub enum GameState {
    Playing,
    Checkmate { winner: PieceColor },
    Draw,
}

pub const POSSIBLE_PAWN_PROMOTES: [PiecePerson; 4] = [
    PiecePerson::Queen,
    PiecePerson::Rook { moved: true },
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

    pub fn pawn_going_up(&self) -> bool {
        self.turn == self.player_1_color
    }

    pub fn get_possible_moves(&self, initial_position: Coordinate) -> Option<Vec<Move>> {
        if let Square::Filled(piece) = self.get_square(&initial_position) {
            if piece.color != self.turn {
                // look into making this a part of the if-let statement above
                return None;
            }

            // println!("Generating possible moves for {:?}", piece);

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

                    let row_of_passant = if going_up { 3 } else { BOARD_TILE_DIM - 4 };

                    // check if we are on the rank of en passant
                    if initial_position.y == row_of_passant {
                        // check if the square beside you is filled with a pawn that just moved, if so add a new en passant take move to capture it

                        for x_offset in [1, -1] {
                            if let Square::Filled(Piece {
                                color,
                                piece_person: PiecePerson::Pawn { first_move },
                            }) = self.get_square(&(initial_position + (x_offset, 0)))
                            {
                                if color != self.turn {
                                    if let Some(first_move_number) = first_move {
                                        if first_move_number == self.move_number - 1 {
                                            output.push(Move::EnPassant {
                                                initial_position,
                                                final_position: initial_position
                                                    + (x_offset, direction),
                                            });
                                            // println!("PASSANT");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    output
                }
                PiecePerson::Rook { .. } => self.cast_ray(&initial_position, &ROOK_SEARCH_OFFSETS),
                PiecePerson::Bishop => self.cast_ray(&initial_position, &BISHOP_SEARCH_OFFSETS),
                PiecePerson::Queen => self.cast_ray(
                    &initial_position,
                    &[ROOK_SEARCH_OFFSETS, BISHOP_SEARCH_OFFSETS].concat(),
                ),
                PiecePerson::King { moved: king_moved } => {
                    output = self.check_squares(&initial_position, &KING_SEARCH_OFFSETS);

                    // determine if we are on the white or black side of the board
                    let row_to_check: isize = if self.pawn_going_up() {
                        BOARD_TILE_DIM - 1
                    } else {
                        0
                    };

                    // check that the king hasn't moved
                    if king_moved == false
                        && !self.check_check(
                            self.turn,
                            Coordinate {
                                x: 4,
                                y: row_to_check,
                            },
                        )
                    {
                        // check king side
                        // check that the rook hasn't moved
                        if let Square::Filled(Piece {
                            color: _color,
                            piece_person: PiecePerson::Rook { moved: false },
                        }) = self.get_square(&Coordinate {
                            x: BOARD_TILE_DIM - 1,
                            y: row_to_check,
                        }) {
                            let mut possible_castle = true;

                            for x in [5, 6] {
                                let intermediate = Coordinate { x, y: row_to_check };

                                // check that the intermediate squares are vacant,
                                // check that the intermediate squares are not under attack

                                if self.squares[intermediate.x as usize][intermediate.y as usize]
                                    != Square::Empty
                                    || self.check_check(self.turn, intermediate)
                                {
                                    possible_castle = false;
                                    break;
                                }
                            }
                            // add the castle move to output
                            if possible_castle {
                                output.push(Move::Castle {
                                    side: Side::KingsSide,
                                });
                            }
                        }

                        // check Queen side
                        if let Square::Filled(Piece {
                            color: _color,
                            piece_person: PiecePerson::Rook { moved: false },
                        }) = self.get_square(&Coordinate {
                            x: 0,
                            y: row_to_check,
                        }) {
                            let mut possible_castle = true;

                            for x in [2, 3] {
                                let intermediate = Coordinate { x, y: row_to_check };

                                // check that the intermediate squares are vacant,
                                // check that the intermediate squares are not under attack

                                if self.squares[intermediate.x as usize][intermediate.y as usize]
                                    != Square::Empty
                                    || self.check_check(self.turn, intermediate)
                                {
                                    possible_castle = false;
                                    break;
                                }
                            }

                            if self.squares[1][row_to_check as usize] != Square::Empty {
                                possible_castle = false;
                            }

                            // add the castle move to output
                            if possible_castle {
                                output.push(Move::Castle {
                                    side: Side::QueenSide,
                                });
                            }
                        }
                    }

                    output
                }
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
                // println!("{:?}", piece_move);
                test_board.apply_move(&piece_move);
                // todo: Don't go through the clone process with a castle because, we already check that castling won't produce check
                !test_board.check_check(self.turn, test_board.locate_king(self.turn))
                // todo: Replace self.turn with test-board.turn.opposite() as it makes more sense
            })
            .collect()
    }

    pub fn get_all_moves_for_turn(&self) -> Vec<Move> {
        let mut output = Vec::new();
        // todo: Make this faster by not enumerating over everything, just looping
        for (idx, row) in self.squares.iter().enumerate() {
            for (idy, _) in row.iter().enumerate() {
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
        // println!("Locating king");
        // println!("Second column of board: {:?}", self.squares[1]);
        // println!("Fifth column of board: {:?}", self.squares[4]);
        for (idx, row) in self.squares.iter().enumerate() {
            for (idy, square) in row.iter().enumerate() {
                if let Square::Filled(Piece {
                    color,
                    piece_person: PiecePerson::King { .. },
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

    fn check_check(&self, color: PieceColor, king_location: Coordinate) -> bool {
        // println!(
        //     "Checking for any checks. King Location: {:?}",
        //     king_location
        // );

        for (piece_persons, offsets) in [
            (
                &[
                    PiecePerson::Rook { moved: true },
                    PiecePerson::Rook { moved: false },
                ][..],
                ROOK_SEARCH_OFFSETS,
            ),
            (&[PiecePerson::Bishop][..], BISHOP_SEARCH_OFFSETS),
        ] {
            // println!("Checking the offsets for {:?}", piece_persons);

            for offset in offsets {
                let mut sight = king_location;
                'squares: loop {
                    sight = sight + offset;
                    match self.get_square(&sight) {
                        Square::Empty => {
                            // println!("Skipping empty square {:?}", sight);
                        }

                        Square::Filled(piece) => {
                            if piece.color != color {
                                // println!("Checking for queen check");
                                if piece.piece_person == PiecePerson::Queen {
                                    // println!("Check found: {:?}", PiecePerson::Queen);
                                    return true;
                                }

                                for piece_person in piece_persons {
                                    if piece.piece_person == *piece_person {
                                        // println!("Check found: {:?}", *piece_person);
                                        return true;
                                    }
                                }
                            }

                            break 'squares;
                        }

                        Square::Boundary => {
                            break 'squares;
                        }
                    }
                }
            }
        }

        for (piece_persons, offsets) in [
            (
                &[
                    PiecePerson::King { moved: true },
                    PiecePerson::King { moved: false },
                ][..],
                KING_SEARCH_OFFSETS,
            ),
            (&[PiecePerson::Knight][..], KNIGHT_SEARCH_OFFSETS),
        ] {
            for offset in offsets {
                if let Square::Filled(piece) = self.get_square(&(king_location + offset)) {
                    if piece.color != color {
                        for piece_person in piece_persons {
                            if piece.piece_person == *piece_person {
                                // println!("Check found: {:?}", *piece_person);
                                return true;
                            }
                        }
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
                        // println!("Check found: {:?}", piece.piece_person);
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn apply_move(&mut self, instruction: &Move) {
        let pieces_to_update = match instruction {
            Move::Regular {
                initial_position,
                final_position,
                ..
            } => {
                let replacement_piece: Piece =
                    match self.squares[initial_position.x as usize][initial_position.y as usize] {
                        Square::Filled(Piece {
                            color,
                            piece_person:
                                PiecePerson::Pawn {
                                    first_move: Option::None,
                                },
                        }) => Piece {
                            color,
                            piece_person: PiecePerson::Pawn {
                                first_move: Some(self.move_number),
                            },
                        },

                        Square::Filled(Piece {
                            color,
                            piece_person: PiecePerson::King { moved: false },
                        }) => Piece {
                            color,
                            piece_person: PiecePerson::King { moved: true },
                        },

                        Square::Filled(Piece {
                            color,
                            piece_person: PiecePerson::Rook { moved: false },
                        }) => Piece {
                            color,
                            piece_person: PiecePerson::Rook { moved: true },
                        },

                        Square::Filled(piece) => piece,
                        _ => {
                            panic!("Initial Piece not filled, or border encountered")
                        }
                    };

                self.replace_piece(initial_position, final_position, replacement_piece)
            }
            Move::Promote {
                initial_position,
                final_position,
                piece_person,
                ..
            } => {
                let old_pawn =
                    self.squares[initial_position.x as usize][initial_position.y as usize];
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
            }
            Move::Castle { side } => {
                let row_to_act: isize = if self.pawn_going_up() {
                    BOARD_TILE_DIM - 1
                } else {
                    0
                };
                let kings_position = Coordinate {
                    x: 4,
                    y: row_to_act,
                };

                let new_king_piece =
                    match self.squares[kings_position.x as usize][kings_position.y as usize] {
                        Square::Filled(Piece {
                            color,
                            piece_person: PiecePerson::King { moved: false },
                        }) => Piece {
                            color,
                            piece_person: PiecePerson::King { moved: true },
                        },
                        _ => {
                            panic!("No king where king expected")
                        }
                    };

                match side {
                    Side::KingsSide => {
                        self.replace_piece(
                            &kings_position,
                            &(kings_position + (2, 0)),
                            new_king_piece,
                        );
                        // same thing for the rook

                        let rooks_position = Coordinate {
                            x: BOARD_TILE_DIM - 1,
                            y: row_to_act,
                        };

                        let new_rook_piece = match self.squares[rooks_position.x as usize]
                            [rooks_position.y as usize]
                        {
                            Square::Filled(Piece {
                                color,
                                piece_person: PiecePerson::Rook { moved: false },
                            }) => Piece {
                                color,
                                piece_person: PiecePerson::Rook { moved: true },
                            },
                            _ => {
                                panic!("No rook where rook expected")
                            }
                        };
                        self.replace_piece(
                            &rooks_position,
                            &(rooks_position + (-2, 0)),
                            new_rook_piece,
                        );
                    }
                    Side::QueenSide => {
                        self.replace_piece(
                            &kings_position,
                            &(kings_position + (-2, 0)),
                            new_king_piece,
                        );
                        // same thing for the rook

                        let rooks_position = Coordinate {
                            x: 0,
                            y: row_to_act,
                        };

                        let new_rook_piece = match self.squares[rooks_position.x as usize]
                            [rooks_position.y as usize]
                        {
                            Square::Filled(Piece {
                                color,
                                piece_person: PiecePerson::Rook { moved: false },
                            }) => Piece {
                                color,
                                piece_person: PiecePerson::Rook { moved: true },
                            },
                            _ => {
                                panic!("No rook where rook expected")
                            }
                        };
                        self.replace_piece(
                            &rooks_position,
                            &(rooks_position + (3, 0)),
                            new_rook_piece,
                        );
                    }
                }
            }
            Move::EnPassant {
                initial_position,
                final_position,
            } => {
                if let Square::Filled(piece) =
                    self.squares[initial_position.x as usize][initial_position.y as usize]
                {
                    self.replace_piece(&initial_position, &final_position, piece);
                }

                self.squares[final_position.x as usize][initial_position.y as usize] =
                    Square::Empty;
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
    ) {
        let fx = final_position.x as usize;
        let fy = final_position.y as usize;
        let ix = initial_position.x as usize;
        let iy = initial_position.y as usize;

        self.squares[fx][fy] = Square::Filled(promotion_piece);

        self.squares[ix][iy] = Square::Empty;
    }

    pub fn find_computer_move(&self, all_possible_moves: Vec<Move>) -> Move {
        // get all the possible moves
        // let all_possible_moves = self.get_all_moves_for_turn();

        // todo: implement minimax searching

        // pick a random one
        all_possible_moves.choose(&mut rand::rng()).unwrap().clone()
    }

    pub fn outcome(&self) -> GameState {
        let possible_moves = self.get_all_moves_for_turn();

        let game_state = if possible_moves.is_empty() {
            if self.check_check(self.turn, self.locate_king(self.turn)) {
                GameState::Checkmate {
                    winner: self.turn.opposite(),
                }
            } else {
                GameState::Draw
            }
        } else {
            GameState::Playing
        };

        game_state
    }

    pub fn generate_transposition(&self) -> [[[bool; 10]; 8]; 8] {
        let mut output: [[[bool; 10]; 8]; 8] = [[[false; 10]; 8]; 8];

        // if the main player is black, reverse both enumerations
        let black_bottom = self.player_1_color == PieceColor::Black;

        let iterator = if !black_bottom {
            Either::Left(self.squares.iter().enumerate())
        } else {
            Either::Right(self.squares.iter().rev().enumerate())
        };

        for (idx, row) in iterator {

            let iterator2 = if !black_bottom {
                Either::Left(row.iter().enumerate())
            } else {
                Either::Right(row.iter().rev().enumerate())
            };

            // so white is always at the bottom

            for (idy, square) in iterator2 {
                match square {
                    Square::Filled(piece) => {
                        let filled = true;
                        let color = piece.color == PieceColor::White;
                        let mut moved_one_hot = false;
                        let mut en_passant = false;

                        let piece_person_one_hot: usize = match piece.piece_person {
                            PiecePerson::Pawn { first_move } => {

                                if first_move == Some(self.move_number - 1) {
                                    // if it is on the fourth rank of its color and there is a pawn of opposite color next to it, enable en passant
                                    let real_coordinates = if !black_bottom { Coordinate { x: idx as isize, y: idy as isize } } else { Coordinate { x: (BOARD_TILE_DIM - 1) - (idx as isize), y: (BOARD_TILE_DIM - 1) - (idy as isize) } };

                                    let row_of_passant = if piece.color == self.player_1_color { // if we are on the bottom
                                        BOARD_TILE_DIM - 4
                                    } else {
                                        3
                                    };
                                    
                                    println!("Row of Passant {:?}", row_of_passant);
                                    println!("Real coordinates of pawn {:?}", real_coordinates);

                                    if real_coordinates.y == row_of_passant {

                                        let opposite_color = piece.color.opposite();

                                        for offset in [(1, 0), (-1, 0)] {

                                            println!("Left/Right square {:?}", self.get_square(&(real_coordinates + offset)));
                                            
                                            if let Square::Filled(Piece { color, piece_person: PiecePerson::Pawn { first_move } }) = self.get_square(&(real_coordinates + offset)) {
                                                if color == opposite_color {
                                                    println!("En Passant set as true"); 
                                                    en_passant = true;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                                0
                            }
                            PiecePerson::Rook { moved } => {
                                moved_one_hot = moved;
                                1
                            }
                            PiecePerson::Knight => 2,
                            PiecePerson::Bishop => 3,
                            PiecePerson::Queen => 4,
                            PiecePerson::King { moved } => {
                                moved_one_hot = moved;
                                5
                            }
                        };

                        output[idx][idy] = [
                            false,
                            false,
                            false,
                            false,
                            false,
                            false,
                            filled,
                            color,
                            !moved_one_hot, // invert moved one hot so that kings only have an extra true value when they are not moved
                            en_passant,
                        ];
                        output[idx][idy][piece_person_one_hot] = true;
                    }
                    Square::Empty | Square::Boundary => {}
                }
            }
        }
        output
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

    pub fn new(player_1_color: PieceColor) -> Self {
        let player_2_color = player_1_color.opposite();

        let mut squares: [[Square; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize] =
            [[Square::Empty; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

        for (idx, person) in [
            PiecePerson::Rook { moved: false },
            PiecePerson::Knight,
            PiecePerson::Bishop,
        ]
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
            Piece::new(player_2_color, PiecePerson::King { moved: false }),
            Piece::new(player_1_color, PiecePerson::King { moved: false }),
        );

        for (idx, person) in [
            PiecePerson::Rook { moved: false },
            PiecePerson::Knight,
            PiecePerson::Bishop,
        ]
        .iter()
        .rev()
        .enumerate()
        {
            squares[idx + 5] = Board::new_row(
                Piece::new(player_2_color, *person),
                Piece::new(player_1_color, *person),
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
