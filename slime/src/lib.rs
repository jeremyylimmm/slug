use pyo3::prelude::*;

#[pymodule]
mod slime {
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    type Sq = usize;

    #[pyclass]
    struct Move(u16);

    const CAN_QCASTLE_WHITE: u8 = 1 << 0;
    const CAN_QCASTLE_BLACK: u8 = 1 << 1;
    const CAN_KCASTLE_WHITE: u8 = 1 << 2;
    const CAN_KCASTLE_BLACK: u8 = 1 << 3;

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

    #[derive(Copy, Clone)]
    enum Side {
        White,
        Black,
    }

    #[pyclass]
    struct Position {
        bb: [u64; 12],
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
            let mut bb = [0; 12];

            for sq in 0..64 {
                if let Some((piece, side)) = placement[sq] {
                    board[sq] = Some(piece);
                    bb[bb_idx(piece, side)] |= sq.to_bb();
                }
            }

            if bb[bb_idx(Piece::King, Side::White)].count_ones() != 1 {
                return Err(PyValueError::new_err("white does not have 1 king"));
            }

            if bb[bb_idx(Piece::King, Side::Black)].count_ones() != 1 {
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
                bb,
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

            let black_bb = self.bb[6..].iter().fold(0, |acc, x| acc | x);

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
}
