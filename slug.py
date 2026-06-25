from slime import Piece, Position, Move

STARTPOS_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"

mv = Move(0, 32, Piece.Queen)
print(f"{mv}");

pos = Position(STARTPOS_FEN)
print(f"{pos}")