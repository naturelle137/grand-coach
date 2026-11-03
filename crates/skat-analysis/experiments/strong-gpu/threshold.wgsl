// SPDX-License-Identifier: EUPL-1.2

// Exact Boolean minimax with one stack per sampled position. Output 2 asks
// the CPU to resolve the position after this bounded search.
struct Node {
    hands: array<u32, 3>,
    turn: u32, count: u32, led: u32, best: u32, winner: u32, points: u32,
    score: u32, remaining: u32, moves: u32,
}
struct Job {
    root: Node, declarer: u32, target_score: u32, limit: u32,
    card: array<u32, 32>, suits: array<u32, 5>,
}
@group(0) @binding(0) var<storage, read> jobs: array<Job>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

fn legal(idx: u32, node: Node) -> u32 {
    let hand = node.hands[node.turn];
    let follow = hand & jobs[idx].suits[jobs[idx].card[node.led] & 7u];
    return select(hand, follow, node.count > 0u && follow != 0u);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= arrayLength(&jobs) { return; }
    // The generator requests the deterministic transition count in the
    // upper output bits. The standalone probe only requests the verdict.
    let counted = (jobs[idx].limit & 0x80000000u) != 0u;
    let cap = jobs[idx].limit & 0x7fffffffu;
    let decl = jobs[idx].declarer;
    let target_score = jobs[idx].target_score;
    var stack: array<Node, 32>;
    var depth = 0u;
    var visited = 0u;
    stack[0] = jobs[idx].root;
    stack[0].moves = legal(idx, stack[0]);

    loop {
        let n = stack[depth];
        let maximise = n.turn == decl;
        var result = false;
        if n.score >= target_score {
            result = true;
        } else if n.score + n.remaining < target_score {
            result = false;
        } else if n.moves == 0u {
            result = !maximise;
        } else {
            if visited >= cap {
                output[idx] = select(0u, visited << 2u, counted) | 2u;
                return;
            }
            visited++;
            let bit = n.moves & (0u - n.moves);
            stack[depth].moves ^= bit;
            let c = firstTrailingBit(bit);
            let info = jobs[idx].card[c];
            var child = n;
            child.hands[n.turn] ^= bit;
            let best = jobs[idx].card[n.best];
            let suit = info & 7u;
            let best_suit = best & 7u;
            if n.count == 0u { child.led = c; }
            if n.count == 0u || (suit == 4u && best_suit != 4u) ||
                (suit == best_suit && (info & 255u) > (best & 255u)) {
                child.best = c;
                child.winner = n.turn;
            }
            child.points += info >> 8u;
            child.count++;
            child.turn = (n.turn + 1u) % 3u;
            if child.count == 3u {
                if child.winner == decl { child.score += child.points; }
                child.remaining -= child.points;
                child.points = 0u;
                child.count = 0u;
                child.turn = child.winner;
            }
            child.moves = legal(idx, child);
            depth++;
            stack[depth] = child;
            continue;
        }

        // One winning child proves a maximising node; one losing child
        // disproves a minimising node. Otherwise try the parent's next move.
        loop {
            if depth == 0u {
                output[idx] = select(0u, visited << 2u, counted) | u32(result);
                return;
            }
            depth--;
            if result != (stack[depth].turn == decl) { break; }
        }
    }
}
