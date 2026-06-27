from slime import Position
import time

STARTPOS_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
HANGING_PIECE_FEN = "rnbqkb1r/1ppppppp/p4n2/4P3/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 3"
MATE_IN_ONE_FEN = "rnbqkbnr/2pp1ppp/pp6/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 0 4"

class Edge:
    def __init__(self, P: float, mv):
        self.mv = mv
        self.N = 0
        self.W = 0
        self.P = P
        self.node = None

    def Q(self):
        if self.N == 0:
            return 0
        return self.W / self.N

    def puct(self, n_parent):
        return self.Q() + self.P * (n_parent**0.5) / (1 + self.N)

class Node:
    def __init__(self, board: Position):
        self.board = board
        self.value = None
        self.is_expanded = False
        self.is_terminal = False
        self.children = dict()

def get_legal_moves(pos):
    moves = pos.gen_pseudolegal_moves()

    for i in reversed(range(len(moves))):
        if pos.make_move(moves[i]).checked(pos.stm()):
            moves[i] = moves[-1]
            moves.pop()

    return moves

def evaluate(pos):
    moves = get_legal_moves(pos)

    if len(moves) == 0:
        if pos.checked(pos.stm()):
            value = -1
        else:
            value = 0
        policy = dict()
    else:
        value = pos.relative_material_balance() / (1000+2*500+4*300+8*100)
        policy = {mv.enc(): (mv, 1/len(moves)) for mv in moves}

    return (policy,value)

def run_one_simulation(root):
    cur = root
    path = []

    while cur.is_expanded and not cur.is_terminal:
        parent_n = sum(c.N for c in cur.children.values())

        (best_move, best_edge) = max(cur.children.items(), key=lambda x: x[1].puct(parent_n))
        path.append((cur, best_move))

        if best_edge.node is None:
            child_board = cur.board.make_move(best_edge.mv)
            best_edge.node = Node(child_board)

        cur = best_edge.node

    if not cur.is_expanded:
        policy, value = evaluate(cur.board)
        cur.value = value
        cur.children = {mv.enc(): Edge(p, mv) for mv, p in policy.values()}
        cur.is_expanded = True
        cur.is_terminal = len(cur.children) == 0

    value = cur.value

    for node, move_key in reversed(path):
        edge = node.children[move_key]
        edge.N += 1

        value *= -1
        edge.W += value

def get_pv(root):
    pv = []

    cur = root

    while cur and len(cur.children) > 0:
        _, e = max(cur.children.items(), key=lambda x: x[1].N)
        pv.append(e.mv)
        cur = e.node

    return pv

def go(pos):
    root = Node(pos)
    num_simulations = 100000

    for i in range(num_simulations):
        run_one_simulation(root)

    pv = get_pv(root)
    best_move = pv[0]

    return pv, root.children[best_move.enc()].Q(), sum(e.N for e in root.children.values())

current_pos = Position(STARTPOS_FEN)

while True:
    line = input()
    args = line.split()

    if args[0] == "uci":
        print("id name slug 0.1.0")
        print("id author Jeremy Lim")
        print("option name Hash type spin default 1 min 1 max 1")
        print("option name Threads type spin default 1 min 1 max 1")
        print("uciok")
    elif args[0] == "isready":
        print("readyok")
    elif args[0] == "quit":
        exit(0)
    elif args[0] == "ucinewgame":
        current_pos = Position(STARTPOS_FEN)
    elif args[0] == "position":
        if len(args) < 2:
            print("specify 'startpos' or 'fen'")
            continue

        if args[1] == "startpos":
            current_pos = Position(STARTPOS_FEN)
            next = 2
        elif args[1] == "fen":
            if len(args) < 8:
                print("expected a FEN string")
                continue

            fen = "".join(args[2:8])

            try:
                current_pos = Position(fen)
            except ValueError as e:
                print(f"malformed fen: {e}") 
                continue

            next = 8
        else:
            print(f"expected 'startpos' or 'fen', got '{args[1]}'")
            continue

        if len(args) <= next:
            continue

        if args[next] != "moves":
            print(f"unexpected token {args[next]}")

        while True:
            next += 1

            if next >= len(args):
                break

            uci_mv = args[next]

            mv = current_pos.from_uci_move(uci_mv)

            legal_moves = get_legal_moves(current_pos)
            assert mv.enc() in set([x.enc() for x in legal_moves]), f"move '{uci_mv}' is not legal"

            current_pos = current_pos.make_move(mv)

    elif args[0] == "go":
        start_time = time.perf_counter()
        pv, score, nodes = go(current_pos)
        end_time = time.perf_counter()

        elapsed = end_time - start_time

        nps = nodes / elapsed

        pv_string = ""

        x = current_pos
        for i, mv in enumerate(pv):
            if i > 0:
                pv_string += " "
            pv_string += x.to_uci_move(mv)
            x = x.make_move(mv)

        print(f"info depth {len(pv)} score cp {int(score*20000)} nodes {nodes} nps {int(nps)} time {int(elapsed*1000)} pv {pv_string}")
        print(f"bestmove {current_pos.to_uci_move(pv[0])}")
    
    elif args[0] == "stop":
        pass
    elif args[0] == "setoption":
        pass
    else:
        pass