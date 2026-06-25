use std::fmt;
use std::fs::{File, create_dir_all};
use std::io::Write;

const FILE_A: u64 = 0x0101010101010101;
const FILE_H: u64 = 0x8080808080808080;

const RANK_1: u64 = 0xff;
const RANK_8: u64 = 0xff << 56;

fn slide_and_gather<F: Fn(u64) -> u64>(mut cur: u64, occ: u64, slide: F) -> u64 {
    let mut result = 0;

    loop {
        cur = slide(cur);

        result |= cur;

        if cur & occ != 0 || cur == 0 {
            break;
        }
    }

    result
}

pub fn rook_attacks(from: u32, occ: u64) -> u64 {
    let rook = 1u64 << from;

    let up = slide_and_gather(rook, occ, |x| x << 8);
    let right = slide_and_gather(rook, occ, |x| x << 1 & !FILE_A);
    let down = slide_and_gather(rook, occ, |x| x >> 8);
    let left = slide_and_gather(rook, occ, |x| x >> 1 & !FILE_H);

    up | right | down | left
}

pub fn bishop_attacks(from: u32, occ: u64) -> u64 {
    let bishop = 1u64 << from;

    let left_up = slide_and_gather(bishop, occ, |x| x << 7 & !FILE_H);
    let right_up = slide_and_gather(bishop, occ, |x| x << 9 & !FILE_A);
    let right_down = slide_and_gather(bishop, occ, |x| x >> 7 & !FILE_A);
    let left_down = slide_and_gather(bishop, occ, |x| x >> 9 & !FILE_H);

    left_up | right_up | right_down | left_down
}

fn find_magic(mask: u64) -> (u64, u32) {
    let mut rng = PCG32::new(90847, 5);

    loop {
        let magic = rng.next64() & rng.next64() & rng.next64();
        let mut seen = StaticBitset::new(1 << mask.count_ones());
        let shift = 64 - mask.count_ones();

        let mut perm = mask;

        let mut success = true;

        loop {
            let index = (perm.overflowing_mul(magic).0 >> shift) as usize;

            if seen.get(index) {
                success = false;
                break;
            }

            seen.set(index);

            if perm == 0 {
                break;
            }

            perm = (perm - 1) & mask;
        }

        if success {
            break (magic, shift);
        }
    }
}

fn file_mask(f: usize) -> u64 {
    let word = 1u64 << f;

    let mut acc = 0;

    for i in 0..8 {
        acc |= word << (i * 8);
    }

    acc
}

fn rank_mask(r: usize) -> u64 {
    0xffu64 << (r * 8)
}

fn get_rook_mask(sq: usize) -> u64 {
    let rank = (sq >> 3) & 7;
    let file = sq & 7;

    let edges_h = rank_mask(rank) & (FILE_A | FILE_H);
    let edges_v = file_mask(file) & (RANK_1 | RANK_8);
    let edges = edges_h | edges_v;

    rook_attacks(sq as u32, 0) & !edges
}

fn get_bishop_mask(sq: usize) -> u64 {
    let edges = FILE_A | FILE_H | RANK_1 | RANK_8;
    bishop_attacks(sq as u32, 0) & !edges
}

fn gen_table<M: Fn(usize) -> u64, A: Fn(u32, u64) -> u64>(
    stream: &mut File,
    name: &str,
    get_mask: M,
    get_attacks: A,
) {
    let mut masks = [0u64; 64];
    let mut magics = [0u64; 64];
    let mut shifts = [0u32; 64];

    for sq in 0..64 {
        let mask = get_mask(sq);
        let (magic, shift) = find_magic(mask);

        masks[sq as usize] = mask;
        magics[sq as usize] = magic;
        shifts[sq as usize] = shift;

        let num_entries = 1 << mask.count_ones();
        let mut bbs = vec![0u64; num_entries];

        let mut perm = mask;

        loop {
            let index = (perm.overflowing_mul(magic).0 >> shift) as usize;

            bbs[index] = get_attacks(sq as u32, perm);

            if perm == 0 {
                break;
            }

            perm = (perm - 1) & mask;
        }

        dump_table(
            stream,
            &format!("{}_ATTACK_TABLE_SQ_{}", name, sq),
            &bbs,
            "u64",
        );
    }

    write!(
        stream,
        "pub const {}_ATTACK_TABLE: [&[u64]; 64] = [\n",
        name
    )
    .unwrap();
    for sq in 0..64 {
        write!(stream, "    &{}_ATTACK_TABLE_SQ_{},\n", name, sq).unwrap();
    }
    write!(stream, "];\n\n").unwrap();

    dump_table(
        stream,
        &format!("{}_ATTACK_TABLE_MASK", name),
        &masks,
        "u64",
    );
    dump_table(
        stream,
        &format!("{}_ATTACK_TABLE_SHIFT", name),
        &shifts,
        "u32",
    );
    dump_table(
        stream,
        &format!("{}_ATTACK_TABLE_MAGIC", name),
        &magics,
        "u64",
    );
}

fn dump_table<T: fmt::LowerHex>(file: &mut File, name: &str, data: &[T], ty: &str) {
    write!(file, "pub const {}: [{};{}] = [\n", name, ty, data.len()).unwrap();
    for (i, m) in data.iter().enumerate() {
        if i % 8 == 0 {
            write!(file, "    ").unwrap();
        }

        write!(file, "0x{:x}, ", m).unwrap();

        if (i + 1) % 8 == 0 {
            write!(file, "\n").unwrap();
        }
    }
    write!(file, "];\n\n").unwrap();
}

fn main() {
    generate_attack_tables();
    generate_zobrist_tables();
    println!("cargo:rerun-if-changed=build.rs");
}

struct PCG32 {
    state: u64,
    inc: u64,
}

impl PCG32 {
    pub fn new(state: u64, inc: u64) -> Self {
        let mut s = Self { state, inc };

        let _ = s.next64();

        s
    }

    fn next32(&mut self) -> u32 {
        let oldstate = self.state;
        // Advance internal state
        self.state = oldstate.overflowing_mul(6364136223846793005u64).0 + (self.inc | 1);
        // Calculate output function (XSH RR), uses old state for max ILP
        let xorshifted: u32 = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
        let rot: i32 = (oldstate >> 59) as i32;
        return (xorshifted >> rot) | (xorshifted << ((-rot) & 31));
    }

    fn next64(&mut self) -> u64 {
        return (self.next32() as u64) | (self.next32() as u64) << 32;
    }
}

struct StaticBitset {
    words: Vec<u64>,
}

impl StaticBitset {
    fn new(n: usize) -> Self {
        let nwords = (n + 63) / 64;
        Self {
            words: vec![0; nwords],
        }
    }

    fn get(&self, idx: usize) -> bool {
        ((self.words[idx / 64] >> (idx % 64)) & 1) != 0
    }

    fn set(&mut self, idx: usize) {
        self.words[idx / 64] |= 1u64 << (idx % 64);
    }
}

fn generate_attack_tables() {
    create_dir_all("src/generated").unwrap();
    let mut file = File::create("src/generated/magic.rs").unwrap();

    gen_table(&mut file, "ROOK", get_rook_mask, rook_attacks);
    gen_table(&mut file, "BISHOP", get_bishop_mask, bishop_attacks);
}

fn generate_zobrist_tables() {
    create_dir_all("src/generated").unwrap();
    let mut file = File::create("src/generated/zobrist.rs").unwrap();

    let mut rng = PCG32::new(239823, 3);

    write!(file, "pub const BASE: u64 = 0x{:x};\n\n", rng.next64()).unwrap();
    write!(
        file,
        "pub const TO_MOVE_BLACK: u64 = 0x{:x};\n\n",
        rng.next64()
    )
    .unwrap();

    write!(file, "pub const PIECE: [[u64;64]; 12] = [\n").unwrap();

    for _ in 0..12 {
        write!(file, "     [").unwrap();

        for sq in 0..64 {
            if sq > 0 {
                write!(file, ", ").unwrap();
            }

            write!(file, "0x{:x}", rng.next64()).unwrap();
        }

        write!(file, "],\n").unwrap();
    }

    write!(file, "];\n\n").unwrap();

    let ep_sq: Vec<u64> = (0..64).map(|_| rng.next64()).collect();
    dump_table(&mut file, "EP_SQUARE", &ep_sq, "u64");

    let mut castling = [0u64; 256];

    for i in 0..8 {
        castling[1 << i] = rng.next64();
    }

    for i in 0..256 {
        let mut acc = 0;

        let mut rem = i;
        while rem != 0 {
            let flag = (rem as u32).trailing_zeros() as usize;
            acc ^= castling[1 << flag];
            rem &= rem - 1;
        }

        castling[i] = acc;
    }

    dump_table(&mut file, "CASTLING", &castling, "u64");
}
