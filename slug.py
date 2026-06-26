import time

from slime import Position

STARTPOS_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
KIWIPETE_FEN = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"

def splitperft(pos, depth):
    side = pos.stm()

    total = 0

    for mv in pos.gen_pseudolegal_moves():
        child = pos.make_move(mv)
        if child.checked(side):
            continue
        result = child.perft(depth-1)
        print(f"{pos.to_uci_move(mv)}: {result}")
        total += result

    print(f"Total: {total}")

    return total

pos = Position(KIWIPETE_FEN)

perft_depth = 5

start_time = time.perf_counter()
result = splitperft(pos, perft_depth)
end_time = time.perf_counter()

elapsed = end_time - start_time
nps = result / elapsed

print(f"Perft({perft_depth}) - Kiwipete: {result} ({nps/1_000_000:.6}M nps)")