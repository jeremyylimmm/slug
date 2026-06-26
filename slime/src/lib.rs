use pyo3::prelude::*;

mod generated;

#[pymodule]
mod slime {
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use super::generated::magic::*;

    type Sq = usize;

    #[pyclass]
    struct Move(u16);

    const CAN_QCASTLE_WHITE: u8 = 1 << 0;
    const CAN_QCASTLE_BLACK: u8 = 1 << 1;
    const CAN_KCASTLE_WHITE: u8 = 1 << 2;
    const CAN_KCASTLE_BLACK: u8 = 1 << 3;

    #[allow(unused)]
    const BB_RANK_1: u64 = 0x00000000000000ff;
    #[allow(unused)]
    const BB_RANK_2: u64 = 0x000000000000ff00;
    #[allow(unused)]
    const BB_RANK_3: u64 = 0x0000000000ff0000;
    #[allow(unused)]
    const BB_RANK_4: u64 = 0x00000000ff000000;
    #[allow(unused)]
    const BB_RANK_5: u64 = 0x000000ff00000000;
    #[allow(unused)]
    const BB_RANK_6: u64 = 0x0000ff0000000000;
    #[allow(unused)]
    const BB_RANK_7: u64 = 0x00ff000000000000;
    #[allow(unused)]
    const BB_RANK_8: u64 = 0xff00000000000000;

    #[allow(unused)]
    const BB_FILE_A: u64 = 0x0101010101010101;
    #[allow(unused)]
    const BB_FILE_B: u64 = 0x0202020202020202;
    #[allow(unused)]
    const BB_FILE_C: u64 = 0x0404040404040404;
    #[allow(unused)]
    const BB_FILE_D: u64 = 0x0808080808080808;
    #[allow(unused)]
    const BB_FILE_E: u64 = 0x1010101010101010;
    #[allow(unused)]
    const BB_FILE_F: u64 = 0x2020202020202020;
    #[allow(unused)]
    const BB_FILE_G: u64 = 0x4040404040404040;
    #[allow(unused)]
    const BB_FILE_H: u64 = 0x8080808080808080;

    const KCASTLE_FILE_MASK: u64 = BB_FILE_F | BB_FILE_G;
    const QCASTLE_FILE_MASK: u64 = BB_FILE_B | BB_FILE_C | BB_FILE_D;

    #[pyclass(from_py_object)]
    #[derive(Copy, Clone)]
    enum Piece {
        Pawn = 0,
        Knight,
        Bishop,
        Rook,
        Queen,
        King,
    }

    #[pyclass(from_py_object)]
    #[derive(Copy, Clone)]
    enum Side {
        White,
        Black,
    }

    #[pyclass]
    struct Position {
        bbs: [u64; 12],
        board: [Option<Piece>; 64],
        castling: u8,
        ep: Option<Sq>,
        stm: Side,
        halfmove_clock: usize,
        fullmoves: usize,
    }

    #[pymethods]
    impl Position {
        #[new]
        fn new(fen: &str) -> PyResult<Self> {
            let args: Vec<&str> = fen.split_whitespace().collect();

            // parse piece placement

            let placement = parse_fen_piece_placement(
                args.get(0)
                    .ok_or(PyValueError::new_err("no piece placement field"))?,
            )
            .map_err(|x| PyValueError::new_err(x))?;

            let mut board = [None; 64];
            let mut bbs = [0; 12];

            for sq in 0..64 {
                if let Some((piece, side)) = placement[sq] {
                    board[sq] = Some(piece);
                    bbs[bb_idx(piece, side)] |= sq.to_bb();
                }
            }

            if bbs[bb_idx(Piece::King, Side::White)].count_ones() != 1 {
                return Err(PyValueError::new_err("white does not have 1 king"));
            }

            if bbs[bb_idx(Piece::King, Side::Black)].count_ones() != 1 {
                return Err(PyValueError::new_err("black does not have 1 king"));
            }

            // parse stm

            let &stm_str = args
                .get(1)
                .ok_or(PyValueError::new_err("no side-to-move field"))?;

            let stm = if stm_str == "w" {
                Side::White
            } else if stm_str == "b" {
                Side::Black
            } else {
                return Err(PyValueError::new_err(format!(
                    "side-to-move was '{}'",
                    stm_str
                )));
            };

            // parse castling ability

            let &castle_str = args
                .get(2)
                .ok_or(PyValueError::new_err("no castling field"))?;

            let mut castling = 0;

            if castle_str != "-" {
                for c in castle_str.chars() {
                    match c {
                        'K' => castling |= CAN_KCASTLE_WHITE,
                        'k' => castling |= CAN_KCASTLE_BLACK,
                        'Q' => castling |= CAN_QCASTLE_WHITE,
                        'q' => castling |= CAN_QCASTLE_BLACK,
                        c => {
                            return Err(PyValueError::new_err(format!(
                                "castling flag contained '{}'",
                                c
                            )));
                        }
                    }
                }
            }

            // parse en passant square

            let &ep_str = args
                .get(3)
                .ok_or(PyValueError::new_err("no en passant square field"))?;

            let ep = if ep_str == "-" {
                None
            } else {
                Some(Sq::from_san(ep_str).ok_or(PyValueError::new_err(format!(
                    "en passant field '{}' is invalid",
                    ep_str
                )))?)
            };

            // parse halfmove clock

            let &halfmove_clock_str = args
                .get(4)
                .ok_or(PyValueError::new_err("no halfmove clock field"))?;
            let halfmove_clock = halfmove_clock_str.parse::<usize>().map_err(|_| {
                PyValueError::new_err(format!(
                    "invalid halfmove clock field '{}'",
                    halfmove_clock_str
                ))
            })?;

            // parse fullmove number

            let &fullmoves_str = args
                .get(5)
                .ok_or(PyValueError::new_err("no fullmoves field"))?;
            let fullmoves = fullmoves_str.parse::<usize>().map_err(|_| {
                PyValueError::new_err(format!("invalid fullmoves field '{}'", fullmoves_str))
            })?;

            Ok(Self {
                bbs,
                board,
                stm,
                castling,
                ep,
                halfmove_clock,
                fullmoves,
            })
        }

        fn __repr__(&self) -> String {
            let mut out = String::new();

            let black_bb = self.bbs[6..].iter().fold(0, |acc, x| acc | x);

            for r in (0..8).rev() {
                for f in 0..8 {
                    let sq = Sq::from_coords(r, f).unwrap();

                    out.push(if let Some(p) = self.board[sq] {
                        if sq.to_bb() & black_bb != 0 {
                            p.san()
                        } else {
                            p.san().to_ascii_uppercase()
                        }
                    } else {
                        ' '
                    });

                    out.push(' ');
                }

                out.push('\n');
            }

            out.push_str(&format!("Side-to-move: {}\n", self.stm.letter()));

            out.push_str("Castling: ");

            if self.castling & CAN_KCASTLE_WHITE != 0 {
                out.push('K');
            }
            if self.castling & CAN_QCASTLE_WHITE != 0 {
                out.push('Q');
            }
            if self.castling & CAN_KCASTLE_BLACK != 0 {
                out.push('k');
            }
            if self.castling & CAN_QCASTLE_BLACK != 0 {
                out.push('q');
            }

            out.push('\n');

            out.push_str(&format!(
                "En-passant: {}",
                if let Some(ep) = self.ep {
                    ep.san()
                } else {
                    "".to_string()
                }
            ));

            out.push('\n');

            out.push_str(&format!("Halfmove clock: {}\n", self.halfmove_clock));
            out.push_str(&format!("Fullmoves: {}\n", self.fullmoves));

            out
        }

        fn has_kcastle_rights(&self, side: Side) -> bool {
            match side {
                Side::White => self.castling & CAN_KCASTLE_WHITE != 0,
                Side::Black => self.castling & CAN_KCASTLE_BLACK != 0,
            }
        }

        fn has_qcastle_rights(&self, side: Side) -> bool {
            match side {
                Side::White => self.castling & CAN_QCASTLE_WHITE != 0,
                Side::Black => self.castling & CAN_QCASTLE_BLACK != 0,
            }
        }

        fn gen_pseudolegal_moves(&self) -> Vec<Move> {
            let mut moves = vec![];

            let occ = self.occ();

            let white_occ = self.bbs[..6].iter().fold(0, |acc, x| acc | x);
            let black_occ = self.bbs[6..].iter().fold(0, |acc, x| acc | x);

            let (allies, enemies) = match self.stm {
                Side::White => (white_occ, black_occ),
                Side::Black => (black_occ, white_occ),
            };

            // pawn single and double pushes

            let pawns = self.bb(Piece::Pawn, self.stm);

            let (pawn_pushes, pawn_double_pushes, pawn_push_dir, promotion_rank) = match self.stm {
                Side::White => (
                    white_pawn_pushes(pawns, occ),
                    white_pawn_double_pushes(pawns, occ),
                    1,
                    7,
                ),
                Side::Black => (
                    black_pawn_pushes(pawns, occ),
                    black_pawn_double_pushes(pawns, occ),
                    -1,
                    0,
                ),
            };

            for to in pawn_pushes.iter_bb() {
                let from = (to as i32 - 8 * pawn_push_dir) as usize;

                if to.rank() == promotion_rank {
                    moves.push(Move::new(from, to, Some(Piece::Knight)));
                    moves.push(Move::new(from, to, Some(Piece::Bishop)));
                    moves.push(Move::new(from, to, Some(Piece::Rook)));
                    moves.push(Move::new(from, to, Some(Piece::Queen)));
                } else {
                    moves.push(Move::new(from, to, None));
                }
            }

            for to in pawn_double_pushes.iter_bb() {
                let from = (to as i32 - 16 * pawn_push_dir) as usize;
                moves.push(Move::new(from, to, None));
            }

            // pawn captures

            let ep_bb = if let Some(ep) = self.ep {
                ep.to_bb()
            } else {
                0
            };

            let (pawn_left_captures, pawn_right_captures) = match self.stm {
                Side::White => (
                    white_pawn_left_captures(pawns, enemies | ep_bb),
                    white_pawn_right_captures(pawns, enemies | ep_bb),
                ),
                Side::Black => (
                    black_pawn_left_captures(pawns, enemies | ep_bb),
                    black_pawn_right_captures(pawns, enemies | ep_bb),
                ),
            };

            for captures in [pawn_left_captures, pawn_right_captures] {
                for to in captures.0.iter_bb() {
                    let from = (to as i32 - captures.1) as usize;

                    if to.rank() == promotion_rank {
                        moves.push(Move::new(from, to, Some(Piece::Knight)));
                        moves.push(Move::new(from, to, Some(Piece::Bishop)));
                        moves.push(Move::new(from, to, Some(Piece::Rook)));
                        moves.push(Move::new(from, to, Some(Piece::Queen)));
                    } else {
                        moves.push(Move::new(from, to, None));
                    }
                }
            }

            // knight_moves

            let knights = self.bb(Piece::Knight, self.stm);

            for from in knights.iter_bb() {
                for to in knight_moves(from, allies).iter_bb() {
                    moves.push(Move::new(from, to, None));
                }
            }

            // king moves

            let king = self.bb(Piece::King, self.stm).trailing_zeros();

            for to in king_moves(king as Sq, allies).iter_bb() {
                moves.push(Move::new(king as Sq, to, None));
            }

            // bishop moves

            let bishops = self.bb(Piece::Bishop, self.stm);

            for from in bishops.iter_bb() {
                for to in bishop_moves(from, occ, allies).iter_bb() {
                    moves.push(Move::new(from, to, None));
                }
            }

            // rook moves

            let rooks = self.bb(Piece::Rook, self.stm);

            for from in rooks.iter_bb() {
                for to in rook_moves(from, occ, allies).iter_bb() {
                    moves.push(Move::new(from, to, None));
                }
            }

            // queen moves

            let queens = self.bb(Piece::Queen, self.stm);

            for from in queens.iter_bb() {
                for to in queen_moves(from, occ, allies).iter_bb() {
                    moves.push(Move::new(from, to, None));
                }
            }

            // castling

            let king_sq = self.bb(Piece::King, self.stm).trailing_zeros() as Sq;

            let (home_rank, home_rank_mask) = match self.stm {
                Side::White => (0, BB_RANK_1),
                Side::Black => (7, BB_RANK_8),
            };

            let kcastle_mask = KCASTLE_FILE_MASK & home_rank_mask;
            let qcastle_mask = QCASTLE_FILE_MASK & home_rank_mask;

            let kcastle_path_attacked = kcastle_mask.iter_bb().any(|sq|self.attacked(sq, self.stm.opp()));
            let qcastle_path_attacked = qcastle_mask.iter_bb().any(|sq|self.attacked(sq, self.stm.opp()));

            let can_kcastle = self.has_kcastle_rights(self.stm)
                && (occ & kcastle_mask == 0)
                && !self.checked(self.stm)
                && !kcastle_path_attacked;

            let can_qcastle = self.has_qcastle_rights(self.stm)
                && (occ & qcastle_mask == 0)
                && !self.checked(self.stm)
                && !qcastle_path_attacked;

            if can_kcastle {
                let rook_sq = Sq::from_coords(home_rank, 7).unwrap();
                assert!(king_sq.file() == 4 && king_sq.rank() == home_rank);
                assert!(self.bb(Piece::Rook, self.stm) & rook_sq.to_bb() != 0);
                moves.push(Move::new(king_sq, rook_sq, None));
            }

            if can_qcastle {
                let rook_sq = Sq::from_coords(home_rank, 0).unwrap();
                assert!(king_sq.file() == 4 && king_sq.rank() == home_rank);
                assert!(self.bb(Piece::Rook, self.stm) & rook_sq.to_bb() != 0);
                moves.push(Move::new(king_sq, rook_sq, None));
            }

            moves
        }

        fn occ(&self) -> u64 {
            self.bbs.iter().fold(0, |acc, x| acc | x)
        }

        fn checked(&self, side: Side) -> bool {
            let king_sq = self.bb(Piece::King, side).trailing_zeros() as Sq;
            self.attacked(king_sq, side.opp())
        }

        fn attacked(&self, sq: Sq, attacker: Side) -> bool {
            let occ = self.occ();

            let pawn_attacks = match attacker {
                Side::White => BLACK_PAWN_ATTACKS[sq],
                Side::Black => WHITE_PAWN_ATTACKS[sq],
            };

            if self.bb(Piece::Pawn, attacker) & pawn_attacks != 0 {
                return true;
            }

            if self.bb(Piece::Knight, attacker) & KNIGHT_ATTACKS[sq] != 0 {
                return true;
            }

            let bish = bishop_attacks(sq, occ);
            let rook = rook_attacks(sq, occ);

            if self.bb(Piece::Bishop, attacker) & bish != 0 {
                return true;
            }

            if self.bb(Piece::Rook, attacker) & rook != 0 {
                return true;
            }

            if self.bb(Piece::Queen, attacker) & (bish | rook) != 0 {
                return true;
            }

            if self.bb(Piece::King, attacker) & KING_ATTACKS[sq] != 0 {
                return true;
            }

            false
        }

        fn bb(&self, piece: Piece, side: Side) -> u64 {
            self.bbs[bb_idx(piece, side)]
        }
    }

    fn parse_fen_piece_placement(x: &str) -> Result<[Option<(Piece, Side)>; 64], String> {
        let mut board = [None; _];

        let mut chars = x.chars();

        for r in (0..8).rev() {
            let mut f = 0;

            if r < 7 {
                if !matches!(chars.next(), Some('/')) {
                    return Err(format!("rank {} isn't separated by a '/'", r + 1));
                }
            }

            while f < 8 {
                let sq = Sq::from_coords(r, f).unwrap();

                match chars
                    .next()
                    .ok_or("unexpected end of piece placement".to_string())?
                {
                    c @ '1'..='8' => {
                        f += c as usize - '0' as usize;

                        if f > 8 {
                            return Err(format!(
                                "piece placement rank {} has too many entries",
                                r + 1
                            ));
                        }
                    }

                    c @ ('p' | 'P') => {
                        board[sq] = Some((
                            Piece::Pawn,
                            if c.is_uppercase() {
                                Side::White
                            } else {
                                Side::Black
                            },
                        ));
                        f += 1;
                    }

                    c @ ('n' | 'N') => {
                        board[sq] = Some((
                            Piece::Knight,
                            if c.is_uppercase() {
                                Side::White
                            } else {
                                Side::Black
                            },
                        ));
                        f += 1;
                    }

                    c @ ('b' | 'B') => {
                        board[sq] = Some((
                            Piece::Bishop,
                            if c.is_uppercase() {
                                Side::White
                            } else {
                                Side::Black
                            },
                        ));
                        f += 1;
                    }

                    c @ ('r' | 'R') => {
                        board[sq] = Some((
                            Piece::Rook,
                            if c.is_uppercase() {
                                Side::White
                            } else {
                                Side::Black
                            },
                        ));
                        f += 1;
                    }

                    c @ ('q' | 'Q') => {
                        board[sq] = Some((
                            Piece::Queen,
                            if c.is_uppercase() {
                                Side::White
                            } else {
                                Side::Black
                            },
                        ));
                        f += 1;
                    }

                    c @ ('k' | 'K') => {
                        board[sq] = Some((
                            Piece::King,
                            if c.is_uppercase() {
                                Side::White
                            } else {
                                Side::Black
                            },
                        ));
                        f += 1;
                    }

                    c => {
                        return Err(format!("expected piece placement, got '{}'", c));
                    }
                }
            }
        }

        Ok(board)
    }

    trait SquareMethods {
        fn to_bb(&self) -> u64;
        fn file(&self) -> usize;
        fn rank(&self) -> usize;
        fn from_san(san: &str) -> Option<Self>
        where
            Self: Sized;
        fn from_coords(rank: usize, file: usize) -> Option<Self>
        where
            Self: Sized;
        fn san(&self) -> String;
    }

    impl SquareMethods for Sq {
        fn to_bb(&self) -> u64 {
            1u64 << self
        }

        fn file(&self) -> usize {
            self & 7
        }

        fn rank(&self) -> usize {
            (self >> 3) & 7
        }

        fn from_coords(rank: usize, file: usize) -> Option<Self>
        where
            Self: Sized,
        {
            if file >= 8 || rank >= 8 {
                None
            } else {
                Some(rank * 8 + file)
            }
        }

        fn from_san(san: &str) -> Option<Self> {
            let chars: Vec<char> = san.chars().collect();

            if chars.len() != 2 {
                return None;
            }

            let file = chars[0] as i32 - 'a' as i32;
            let rank = chars[1] as i32 - '1' as i32;

            if file < 0 || rank < 0 {
                return None;
            }

            Self::from_coords(rank.try_into().unwrap(), file.try_into().unwrap())
        }

        fn san(&self) -> String {
            let file_map: Vec<char> = ('a'..='h').collect();
            let rank_map: Vec<char> = ('1'..='8').collect();

            let file = file_map[self.file()];
            let rank = rank_map[self.rank()];

            format!("{}{}", file, rank)
        }
    }

    fn bb_idx(piece: Piece, side: Side) -> usize {
        piece.id() + side.id() * 6
    }

    impl Piece {
        fn id(&self) -> usize {
            *self as usize
        }

        fn from_id(id: usize) -> Option<Self> {
            match id {
                0 => Some(Piece::Pawn),
                1 => Some(Piece::Knight),
                2 => Some(Piece::Bishop),
                3 => Some(Piece::Rook),
                4 => Some(Piece::Queen),
                5 => Some(Piece::King),
                _ => None,
            }
        }

        fn san(&self) -> char {
            match self {
                Piece::Pawn => 'p',
                Piece::Knight => 'n',
                Piece::Bishop => 'b',
                Piece::Rook => 'r',
                Piece::Queen => 'q',
                Piece::King => 'k',
            }
        }
    }

    impl Side {
        fn id(&self) -> usize {
            *self as usize
        }

        fn letter(&self) -> char {
            match self {
                Side::White => 'w',
                Side::Black => 'b',
            }
        }

        fn opp(&self) -> Side {
            match self {
                Side::White => Side::Black,
                Side::Black => Side::White,
            }
        }
    }

    #[pymethods]
    impl Move {
        #[new]
        fn new(from: Sq, to: Sq, promotion: Option<Piece>) -> Self {
            let p = if let Some(p) = promotion {
                p.id() + 1
            } else {
                0
            };

            Self((from as u16 & 63) | ((to as u16 & 63) << 6) | ((p as u16) << 12))
        }

        fn from(&self) -> Sq {
            (self.0 & 63) as Sq
        }

        fn to(&self) -> Sq {
            ((self.0 >> 6) & 63) as Sq
        }

        fn promotion(&self) -> Option<Piece> {
            let p = self.0 >> 12;

            if p == 0 {
                None
            } else {
                Some(Piece::from_id((p - 1) as usize).unwrap())
            }
        }

        fn __repr__(&self) -> String {
            return format!(
                "{}{}{}",
                self.from().san(),
                self.to().san(),
                if let Some(p) = self.promotion() {
                    format!("{}", p.san())
                } else {
                    "".to_string()
                }
            );
        }
    }

    const WHITE_PAWN_ATTACKS: [u64; 64] = {
        let mut table = [0u64; 64];

        let mut sq = 0;

        while sq < 64 {
            let bb = 1u64 << sq;

            let left = (bb << 7) & !BB_FILE_H;
            let right = (bb << 9) & !BB_FILE_A;

            table[sq] = left | right;

            sq += 1;
        }

        table
    };

    const BLACK_PAWN_ATTACKS: [u64; 64] = {
        let mut table = [0u64; 64];

        let mut sq = 0;

        while sq < 64 {
            let bb = 1u64 << sq;

            let left = (bb >> 9) & !BB_FILE_H;
            let right = (bb >> 7) & !BB_FILE_A;

            table[sq] = left | right;

            sq += 1;
        }

        table
    };

    const KNIGHT_ATTACKS: [u64; 64] = {
        let mut table = [0u64; 64];

        let mut sq = 0;

        while sq < 64 {
            let knight = 1u64 << sq;

            let m0 = knight << 6 & !(BB_FILE_G | BB_FILE_H);
            let m1 = knight << 15 & !(BB_FILE_H);
            let m2 = knight << 17 & !(BB_FILE_A);
            let m3 = knight << 10 & !(BB_FILE_A | BB_FILE_B);
            let m4 = knight >> 6 & !(BB_FILE_A | BB_FILE_B);
            let m5 = knight >> 15 & !(BB_FILE_A);
            let m6 = knight >> 17 & !(BB_FILE_H);
            let m7 = knight >> 10 & !(BB_FILE_G | BB_FILE_H);

            table[sq] = m0 | m1 | m2 | m3 | m4 | m5 | m6 | m7;

            sq += 1;
        }

        table
    };

    const KING_ATTACKS: [u64; 64] = {
        let mut table = [0u64; 64];

        let mut sq = 0;

        while sq < 64 {
            let king = 1u64 << sq;

            let m0 = (king << 7) & !BB_FILE_H;
            let m1 = king << 8;
            let m2 = (king << 9) & !BB_FILE_A;
            let m3 = (king << 1) & !BB_FILE_A;
            let m4 = (king >> 7) & !BB_FILE_A;
            let m5 = king >> 8;
            let m6 = (king >> 9) & !BB_FILE_H;
            let m7 = (king >> 1) & !BB_FILE_H;

            table[sq] = m0 | m1 | m2 | m3 | m4 | m5 | m6 | m7;

            sq += 1;
        }

        table
    };

    fn white_pawn_pushes(bb: u64, occ: u64) -> u64 {
        (bb << 8) & !occ
    }

    fn black_pawn_pushes(bb: u64, occ: u64) -> u64 {
        (bb >> 8) & !occ
    }

    fn white_pawn_double_pushes(bb: u64, occ: u64) -> u64 {
        (white_pawn_pushes(bb & BB_RANK_2, occ) << 8) & !occ
    }

    fn black_pawn_double_pushes(bb: u64, occ: u64) -> u64 {
        (black_pawn_pushes(bb & BB_RANK_7, occ) >> 8) & !occ
    }

    fn white_pawn_left_captures(bb: u64, mask: u64) -> (u64, i32) {
        (((bb << 7) & !BB_FILE_H) & mask, 7)
    }

    fn white_pawn_right_captures(bb: u64, mask: u64) -> (u64, i32) {
        (((bb << 9) & !BB_FILE_A) & mask, 9)
    }

    fn black_pawn_left_captures(bb: u64, mask: u64) -> (u64, i32) {
        (((bb >> 9) & !BB_FILE_H) & mask, -9)
    }

    fn black_pawn_right_captures(bb: u64, mask: u64) -> (u64, i32) {
        (((bb >> 7) & !BB_FILE_A) & mask, -7)
    }

    fn knight_moves(sq: Sq, allies: u64) -> u64 {
        KNIGHT_ATTACKS[sq] & !allies
    }

    fn king_moves(sq: Sq, allies: u64) -> u64 {
        KING_ATTACKS[sq] & !allies
    }

    fn bishop_attacks(sq: Sq, occ: u64) -> u64 {
        let index = ((occ & BISHOP_ATTACK_TABLE_MASK[sq])
            .overflowing_mul(BISHOP_ATTACK_TABLE_MAGIC[sq])
            .0
            >> BISHOP_ATTACK_TABLE_SHIFT[sq]) as usize;

        BISHOP_ATTACK_TABLE[sq][index]
    }

    fn bishop_moves(sq: Sq, occ: u64, allies: u64) -> u64 {
        bishop_attacks(sq, occ) & !allies
    }

    fn rook_attacks(sq: Sq, occ: u64) -> u64 {
        let index = ((occ & ROOK_ATTACK_TABLE_MASK[sq])
            .overflowing_mul(ROOK_ATTACK_TABLE_MAGIC[sq])
            .0
            >> ROOK_ATTACK_TABLE_SHIFT[sq]) as usize;

        ROOK_ATTACK_TABLE[sq][index]
    }

    fn rook_moves(sq: Sq, occ: u64, allies: u64) -> u64 {
        rook_attacks(sq, occ) & !allies
    }

    fn queen_moves(sq: Sq, occ: u64, allies: u64) -> u64 {
        bishop_moves(sq, occ, allies) | rook_moves(sq, occ, allies)
    }

    struct BBIterator {
        value: u64,
    }

    impl Iterator for BBIterator {
        type Item = usize;

        fn next(&mut self) -> Option<Self::Item> {
            if self.value != 0 {
                let x = self.value.trailing_zeros() as usize;
                self.value &= self.value - 1;
                Some(x)
            } else {
                None
            }
        }
    }

    trait Bitboard {
        fn iter_bb(&self) -> BBIterator;
    }

    impl Bitboard for u64 {
        fn iter_bb(&self) -> BBIterator {
            BBIterator { value: *self }
        }
    }
}
