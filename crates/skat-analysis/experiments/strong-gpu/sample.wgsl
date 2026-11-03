// SPDX-License-Identifier: EUPL-1.2
struct Job {
    key0: vec4<u32>,
    key1: vec4<u32>,
    // Unseen cards, first and second slot sizes, samples per decision.
    spec: vec4<u32>,
}
@group(0) @binding(0) var<storage, read> jobs: array<Job>;
@group(0) @binding(1) var<storage, read_write> results: array<vec4<u32>>;

fn rot(x: vec4<u32>, k: u32) -> vec4<u32> {
    return (x << vec4(k)) | (x >> vec4(32u-k));
}
fn block(job: Job, count: u32) -> array<vec4<u32>, 4> {
    let constants = vec4(0x61707865u, 0x3320646eu, 0x79622d32u, 0x6b206574u);
    var a = constants;
    var b = job.key0;
    var c = job.key1;
    var d = vec4(count, 0u, 0u, 0u);
    for (var r = 0u; r < 8u; r++) {
        a += b; d = rot(d ^ a, 16u);
        c += d; b = rot(b ^ c, 12u);
        a += b; d = rot(d ^ a, 8u);
        c += d; b = rot(b ^ c, 7u);
        if ((r & 1u) == 0u) {
            b = b.yzwx; c = c.zwxy; d = d.wxyz;
        } else {
            b = b.wxyz; c = c.zwxy; d = d.yzwx;
        }
    }
    return array(a + constants, b + job.key0, c + job.key1, d + vec4(count,0u,0u,0u));
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let per = jobs[0].spec.w;
    if (id.x >= arrayLength(&jobs) * per) { return; }
    let job = jobs[id.x / per];
    var bits = job.spec.x;
    let n = countOneBits(bits);
    var cards: array<u32,32>;
    for (var i = 0u; i < n; i++) {
        cards[i] = bits & (0u-bits);
        bits &= bits-1u;
    }
    var cache: array<vec4<u32>,4>;
    var suspect = 0u;
    for (var i = 0u; i < n; i++) {
        let draw = (id.x % per) * n + i;
        if (i == 0u || draw % 4u == 0u) { cache = block(job, draw / 4u); }
        let v = cache[draw % 4u];
        // A rejection by uniform_below (all bounds fit u32) can only
        // occur at the top of u128. Replay the whole decision on CPU.
        if ((v.y & v.x & v.w) == 0xffffffffu) { suspect = 1u; }
        // First draw selects the unique feasible group split.
        if (i == 0u) { continue; }
        let bound = n-i+1u;
        let words = vec4(v.y, v.x, v.w, v.z);
        var j = 0u;
        for (var k = 0u; k < 4u; k++) {
            j = ((j << 16u) + (words[k] >> 16u)) % bound;
            j = ((j << 16u) + (words[k] & 65535u)) % bound;
        }
        let last = n-i;
        let tmp = cards[j]; cards[j] = cards[last]; cards[last] = tmp;
    }
    var masks = vec3(0u);
    for (var i = 0u; i < n; i++) {
        let slot = u32(i >= job.spec.y) + u32(i >= job.spec.y + job.spec.z);
        masks[slot] |= cards[i];
    }
    results[id.x] = vec4(masks, suspect);
}
