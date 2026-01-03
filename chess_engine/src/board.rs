use crate::anti_move::AntiMove;
use crate::coordinate::Coordinate;
use crate::piece::{Piece, PieceColor, PiecePerson};
use crate::piece_move::Move;
use crate::{
    BISHOP_SEARCH_OFFSETS, BOARD_WEIGHTS, BoardSquares, KING_SEARCH_OFFSETS, KNIGHT_SEARCH_OFFSETS,
    MoveType, Outcome, POSSIBLE_PAWN_PROMOTES, PieceWeights, ROOK_SEARCH_OFFSETS, Side,
    SizeOfCoordinate, SizeOfOffset, SmallTransposition, Square, Transposition,
};
use either::Either;
use pgn_reader::{CastlingSide, San};
use std::fmt;
use std::mem::discriminant;

pub const BOARD_TILE_DIM: SizeOfCoordinate = 8;

#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct Board {
    pub player_1_color: PieceColor,
    pub turn: PieceColor,
    pub squares: BoardSquares,
    pub move_number: i32,
    pub white_king_location: Coordinate,
    pub black_king_location: Coordinate,
}

// custom implementation for unusual values
impl Board {
    fn get_square_bounds_check(&self, position: &Coordinate) -> Square {
        let x = position.x as usize;
        let y = position.y as usize;
        if x >= self.squares.len() || y >= self.squares[0].len() {
            Square::Boundary
        } else {
            self.squares[x][y]
        }
    }

    fn possible_jump(&self, position: &Coordinate) -> bool {
        self.get_square_bounds_check(position) == Square::Empty
    }

    fn possible_take(&self, position: &Coordinate) -> bool {
        if let Square::Filled(piece) = self.get_square_bounds_check(position) {
            piece.color != self.turn
        } else {
            false
        }
    }

    fn possible_move(&self, position: &Coordinate) -> Option<MoveType> {
        match self.get_square_bounds_check(position) {
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

    fn cast_ray(
        &self,
        position: &Coordinate,
        offsets: &[(SizeOfOffset, SizeOfOffset)],
    ) -> Vec<Move> {
        let mut output = Vec::with_capacity(27);
        for offset in offsets {
            let mut sight = *position + *offset;

            while let Some(move_type) = self.possible_move(&sight) {
                output.push(Move::Regular {
                    initial_position: *position,
                    final_position: sight,
                    move_type,
                });

                if move_type == MoveType::Take {
                    break;
                }

                sight = sight + *offset;
            }
        }
        output
    }

    /// Similar to cast_ray(), but only checks specific spots around a piece given by offsets.
    /// Used for checking the squares King and Knight can move to.
    fn check_squares(
        &self,
        position: &Coordinate,
        offsets: &[(SizeOfOffset, SizeOfOffset)],
    ) -> Vec<Move> {
        let mut output = Vec::with_capacity(8);
        for offset in offsets {
            let sight = *position + *offset;
            if let Some(move_type) = self.possible_move(&sight) {
                output.push(Move::Regular {
                    initial_position: *position,
                    final_position: sight,
                    move_type,
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
        let mut output = Vec::new();

        match piece_person {
            PiecePerson::Pawn { first_move } => {
                let going_up = self.pawn_going_up();
                // println!("Going up: {}", going_up);
                // println!("self.turn: {:?}", self.turn);
                // println!("player_1_color: {:?}", self.player_1_color);

                let direction: SizeOfOffset = if going_up { -1 } else { 1 };

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
                        }) = self.get_square_bounds_check(&(initial_position + (x_offset, 0)))
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
                let row_to_check = if self.pawn_going_up() {
                    BOARD_TILE_DIM - 1
                } else {
                    0
                };

                // check that the king hasn't moved and is not in check
                // println!("Inital king position: {:?} \nCoordinate to check {:?} \n",initial_position, row_to_check );
                if *king_moved == false
                    && !self.check_check(
                        self.turn,
                        initial_position,
                    )
                {
                    // check king side
                    // check that the rook hasn't moved
                    if let Square::Filled(Piece {
                        piece_person: PiecePerson::Rook { moved: false },
                        ..
                    }) = (self[Coordinate {
                        x: BOARD_TILE_DIM - 1,
                        y: row_to_check,
                    }]) {
                        let mut possible_castle = true;

                        for x in [5, 6] {
                            let intermediate = Coordinate { x, y: row_to_check };

                            // check that the intermediate Squares are vacant,
                            // check that the intermediate Squares are not under attack

                            if self[intermediate] != Square::Empty
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
                        piece_person: PiecePerson::Rook { moved: false },
                        ..
                    }) = (self[Coordinate {
                        x: 0,
                        y: row_to_check,
                    }]) {
                        let mut possible_castle = true;

                        for x in [2, 3] {
                            let intermediate = Coordinate { x, y: row_to_check };

                            // check that the intermediate Squares are vacant,
                            // check that the intermediate Squares are not under attack

                            if self[intermediate] != Square::Empty
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

    pub fn get_possible_moves(&self, initial_position: Coordinate) -> Option<impl Iterator<Item=Move>> {
        if let Square::Filled(piece) = self.get_square_bounds_check(&initial_position) {
            if piece.color != self.turn {
                // look into making this a part of the if-let statement above
                return None;
            }
            // println!("Generating possible moves for {:?}", piece);
            Some(self.filter_legal_moves(
                self.get_possible_moves_unchecked(initial_position, &piece.piece_person).into_iter(),
            ))
        } else {
            None
        }
    }

    pub fn filter_legal_moves(&self, moves: impl Iterator<Item=Move>) -> impl Iterator<Item=Move> {
        moves
            .filter(|piece_move| {
                let mut test_board = self.clone();
                // println!("{:?}", piece_move);
                test_board.apply_move(&piece_move);
                // todo: Don't go through the clone process with a castle because, we already check that castling won't produce check
                !test_board.check_check(self.turn, test_board.locate_king(self.turn))
                // todo: Replace self.turn with test-board.turn.opposite() as it makes more sense
            })
    }

    /// Filter moves for those that do not cause check without cloning the board every time
    pub fn filter_legal_moves_iter(&self, moves: impl Iterator<Item=Move>) -> impl Iterator<Item=Move> {
        let mut board = self.clone(); // todo: rewrite the main code to just mutate self without cloning into a temporary board
        // todo: try to do it with unsafe that gaurentees that self is immutable across each check
        moves
            .filter(move |piece_move| { // todo: investigate this move
                let anti_move = AntiMove::from_piece_move(&board, piece_move);
                // println!("{:?}", piece_move);
                board.apply_move(&piece_move);

                let causes_check = board.check_check(self.turn, board.locate_king(self.turn));

                board.apply_anti_move(&anti_move);

                !causes_check // we want the ones that don't cause check
            })
    }

    pub fn get_all_moves_for_turn(&self) -> Vec<Move> {
        let mut output = Vec::new(); // todo: this should be vec with capacity like 40
        // todo: Make this faster by not enumerating over everything, just looping
        for idx in 0..self.squares.len() {
            for idy in 0..self.squares[0].len() {
                if let Some(piece_moves) = self.get_possible_moves(Coordinate {
                    x: idx as SizeOfCoordinate,
                    y: idy as SizeOfCoordinate,
                }) {
                    output.extend(piece_moves);
                }
            }
        }
        output
    }

    /// Get all the possible moves of the current player using iterators (not loops)
    /// Includes moves that cause check
    pub fn get_all_moves_for_turn_unchecked_iter(&self) -> impl Iterator<Item=Move> {
        self.squares
            .into_iter()
            .enumerate()
            .flat_map(|(idx, row)| {
                row.into_iter()
                    .enumerate()
                    .map(move |(idy, square)| (idx, idy, square))
            })
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
            .flat_map(|(idx, idy, piece)| {
                self.get_possible_moves_unchecked(
                    Coordinate {
                        x: idx as SizeOfCoordinate,
                        y: idy as SizeOfCoordinate,
                    },
                    &piece,
                )
            })
    }

    /// gets all the moves for the current turn that do not cause check by using iterators
    pub fn get_all_moves_for_turn_iter(&self) -> impl Iterator<Item=Move> {
        self.filter_legal_moves_iter(self.get_all_moves_for_turn_unchecked_iter())
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

        for (piece_person, offsets) in [
            (PiecePerson::Rook { moved: true }, ROOK_SEARCH_OFFSETS),
            (PiecePerson::Bishop, BISHOP_SEARCH_OFFSETS),
        ] {
            // println!("Checking the offsets for {:?}", piece_persons);

            for offset in offsets {
                let mut sight = king_location;
                'squares: loop {
                    sight = sight + offset;
                    match self.get_square_bounds_check(&sight) {
                        Square::Empty => {
                            // println!("Skipping empty square {:?}", sight);
                        }

                        Square::Filled(piece) => {
                            if piece.color != color {
                                // println!("Checking for queen check");
                                if piece.piece_person == PiecePerson::Queen
                                    || discriminant(&piece.piece_person)
                                        == discriminant(&piece_person)
                                {
                                    // println!("Check found: {:?}", *piece_person);
                                    return true;
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

        for (piece_person, offsets) in [
            (PiecePerson::King { moved: true }, KING_SEARCH_OFFSETS),
            (PiecePerson::Knight, KNIGHT_SEARCH_OFFSETS),
        ] {
            for offset in offsets {
                if let Square::Filled(piece) =
                    self.get_square_bounds_check(&(king_location + offset))
                {
                    if piece.color != color {
                        if discriminant(&piece.piece_person) == discriminant(&piece_person) {
                            // println!("Check found: {:?}", *piece_person);
                            return true;
                        }
                    }
                }
            }
        }

        let going_up = color == self.player_1_color;
        let threatening_pawn_y_offset: SizeOfOffset = if going_up { -1 } else { 1 };
        let offsets = [
            (-1, threatening_pawn_y_offset),
            (1, threatening_pawn_y_offset),
        ];

        for offset in offsets {
            if let Square::Filled(piece) = self.get_square_bounds_check(&(king_location + offset)) {
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

    /// Applies a move to a board. Assumes that it is a move that can be applied for the current board state.
    pub fn apply_move(&mut self, instruction: &Move) {
        match instruction {
            Move::Regular {
                initial_position,
                final_position,
                ..
            } => {
                // todo: untested
                if let Square::Filled(Piece {
                    color,
                    piece_person,
                }) = self[*initial_position]
                {
                    let replacement_piece = match piece_person {
                        PiecePerson::Pawn { first_move: None } => PiecePerson::Pawn {
                            first_move: Some(self.move_number),
                        },
                        PiecePerson::King { .. } => {
                            self.update_king_location(*final_position);
                            PiecePerson::King { moved: true }
                        }
                        PiecePerson::Rook { moved: false } => PiecePerson::Rook { moved: true },
                        piece => piece,
                    };
                    self[*initial_position] = Square::Empty;
                    self[*final_position] = Square::Filled(Piece {
                        color,
                        piece_person: replacement_piece,
                    });
                } else {
                    panic!("Initial Piece not filled, or border encountered")
                }
            }
            Move::Promote {
                initial_position,
                final_position,
                piece_person,
                ..
            } => {
                // todo: untested
                self[*initial_position] = Square::Empty;
                self[*final_position] = Square::Filled(Piece {
                    color: self.turn,
                    piece_person: *piece_person,
                });
            }
            Move::Castle { side } => {
                let row_to_act = if self.pawn_going_up() {
                    BOARD_TILE_DIM - 1
                } else {
                    0
                };
                let kings_position = Coordinate {
                    x: 4,
                    y: row_to_act,
                };

                let new_king_piece = Piece {
                    color: self.turn,
                    piece_person: PiecePerson::King { moved: true },
                };

                let new_rook_piece = Piece {
                    color: self.turn,
                    piece_person: PiecePerson::Rook { moved: true },
                };

                match side {
                    Side::KingsSide => {
                        let final_position = kings_position + (2 as SizeOfOffset, 0);
                        self.update_king_location(final_position);

                        self.replace_piece(&kings_position, &final_position, new_king_piece);
                        // same thing for the rook

                        let rooks_position = Coordinate {
                            x: BOARD_TILE_DIM - 1,
                            y: row_to_act,
                        };

                        self.replace_piece(
                            &rooks_position,
                            &(rooks_position + (-2 as SizeOfOffset, 0)),
                            new_rook_piece,
                        );
                    }
                    Side::QueenSide => {
                        let final_position = kings_position + (-2 as SizeOfOffset, 0);
                        self.update_king_location(final_position);

                        self.replace_piece(&kings_position, &final_position, new_king_piece);
                        // same thing for the rook

                        let rooks_position = Coordinate {
                            x: 0,
                            y: row_to_act,
                        };

                        self.replace_piece(
                            &rooks_position,
                            &(rooks_position + (3 as SizeOfOffset, 0)),
                            new_rook_piece,
                        );
                    }
                }
            }
            Move::EnPassant {
                initial_position,
                final_position,
            } => {
                // todo: untested
                let pawn_square_being_moved = self[*initial_position];
                self[*initial_position] = Square::Empty;
                self[*final_position] = pawn_square_being_moved;

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

    pub fn apply_anti_move(&mut self, anti_move: &AntiMove) {
        // todo finish this
        match *anti_move {
            AntiMove::RegularOrPromote {
                original_position,
                original_square,
                final_position,
                square_taken,
            } => {
                self[original_position] = original_square;
                self[final_position] = square_taken;

                if let Square::Filled(Piece { color, piece_person : PiecePerson::King {..} }) = self[original_position] {
                    match color {
                        PieceColor::Black => {
                            self.black_king_location = original_position;
                        }
                        PieceColor::White => {
                            self.white_king_location = original_position;
                        }
                    }
                }
            }
            AntiMove::Castle { side } => {
                let row_to_act = if !self.pawn_going_up() {
                    // "not pawn_going_up" because the turn has been flipped
                    (BOARD_TILE_DIM - 1) as usize
                } else {
                    0
                };

                let initial_kings_x = 4;
                let final_kings_x;
                let final_rooks_x;
                let initial_rooks_x;

                match side {
                    Side::QueenSide => {
                        final_kings_x = 2;
                        final_rooks_x = 3;
                        initial_rooks_x = 0;
                    }
                    Side::KingsSide => {
                        final_kings_x = ((BOARD_TILE_DIM - 1) - 1) as usize;
                        final_rooks_x = ((BOARD_TILE_DIM - 1) - 2) as usize;
                        initial_rooks_x = (BOARD_TILE_DIM - 1) as usize;
                    }
                }

                // println!("Side: {:?}", side);
                // println!("Before Antimove Internal: \n{}", self);

                let initial_king_coord = Coordinate { x: initial_kings_x as SizeOfCoordinate, y: row_to_act as SizeOfCoordinate};

                self.squares[final_kings_x][row_to_act] = Square::Empty;
                self.squares[final_rooks_x][row_to_act] = Square::Empty;

                self.squares[initial_kings_x][row_to_act] = Square::Filled(Piece {
                    color: self.turn.opposite(),
                    piece_person: PiecePerson::King { moved: false },
                });

                self.squares[initial_rooks_x][row_to_act] = Square::Filled(Piece {
                    color: self.turn.opposite(),
                    piece_person: PiecePerson::Rook { moved: false },
                });

                match self.turn.opposite() {
                    PieceColor::Black => {
                        self.black_king_location = initial_king_coord ;
                    }
                    PieceColor::White => {
                        self.white_king_location = initial_king_coord;
                    }
                }

                // println!("After Antimove Internal: \n{}", self);
            }
            AntiMove::EnPassant {
                original_position,
                final_position,
            } => {
                self[original_position] = self[final_position];
                self[final_position] = Square::Empty;
                self.squares[final_position.x as usize][original_position.y as usize] =
                    Square::Filled(Piece {
                        color: self.turn, // it is the other players turn right now
                        piece_person: PiecePerson::Pawn {
                            first_move: Some(self.move_number - 2), // todo: double check this
                        },
                    });
            }
        }
        self.move_number -= 1;
        self.turn = self.turn.opposite();
    }

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
                    ranks = vec![crate::rank_to_y(rank)];
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
                                        x: idx as SizeOfCoordinate,
                                        y: *idy as SizeOfCoordinate,
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
                            if final_position.x == to.file() as SizeOfCoordinate
                                && final_position.y
                                    == crate::rank_to_y(to.rank()) as SizeOfCoordinate
                                && (*move_type == MoveType::Take) == capture
                                && piece_person.compare_with_role(role)
                            {
                                move_to_apply = Some(piece_move);
                                break;
                            }
                        }
                    }
                } else if capture
                    && self.squares[to.file() as usize][crate::rank_to_y(to.rank())]
                        == Square::Empty
                {
                    // only if capture is true, but there is no piece to take currently on the "to" square, en passant
                    for piece_move in &moves {
                        if let Move::EnPassant {
                            initial_position,
                            final_position,
                        } = piece_move
                        {
                            if final_position.x == to.file() as SizeOfCoordinate
                                && final_position.y
                                    == crate::rank_to_y(to.rank()) as SizeOfCoordinate
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
                            if final_position.x == to.file() as SizeOfCoordinate
                                && final_position.y
                                    == crate::rank_to_y(to.rank()) as SizeOfCoordinate
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
        self[*final_position] = Square::Filled(promotion_piece);
        self[*initial_position] = Square::Empty;
    }

    /// Determine the outcome of a game, either checkmate one color wins, draw, or playing.
    /// I think I forgot to integrate sufficient material into draw calculation.
    pub fn outcome(&self) -> Outcome {
        let possible_moves: Vec<Move> = self.get_all_moves_for_turn();
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
        // todo, integrate this into outcome for game, I think I forgot
        // todo, cache this so that we are not iterating
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
                                            x: idx as SizeOfCoordinate,
                                            y: idy as SizeOfCoordinate,
                                        }
                                    } else {
                                        Coordinate {
                                            x: (BOARD_TILE_DIM - 1) - (idx as SizeOfCoordinate),
                                            y: (BOARD_TILE_DIM - 1) - (idy as SizeOfCoordinate),
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

                                        for offset in [(1 as SizeOfOffset, 0), (-1, 0)] {
                                            // println!("Left/Right square {:?}", self.get_square(&(real_coordinates + offset)));

                                            if let Square::Filled(Piece {
                                                color,
                                                piece_person: PiecePerson::Pawn { first_move },
                                            }) = self.get_square_bounds_check(
                                                &(real_coordinates + offset),
                                            ) {
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
                if let Square::Filled(piece) = self[*initial_position] {
                    let mut output = 0.;

                    if let MoveType::Take = move_type {
                        if let Square::Filled(taken_piece) = self[*final_position] {
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
                    if let Square::Filled(taken_piece) = self[*final_position] {
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
                let row_to_act = if self.pawn_going_up() {
                    BOARD_TILE_DIM - 1
                } else {
                    0
                };

                let rooks_initial_position;
                let rooks_final_position: Coordinate;

                match side {
                    Side::KingsSide => {
                        // final position of rook, subtract initial position of rook
                        rooks_initial_position = Coordinate {
                            x: BOARD_TILE_DIM - 1,
                            y: row_to_act,
                        };
                        rooks_final_position = rooks_initial_position + (-2 as SizeOfOffset, 0);
                    }
                    Side::QueenSide => {
                        // final position of rook, subtract initial position of rook
                        rooks_initial_position = Coordinate {
                            x: 0,
                            y: row_to_act,
                        };
                        rooks_final_position = rooks_initial_position + (3 as SizeOfOffset, 0);
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
        let mut y_of_black_king = 0;

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

use std::ops::Index;

impl Index<Coordinate> for Board {
    type Output = Square;

    fn index(&self, coordinate: Coordinate) -> &Self::Output {
        &self.squares[coordinate.x as usize][coordinate.y as usize]
    }
}

use std::ops::IndexMut;
impl IndexMut<Coordinate> for Board {
    fn index_mut(&mut self, coordinate: Coordinate) -> &mut Self::Output {
        &mut self.squares[coordinate.x as usize][coordinate.y as usize]
    }
}
