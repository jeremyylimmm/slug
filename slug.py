import time

from slime import Position

STARTPOS_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
KIWIPETE_FEN = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"

def perft(pos, depth):
    if depth <= 0:
        return 1

    side = pos.stm()

    count = 0

    for mv in pos.gen_pseudolegal_moves():
        child = pos.make_move(mv)
        if child.checked(side):
            continue
        count += perft(child, depth-1)

    return count

def splitperft(pos, depth):
    side = pos.stm()

    total = 0

    for mv in pos.gen_pseudolegal_moves():
        child = pos.make_move(mv)
        if child.checked(side):
            continue
        result = perft(child, depth-1)
        print(f"{pos.to_uci_move(mv)}: {result}")
        total += result

    print(f"Total: {total}")

pos = Position(KIWIPETE_FEN)


start_time = time.perf_counter()
result = perft(pos, 6)
end_time = time.perf_counter()

elapsed = end_time - start_time
nps = result / elapsed

print(f"Perft(6) - Kiwipete: {result} ({nps/1000:.6}k nps)")

#pos = pos.make_move(pos.from_uci_move("e5d7"))
#splitperft(pos, 1)