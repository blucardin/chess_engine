pub mod move_serialization;

use bincode::{Decode, Encode};
// use rand::seq::IndexedRandom;
use std::cmp::PartialEq;
use std::{fmt, ops};

extern crate either;
use either::Either;

extern crate approx;

use pgn_reader::{CastlingSide, File, Rank, Role, San, SanPlus};
// 0.9.0

pub const BOARD_TILE_DIM: isize = 8;
const DEFAULT_BOARD_TILE_SIZE: f32 = 100.0;
pub const DEFAULT_BOARD_HEIGHT: f32 = BOARD_TILE_DIM as f32 * DEFAULT_BOARD_TILE_SIZE;
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

const BOARD_WEIGHTS: [f32; 8] = [0., 0.3, 0.6, 0.9, 0.9, 0.6, 0.3, 0.];

pub type Transposition = [[[bool; 10]; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

pub type SmallTransposition = [[[bool; 12]; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

pub type BoardSquares = [[Square; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

// pub type PieceWeights = [f32 ; 5];

pub struct PieceWeights {
    pub pawn: f32,
    pub knight: f32,
    pub bishop: f32,
    pub rook: f32,
    pub queen: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceColor {
    Black,
    White,
}

impl PieceColor {
    pub fn opposite(&self) -> Self {
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

    pub fn convert_signed(&self, value: f32) -> f32 {
        match self {
            PieceColor::Black => -value,
            PieceColor::White => value,
        }
    }
}

#[derive(Encode, Decode, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[repr(i8)]
pub enum PiecePerson {
    Pawn { first_move: Option<i32> } = 1,
    Rook { moved: bool } = 3,
    Knight = 2,
    Bishop = 4,
    Queen = 5,
    King { moved: bool } = 6,
}

impl PiecePerson {
    fn new_pawn() -> Self {
        PiecePerson::Pawn { first_move: None }
    }

    fn compare_with_role(&self, role: Role) -> bool {
        match role {
            Role::Pawn => {
                matches!(*self, PiecePerson::Pawn { .. })
            }
            Role::Knight => *self == PiecePerson::Knight,
            Role::Bishop => *self == PiecePerson::Bishop,
            Role::Rook => {
                matches!(*self, PiecePerson::Rook { .. })
            }
            Role::Queen => *self == PiecePerson::Queen,
            Role::King => {
                matches!(*self, PiecePerson::King { .. })
            }
        }
    }

    fn from_role(role: Role) -> Self {
        match role {
            Role::Pawn => {
                panic!("Cannot convert role pawn to PiecePerson because of first move ambiguity")
            }
            Role::Knight => PiecePerson::Knight,
            Role::Bishop => PiecePerson::Bishop,
            Role::Rook => PiecePerson::Rook { moved: true },
            Role::Queen => PiecePerson::Queen,
            Role::King => PiecePerson::King { moved: true },
        }
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

    fn get_index(&self) -> usize {
        match self {
            PiecePerson::Pawn { .. } => 0,
            PiecePerson::Rook { .. } => 1,
            PiecePerson::Knight => 2,
            PiecePerson::Bishop => 3,
            PiecePerson::Queen => 4,
            PiecePerson::King { .. } => 5,
        }
    }

    fn value(&self, weights: &PieceWeights) -> f32 {
        match self {
            PiecePerson::Pawn { .. } => weights.pawn,
            PiecePerson::Rook { .. } => weights.rook,
            PiecePerson::Knight => weights.knight,
            PiecePerson::Bishop => weights.bishop,
            PiecePerson::Queen => weights.queen,
            PiecePerson::King { .. } => {
                panic!("No value for king.")
            }
        }
    }

    // [1., 4., 2., 3. 5.]
    //     match self {
    //         PiecePerson::Pawn { .. } => 1.,
    //         PiecePerson::Rook { .. } => 4.,
    //         PiecePerson::Knight => 2.,
    //         PiecePerson::Bishop => 3.,
    //         PiecePerson::Queen => 5.,
    //         PiecePerson::King { .. } => panic!("King has no value"),
    //     }
    // }

    fn get_uci_name(&self) -> String {
        String::from(["p", "r", "n", "b", "q", "k"][self.get_index()])
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
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

    fn get_string(&self) -> &str {
        let index = self.piece_person.get_index();

        match self.color {
            PieceColor::Black => ["♙", "♖", "♘", "♗", "♕", "♔"][index],
            PieceColor::White => ["♟", "♜", "♞", "♝", "♛", "♚"][index],
        }
    }
}

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
pub struct Coordinate {
    pub x: isize,
    pub y: isize,
}

const FILES: [&'static str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

impl Coordinate {
    fn to_uci_coordinate(&self) -> String {
        let x_string = (BOARD_TILE_DIM - self.x).to_string();
        let y_string = FILES[self.y as usize].to_string();
        format!("{}{}", x_string, y_string)
    }

    fn to_text(&self) -> String {
        format!("{}{}", self.x, self.y)
    }

    pub fn value(
        &self,
        piece_person: PiecePerson,
        color: PieceColor,
        weights: &PieceWeights,
    ) -> f32 {
        color.convert_signed(
            piece_person.value(weights)
                + BOARD_WEIGHTS[self.x as usize]
                + BOARD_WEIGHTS[self.y as usize],
        )
    }
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

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum Side {
    QueenSide,
    KingsSide,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
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

impl Move {
    // fn to_byte(&self) -> u16 {
    //     // let output = [false ; 8];
    //     let website = 0u16;
    //
    //     match self {
    //         Move::Regular { .. } => {}
    //         Move::Promote { .. } => {}
    //         Move::Castle { .. } => {}
    //         Move::EnPassant { .. } => {}
    //     }
    //
    //     //
    //     // let website = output.iter().fold(0u16, |v, b| (v << 1) | (*b as u16));
    //     //
    //     // println!("{}", website);
    //
    //     website
    // }

    pub fn move_to_text_cheat(&self) -> String {
        // untested
        // REMEMBER, supposed to be called before we apply the move
        let from_to = match self {
            Move::Regular {
                initial_position,
                final_position,
                ..
            }
            | Move::Promote {
                initial_position,
                final_position,
                ..
            }
            | Move::EnPassant {
                initial_position,
                final_position,
                ..
            } => {
                format!("{}{}", initial_position.to_text(), final_position.to_text())
            }
            Move::Castle { side } => {
                return match side {
                    Side::QueenSide => String::from("cq"),
                    Side::KingsSide => String::from("ck"),
                };
            }
        };

        let capture = match self {
            Move::Regular { move_type, .. } | Move::Promote { move_type, .. } => {
                *move_type == MoveType::Take
            }
            Move::EnPassant { .. } => true,
            Move::Castle { .. } => {
                panic!("Castles should be returned already")
            }
        };

        let capture_string = if capture { "x" } else { "" };

        let prefix = match self {
            Move::Regular { .. } => String::from("r"),
            Move::Promote { piece_person, .. } => {
                format!("p{}", piece_person.get_uci_name())
            }
            Move::Castle { .. } => {
                panic!("Castles should be returned already")
            }
            Move::EnPassant { .. } => String::from("e"),
        };

        format!("{prefix}{from_to}{capture_string}")
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Square {
    Filled(Piece),
    Empty,
    Boundary,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum MoveType {
    Jump,
    Take,
}

#[derive(Debug)]
pub enum Outcome {
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

fn rank_to_y(rank: Rank) -> usize {
    (BOARD_TILE_DIM as usize - 1) - rank as usize
}

#[derive(Clone)]
pub struct Board {
    pub player_1_color: PieceColor,
    pub turn: PieceColor,
    pub squares: BoardSquares,
    pub move_number: i32,
    white_king_location: Coordinate,
    black_king_location: Coordinate,
}

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

    pub fn get_possible_moves_unchecked(
        &self,
        initial_position: Coordinate,
        piece_person: &PiecePerson,
    ) -> Vec<Move> {
        // todo: untested
        let mut output = Vec::new();

        match piece_person {
            PiecePerson::Pawn { first_move } => {
                let going_up = self.pawn_going_up();
                // println!("Going up: {}", going_up);
                // println!("self.turn: {:?}", self.turn);
                // println!("player_1_color: {:?}", self.player_1_color);

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
                // println!("front: {:?}", front);
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
                            // println!("front move added for pawn.");
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

                // println!("Output before king filter {:?}", output);
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
                if *king_moved == false
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

                            // check that the intermediate Squares are vacant,
                            // check that the intermediate Squares are not under attack

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

                            // check that the intermediate Squares are vacant,
                            // check that the intermediate Squares are not under attack

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
            PiecePerson::Knight => self.check_squares(&initial_position, &KNIGHT_SEARCH_OFFSETS),
        }
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
                    // println!("Going up: {}", going_up);
                    // println!("self.turn: {:?}", self.turn);
                    // println!("player_1_color: {:?}", self.player_1_color);

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
                    // println!("front: {:?}", front);
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
                                // println!("front move added for pawn.");
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

                    // println!("Output before king filter {:?}", output);
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

                                // check that the intermediate Squares are vacant,
                                // check that the intermediate Squares are not under attack

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

                                // check that the intermediate Squares are vacant,
                                // check that the intermediate Squares are not under attack

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

    pub fn get_all_moves_for_turn_iter(&self) -> impl Iterator {
        // todo: untested
        self.squares
            .into_iter()
            .enumerate()
            .flat_map(|(idx, row)| row.into_iter().enumerate().map(move |(idy, square)| (idx, idy, square)))
            .filter_map(|(idx, idy, square)| {
                if let Square::Filled(piece) = square {
                    if piece.color == self.turn {
                        Some((idx, idy, piece.piece_person))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            // .filter(|(idx, idy, piece)| piece.color == self.turn)
            .flat_map(|(idx, idy, piece)| self.get_possible_moves_unchecked(Coordinate { x: idx as isize, y: idy as isize}, &piece))
    }

    pub fn locate_king(&self, search_color: PieceColor) -> Coordinate {
        // println!("Locating king");
        // println!("Second column of board: {:?}", self.Squares[1]);
        // println!("Fifth column of board: {:?}", self.Squares[4]);
        match search_color {
            PieceColor::Black => self.black_king_location,
            PieceColor::White => self.white_king_location,
        }
    }

    fn update_king_location(&mut self, new_location: Coordinate) {
        match self.turn {
            // we are only going to be updating the king's location when it is our turn, so
            PieceColor::Black => self.black_king_location = new_location,
            PieceColor::White => self.white_king_location = new_location,
        }
    }

    pub fn check_check(&self, color: PieceColor, king_location: Coordinate) -> bool {
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
        match instruction {
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
                            piece_person: PiecePerson::King { .. },
                        }) => {
                            self.update_king_location(*final_position);
                            Piece {
                                color,
                                piece_person: PiecePerson::King { moved: true },
                            }
                        }

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

                let new_king_piece = // todo: replace this with just setting the king's piece because it is faster
                    match self.squares[kings_position.x as usize][kings_position.y as usize] {
                        Square::Filled(Piece {
                            color,
                            piece_person: PiecePerson::King { moved: false },
                        }) => Piece {
                            color,
                            piece_person: PiecePerson::King { moved: true },
                        },
                        _ => {
                            panic!("No unmoved king where unmoved king expected")
                        }
                    };

                match side {
                    Side::KingsSide => {
                        let final_position = kings_position + (2, 0);
                        self.update_king_location(final_position);

                        self.replace_piece(&kings_position, &final_position, new_king_piece);
                        // same thing for the rook

                        let rooks_position = Coordinate {
                            x: BOARD_TILE_DIM - 1,
                            y: row_to_act,
                        };

                        let new_rook_piece = match self.squares[rooks_position.x as usize] // todo: replace this with just setting the rooks piece because it is faster
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
                        let final_position = kings_position + (-2, 0);
                        self.update_king_location(final_position);

                        self.replace_piece(&kings_position, &final_position, new_king_piece);
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
    }
    //
    // fn reverse_apply_move(&mut self, piece_move: Move, original_square: Square, overwritten_square: Square) {
    //     match piece_move {
    //         Move::Regular { initial_position, final_position, .. } | Move::Promote { initial_position, final_position, .. } => {
    //             let fx = final_position.x as usize;
    //             let fy = final_position.y as usize;
    //             let ix = initial_position.x as usize;
    //             let iy = initial_position.y as usize;
    //
    //             self.Squares[ix][iy] = original_square;
    //             self.Squares[fx][fy] = overwritten_square;
    //         }
    //         Move::Castle { .. } => {}
    //         Move::EnPassant { .. } => {}
    //     }
    // }

    pub fn move_from_san(&self, san: San) -> Move {
        match san {
            San::Normal {
                role,
                file,
                rank,
                capture,
                to,
                promotion,
            } => {
                let mut files = Vec::from_iter(0..BOARD_TILE_DIM as usize);
                let mut ranks = Vec::from_iter(0..BOARD_TILE_DIM as usize);

                if let Some(file) = file {
                    files = vec![file as usize];
                }

                if let Some(rank) = rank {
                    ranks = vec![rank_to_y(rank)];
                }

                // if file_known && rank_known {
                //     // todo: just apply the move that this defines
                // }

                let mut moves: Vec<Move> = vec![];

                for idx in files {
                    for idy in &ranks {
                        if let Square::Filled(Piece {
                            color,
                            piece_person,
                        }) = self.squares[idx][*idy]
                        {
                            if piece_person.compare_with_role(role) && color == self.turn {
                                moves.extend(
                                    self.get_possible_moves(Coordinate {
                                        x: idx as isize,
                                        y: *idy as isize,
                                    })
                                    .unwrap(),
                                );
                            }
                        }
                    }
                }

                let mut move_to_apply = None;

                if let Some(role) = promotion {
                    for piece_move in &moves {
                        if let Move::Promote {
                            initial_position,
                            final_position,
                            move_type,
                            piece_person,
                        } = piece_move
                        {
                            if final_position.x == to.file() as isize
                                && final_position.y == rank_to_y(to.rank()) as isize
                                && (*move_type == MoveType::Take) == capture
                                && piece_person.compare_with_role(role)
                            {
                                move_to_apply = Some(piece_move);
                                break;
                            }
                        }
                    }
                } else if capture
                    && self.squares[to.file() as usize][rank_to_y(to.rank())] == Square::Empty
                {
                    // only if capture is true, but there is no piece to take currently on the "to" square, en passant
                    for piece_move in &moves {
                        if let Move::EnPassant {
                            initial_position,
                            final_position,
                        } = piece_move
                        {
                            if final_position.x == to.file() as isize
                                && final_position.y == rank_to_y(to.rank()) as isize
                            {
                                move_to_apply = Some(piece_move);
                                break;
                            }
                        }
                    }
                } else {
                    for piece_move in &moves {
                        if let Move::Regular {
                            initial_position,
                            final_position,
                            move_type,
                        } = piece_move
                        {
                            if final_position.x == to.file() as isize
                                && final_position.y == rank_to_y(to.rank()) as isize
                                && (*move_type == MoveType::Take) == capture
                            {
                                move_to_apply = Some(piece_move);
                                break;
                            }
                        }
                    }
                };

                match move_to_apply {
                    Some(piece_move) => piece_move.clone(),
                    None => {
                        panic!(
                            "No valid moves found. San: {:#?} Moves Evaluated: {:#?}",
                            san, moves
                        )
                    }
                }

                // Possible Speedup
                // if let Some(role) = promotion {
                //     for piece_move in moves {
                //         if let Move::Promote { initial_position, final_position, move_type, piece_person } = piece_move {
                //              // todo: just apply the move that this defines
                //         }
                //     }
                // }
            }
            San::Castle(castling_side) => {
                let piece_move = Move::Castle {
                    side: match castling_side {
                        CastlingSide::KingSide => Side::KingsSide,
                        CastlingSide::QueenSide => Side::QueenSide,
                    },
                };
                piece_move
            }
            San::Put { .. } => {
                panic!("The san reader returned a san with a put.")
            }
            San::Null => {
                panic!("The san reader returned a san with a null.")
            }
        }
    }

    fn move_to_uci(&self, piece_move: Move) -> String {
        // untested
        // REMEMBER, supposed to be called before we apply the move
        let (initial_position, final_position) = match piece_move {
            Move::Regular {
                initial_position,
                final_position,
                ..
            }
            | Move::Promote {
                initial_position,
                final_position,
                ..
            }
            | Move::EnPassant {
                initial_position,
                final_position,
                ..
            } => (
                initial_position.to_uci_coordinate(),
                final_position.to_uci_coordinate(),
            ),
            Move::Castle { side } => {
                let row = if self.turn == self.player_1_color {
                    "1"
                } else {
                    "8"
                };

                return match side {
                    Side::QueenSide => {
                        format!("e{row}c{row}")
                    }
                    Side::KingsSide => {
                        format!("e{row}g{row}")
                    }
                };
            }
        };

        let capture = match piece_move {
            Move::Regular { move_type, .. } | Move::Promote { move_type, .. } => {
                move_type == MoveType::Take
            }
            Move::EnPassant { .. } => true,
            Move::Castle { .. } => {
                panic!("Castles should be returned already")
            }
        };

        let capture_string = if capture { "x" } else { "" };

        format!("{initial_position}{capture_string}{final_position}")
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

    // pub fn find_computer_move(&self, all_possible_moves: Vec<Move>) -> Move {
    //     // get all the possible moves
    //     // let all_possible_moves = self.get_all_moves_for_turn();
    //
    //     // todo: implement minimax searching
    //
    //     // pick a random one
    //     all_possible_moves.choose(&mut rand::rng()).unwrap().clone()
    // }

    pub fn outcome(&self) -> Outcome {
        let possible_moves = self.get_all_moves_for_turn();
        // println!("Outcome Possible Moves: {:?} \nTurn {:?}", possible_moves, self.turn);
        // println!("Outcome Turn {:?}", self.turn);

        let game_state = if possible_moves.is_empty() {
            if self.check_check(self.turn, self.locate_king(self.turn)) {
                Outcome::Checkmate {
                    winner: self.turn.opposite(),
                }
            } else {
                Outcome::Draw
            }
        } else {
            Outcome::Playing
        };

        game_state
    }

    pub fn sufficient_material(&self) -> bool {
        let mut black_bishop_or_knight = false;
        let mut white_bishop_or_knight = false;
        for row in self.squares.iter() {
            for square in row.iter() {
                if let Square::Filled(piece) = square {
                    match piece.piece_person {
                        PiecePerson::Knight | PiecePerson::Bishop => match piece.color {
                            PieceColor::Black => {
                                if black_bishop_or_knight {
                                    return true;
                                } else {
                                    black_bishop_or_knight = true;
                                }
                            }
                            PieceColor::White => {
                                if white_bishop_or_knight {
                                    return true;
                                } else {
                                    white_bishop_or_knight = true;
                                }
                            }
                        },
                        PiecePerson::King { .. } => {}
                        _ => {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn generate_transposition(&self) -> Transposition {
        let mut output = [[[false; 10]; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

        let player_representing_color_1 = self.turn;

        // if the player whose turn it is, is not the player that is on the bottom, spin the board.
        let spin_board = self.turn != self.player_1_color;

        let iterator = if !spin_board {
            Either::Left(self.squares.iter().enumerate())
        } else {
            Either::Right(self.squares.iter().rev().enumerate())
        };

        for (idx, row) in iterator {
            let iterator2 = if !spin_board {
                Either::Left(row.iter().enumerate())
            } else {
                Either::Right(row.iter().rev().enumerate())
            };

            // so white is always at the bottom

            for (idy, square) in iterator2 {
                match square {
                    Square::Filled(piece) => {
                        let filled = true;
                        let color = piece.color == player_representing_color_1;
                        let mut moved_one_hot = false;
                        let mut en_passant = false;

                        let piece_person_one_hot: usize = match piece.piece_person {
                            PiecePerson::Pawn { first_move } => {
                                if first_move == Some(self.move_number - 1) {
                                    // if it is on the fourth rank of its color and there is a pawn of opposite color next to it, enable en passant
                                    let real_coordinates = if !spin_board {
                                        Coordinate {
                                            x: idx as isize,
                                            y: idy as isize,
                                        }
                                    } else {
                                        Coordinate {
                                            x: (BOARD_TILE_DIM - 1) - (idx as isize),
                                            y: (BOARD_TILE_DIM - 1) - (idy as isize),
                                        }
                                    };

                                    let row_of_passant = if piece.color == self.player_1_color {
                                        // if we are on the bottom
                                        BOARD_TILE_DIM - 4
                                    } else {
                                        3
                                    };

                                    // println!("Row of Passant {:?}", row_of_passant);
                                    // println!("Real coordinates of pawn {:?}", real_coordinates);

                                    if real_coordinates.y == row_of_passant {
                                        let opposite_color = piece.color.opposite();

                                        for offset in [(1, 0), (-1, 0)] {
                                            // println!("Left/Right square {:?}", self.get_square(&(real_coordinates + offset)));

                                            if let Square::Filled(Piece {
                                                color,
                                                piece_person: PiecePerson::Pawn { first_move },
                                            }) = self.get_square(&(real_coordinates + offset))
                                            {
                                                if color == opposite_color {
                                                    // println!("En Passant set as true");
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
                            moved_one_hot, // invert moved one hot so that kings only have an extra true value when they are not moved // undo this inversion because it gives everything an extra true value
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

    pub fn generate_small_transposition(&self) -> SmallTransposition {
        let mut output = [[[false; 12]; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];
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
                        let mut piece_index = piece.piece_person.get_index();

                        if piece.color == PieceColor::Black {
                            piece_index += 6;
                        }

                        output[idx][idy][piece_index] = true;
                    }
                    Square::Empty | Square::Boundary => {}
                }
            }
        }
        output
    }

    pub fn natural_score(&self, weights: &PieceWeights) -> f32 {
        let mut output: f32 = 0.;
        let mut black_king_found = false;
        let mut white_king_found = false;
        for (idx, row) in self.squares.iter().enumerate() {
            for (idy, square) in row.iter().enumerate() {
                if let Square::Filled(piece) = square {
                    if let PiecePerson::King { .. } = piece.piece_person {
                        match piece.color {
                            PieceColor::Black => black_king_found = true,
                            PieceColor::White => white_king_found = true,
                        }
                    } else {
                        let value = piece.piece_person.value(&weights)
                            + BOARD_WEIGHTS[idx]
                            + BOARD_WEIGHTS[idy];
                        // println!("value:{}", value);
                        output += piece.color.convert_signed(value);
                    }
                }
            }
        }

        if !black_king_found {
            return f32::INFINITY;
        } else if !white_king_found {
            return -f32::INFINITY;
        }

        // println!("natural score:{}", output);
        output
    }

    pub fn score_delta(&self, piece_move: &Move, weights: &PieceWeights) -> f32 {
        match piece_move {
            Move::Regular {
                initial_position,
                final_position,
                move_type,
            } => {
                if let Square::Filled(piece) =
                    self.squares[initial_position.x as usize][initial_position.y as usize]
                {
                    let mut output = 0.;

                    if let MoveType::Take = move_type {
                        if let Square::Filled(taken_piece) =
                            self.squares[final_position.x as usize][final_position.y as usize]
                        {
                            if let PiecePerson::King { .. } = taken_piece.piece_person {
                                return match taken_piece.color {
                                    PieceColor::Black => {
                                        f32::INFINITY // if we captured a black king, white is now infinity
                                    }
                                    PieceColor::White => {
                                        -f32::INFINITY // if we captured a black king, white is now infinity
                                    }
                                };
                            } else {
                                output -= final_position.value(
                                    taken_piece.piece_person,
                                    taken_piece.color,
                                    weights,
                                );
                            }
                        } else {
                            panic!("Move is take, but there is no piece to take.")
                        }
                    }

                    if let PiecePerson::King { .. } = piece.piece_person {
                        // if the king has been moved, there is no change in board value
                        return output;
                    }

                    output -= initial_position.value(piece.piece_person, piece.color, weights);
                    output += final_position.value(piece.piece_person, piece.color, weights);

                    output
                } else {
                    panic!("Initial square not filled. {self}, {:?}", piece_move)
                }
            }
            Move::Promote {
                initial_position,
                final_position,
                move_type,
                piece_person: new_piece_person,
            } => {
                let mut output = 0.;

                if let MoveType::Take = move_type {
                    if let Square::Filled(taken_piece) =
                        self.squares[final_position.x as usize][final_position.y as usize]
                    {
                        if let PiecePerson::King { .. } = taken_piece.piece_person {
                            return match taken_piece.color {
                                PieceColor::Black => {
                                    f32::INFINITY // if we captured a black king, white is now infinity
                                }
                                PieceColor::White => {
                                    -f32::INFINITY // if we captured a black king, white is now infinity
                                }
                            };
                        } else {
                            output -= final_position.value(
                                taken_piece.piece_person,
                                taken_piece.color,
                                weights,
                            );
                        }
                    } else {
                        panic!("Move is take, but there is no piece to take.")
                    }
                }

                // self.turn is the same as the pawn's color
                output -= initial_position.value(
                    PiecePerson::Pawn { first_move: None },
                    self.turn,
                    weights,
                );
                output += final_position.value(*new_piece_person, self.turn, weights);

                output
            }
            Move::Castle { side } => {
                let row_to_act: isize = if self.pawn_going_up() {
                    BOARD_TILE_DIM - 1
                } else {
                    0
                };

                let rooks_initial_position;
                let rooks_final_position;

                match side {
                    Side::KingsSide => {
                        // final position of rook, subtract initial position of rook
                        rooks_initial_position = Coordinate {
                            x: BOARD_TILE_DIM - 1,
                            y: row_to_act,
                        };
                        rooks_final_position = rooks_initial_position + (-2, 0);
                    }
                    Side::QueenSide => {
                        // final position of rook, subtract initial position of rook
                        rooks_initial_position = Coordinate {
                            x: 0,
                            y: row_to_act,
                        };
                        rooks_final_position = rooks_initial_position + (3, 0);
                    }
                }
                rooks_final_position.value(PiecePerson::Rook { moved: true }, self.turn, weights)
                    - rooks_initial_position.value(
                        PiecePerson::Rook { moved: true },
                        self.turn,
                        weights,
                    )
            }
            Move::EnPassant {
                initial_position,
                final_position,
            } => {
                // final - initial - value of taken piece
                final_position.value(PiecePerson::Pawn { first_move: None }, self.turn, weights)
                    - initial_position.value(
                        PiecePerson::Pawn { first_move: None },
                        self.turn,
                        weights,
                    )
                    - Coordinate {
                        x: final_position.x,
                        y: initial_position.y,
                    }
                    .value(
                        PiecePerson::Pawn { first_move: None },
                        self.turn.opposite(),
                        weights,
                    )
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

    pub fn new(player_1_color: PieceColor) -> Self {
        let player_2_color = player_1_color.opposite();

        let mut squares: BoardSquares =
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

        let mut y_of_white_king = BOARD_TILE_DIM - 1;
        let mut y_of_black_king = 0isize;

        if player_1_color == PieceColor::Black {
            (y_of_white_king, y_of_black_king) = (y_of_black_king, y_of_white_king);
        }

        Board {
            player_1_color,
            turn: PieceColor::White,
            squares,
            move_number: 0,
            black_king_location: Coordinate {
                x: 4,
                y: y_of_black_king,
            },
            white_king_location: Coordinate {
                x: 4,
                y: y_of_white_king,
            },
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = [[" "; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

        for (idx, row) in self.squares.iter().enumerate() {
            for (idy, square) in row.iter().enumerate() {
                if let Square::Filled(piece) = square {
                    output[idy][idx] = piece.get_string();
                }
            }
        }

        for row in output {
            writeln!(f, "{:?}", row)?;
        }
        Ok(())

        // write!(f, "{:?}", output)
    }
}

#[cfg(test)]
mod tests {
    use approx::{abs_diff_eq, assert_abs_diff_eq};
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use pgn_reader;
    use pgn_reader::{BufferedReader, Skip, Visitor};

    #[test]
    fn test_move_delta_against_regular_board_eval() {
        const DEFAULT_PIECE_WEIGHTS: PieceWeights = PieceWeights {
            pawn: 1.0,
            knight: 2.0,
            bishop: 3.0,
            rook: 4.0,
            queen: 5.0,
        };
        //     [
        // }1., 4., 2., 3., 5.];
        struct MoveCounter {
            value: f32,
            board: Board,
        }

        impl MoveCounter {
            fn new() -> MoveCounter {
                MoveCounter {
                    value: 0.,
                    board: Board::new(PieceColor::White),
                }
            }
        }

        impl Visitor for MoveCounter {
            type Result = usize;

            fn begin_game(&mut self) {
                self.board = Board::new(PieceColor::White);
                self.value = self.board.natural_score(&DEFAULT_PIECE_WEIGHTS);
            }

            fn san(&mut self, san_plus: SanPlus) {
                let piece_move = self.board.move_from_san(san_plus.san);
                // println!("piece_move {:?}", piece_move);
                // println!("board {}", self.board);
                let score_delta = self.board.score_delta(&piece_move, &DEFAULT_PIECE_WEIGHTS);

                self.value += score_delta;

                self.board.apply_move(&piece_move);
                let natural_score = self.board.natural_score(&DEFAULT_PIECE_WEIGHTS);

                let epsilon = 0.0000030;

                if !abs_diff_eq!(natural_score, self.value, epsilon = epsilon) {
                    println!("{}", self.board);
                    println!("{:?}", piece_move);
                    println!("Score Delta: {:?}", score_delta);
                }

                assert_abs_diff_eq!(natural_score, self.value, epsilon = epsilon);
            }

            fn begin_variation(&mut self) -> Skip {
                Skip(true) // stay in the mainline
            }

            fn end_game(&mut self) -> Self::Result {
                1
            }
        }

        // https://lichess.org/d5kge8qf
        // My own
        // https://lichess.org/4si2z6iq

        let pgn = br#"
1. e4 e5 2. Bc4 Nf6 3. d3 Bc5 4. h3 d6 5. a3 Nc6 6. Ne2 Be6 7. Bxe6 fxe6 8. b4 Bb6 9. Nbc3 d5 10. exd5 exd5 11. Bg5 Qd6 12. Bxf6 Qxf6 13. Nxd5 Qxf2+ 14. Kd2 O-O-O 15. Nxb6+ axb6 16. g4 e4 17. Rf1 Qd4 18. Nxd4 Nxd4 19. c3 Nb5 20. d4 c5 21. bxc5 bxc5 22. Kc2 cxd4 23. Qb1 d3+ 24. Kd2 Rd5 25. Rf5 Rxf5 26. gxf5 Nd6 27. Qf1 Rf8 28. Qg2 Rd8 29. Qxg7 e3+ 30. Ke1 Nxf5 31. Qxh7 d2+ 32. Ke2 Ng3+ 33. Kxe3 d1=Q 34. Rxd1 Rxd1 35. Qg8+ Kc7 36. Qxg3+ Kb6 37. Qg6+ Ka7 38. h4 Re1+ 39. Kd2 Ra1 40. Qd6 Ra2+ 41. Kd3 Ra1 42. Kc4 Rh1 43. Qd4+ Ka8 44. a4 Ra1 45. Qd7 Rh1 46. Qd8+ Ka7 47. a5 Ra1 48. h5 b5+ 49. Kxb5 Rb1+ 50. Kc4 Rb8 51. Qc7+ Rb7 52. Qxb7+ Kxb7 53. h6 Ka6 54. h7 Kxa5 55. h8=Q Kb6 56. Qe5 Kc6 57. Kb4 Kd7 58. Qf6 Kc7 59. c4 Kd7 60. c5 Kc7 61. c6 Kb6 62. Qg6 Kc7 63. Kc5 Kd8 64. Qh7 Kc8 65. Qf5+ Kb8 66. Kd6 Ka7 67. c7 Kb6 68. c8=Q Ka7 1-0

1. e4 d6 2. e5 f5 3. exf6

1. e4 Nc6 2. d4 e5 3. d5 Nce7 4. Nf3 Ng6 5. Bc4 Nf6 6. Nc3 Bc5 7. O-O h6 8. Be3 d6 9. Bxc5 dxc5 10. a3 a6 11. b4 b5 12. Bb3 c4 13. Ba2 Qd6 14. a4 O-O 15. axb5 Qxb4 16. Qd2 axb5 17. Rfb1 Qc5 18. Rxb5 Qd6 19. Rbb1 Rd8 20. Bxc4 Rxa1 21. Rxa1 Bb7 22. Ra7 Ba8 23. Nb5 Qc5 24. Rxc7 Qb6 25. Qc3 Nxe4 26. Qd3 Nxf2 27. Qe2 Ng4+ 28. Kh1 Nf2+ 29. Kg1 Nd3+ 30. Kf1 Ndf4 31. Qe4 Bxd5 32. Bxd5 Qxb5+ 33. Bc4 Rd1+ 34. Kf2 Qb6+ 35. Qe3 Qxc7 36. Qb3 Qc5+ 37. Kg3 Nd5 38. c3 Rh1 39. Bxd5 Nh8 40. Nxe5 Qe3+ 41. Nf3 h5 42. h4 Qe7 43. Qb8+ Kh7 44. c4 Ng6 45. Ng5+ Kh6 46. Nxf7+ Kh7 47. Ng5+ Kh6 48. Qg8 Qe5+ 49. Kf2 Qe1+ 50. Kf3 Nxh4+ 51. Kf4 Rf1+ 52. Nf3 g5+ 53. Qxg5+ Kh7 54. Qxh5+ Kg7 55. Qf7+ Kh6 56. Qf8+ Kh5 57. Bf7+ Ng6+ 58. Bxg6+ Kxg6 59. Qf5+ Kg7 60. Qg5+ Kf7 61. Qf5+ Kg7 62. Qg5+ Kf7 63. Qf5+ Ke7 64. Qg5+ Kd7 65. Qg7+ Kc6 66. Qf6+ Kc5 0-1
        "#;
        let mut reader = BufferedReader::new_cursor(&pgn[..]);

        let mut counter = MoveCounter::new();
        let moves = reader.read_game(&mut counter);

        // assert_eq!(add(1, 2), 3);
    }
}
