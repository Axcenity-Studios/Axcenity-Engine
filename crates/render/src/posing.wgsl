// Compute posing (CO8, renderer/posing.rs, docs/rendering/animation.md):
// each skinned or morphed item is leaned toward its blend shapes and moved
// by its joints once a frame, into the frame's posed buffer that every
// pass's vertex shader (`pose_corner` in shader.wgsl) and ray tracing's
// BLASes read. Two dispatches pose every item of the frame, each mesh's
// data read from the posing arena (`vertices`, `skin_corners`, `moves`):
//
// 1. `cs_shapes`: one thread per move of each item's active shapes (those
//    with a weight other than 0; renderer/posing.rs lists them), adding
//    weight × move to its corner's sums. Shapes overlap, so the sums are
//    atomic, and in fixed point (`FIXED` steps a unit): integer adds give the
//    same total in any order, so a frame poses the same every time.
// 2. `cs_pose`: one thread per posed corner: the rest corner plus its sums
//    (which it sets back to 0 for the next frame), moved by its joints as
//    `Posed::skin` in assets' rig.rs does, written as five words.

// One item to pose. Must match `JobRaw` in renderer/posing.rs.
struct Job {
    // Its first corner in `posed`, and how many it has.
    posed: u32,
    corners: u32,
    // Its first corner's sums (six words a corner), or `NONE` for an item
    // with no active shape.
    sums: u32,
    // Its first joint in `joints`, and how many (0: not skinned).
    joints: u32,
    joint_count: u32,
    // Its active shapes in `shapes`, and all their moves.
    active_first: u32,
    active_count: u32,
    moves: u32,
    // How many moves the items before it have.
    moves_start: u32,
    // Its mesh in the arena: its first corner in `vertices` and its first
    // corner's joints and weights in `skin_corners` (`NONE` without joints).
    vertices: u32,
    skin: u32,
}

// One active shape of an item. Must match `ActiveShape` in
// renderer/posing.rs.
struct Active {
    // Its first move in the arena's `moves`.
    first: u32,
    count: u32,
    // How many moves of the item's active shapes come before it.
    start: u32,
    weight: f32,
}

// The frame's totals. Must match `TotalsRaw` in renderer/posing.rs.
struct Totals {
    jobs: u32,
    corners: u32,
    moves: u32,
    // f32s per corner of `vertices` (`Vertex` in mesh.rs).
    stride: u32,
}

// One joint this frame. Must match `JointRaw` in renderer/skinning.rs.
struct Joint {
    position: mat4x4<f32>,
    normal: mat3x3<f32>,
}

// One corner's joints and how much each counts. Must match `SkinCorner` in
// renderer/skinning.rs.
struct SkinCorner {
    joints: vec4<u32>,
    weights: vec4<f32>,
}

// One corner's move by one shape at weight 1. Must match `MoveRaw` in
// renderer/skinning.rs.
struct Move {
    position: vec3<f32>,
    corner: u32,
    normal: vec3<f32>,
    _pad: u32,
}

@group(0) @binding(0) var<uniform> totals: Totals;
@group(0) @binding(1) var<storage, read> jobs: array<Job>;
@group(0) @binding(2) var<storage, read> shapes: array<Active>;
@group(0) @binding(3) var<storage, read> joints: array<Joint>;
@group(0) @binding(4) var<storage, read_write> sums: array<atomic<i32>>;
@group(0) @binding(5) var<storage, read_write> posed: array<u32>;
// The arena: every posed mesh's corners (`Vertex`s), their joints and
// weights, and the moves of each of its shapes that has been active.
@group(0) @binding(6) var<storage, read> vertices: array<f32>;
@group(0) @binding(7) var<storage, read> skin_corners: array<SkinCorner>;
@group(0) @binding(8) var<storage, read> moves: array<Move>;

// Fixed-point steps a unit (2^20): about a micrometre at a stud's scale,
// with room for ±2048 units of summed moves.
const FIXED: f32 = 1048576.0;
// No one weight × move counts past this, so the sums can't wrap.
const MOST_MOVE: f32 = 1024.0;
const NONE: u32 = 0xffffffffu;
// Words per posed corner: position (three f32s), normal and tangent
// (octahedral, two snorm16s each). Must match `POSED_WORDS` in
// renderer/posing.rs and shader.wgsl.
const POSED_WORDS: u32 = 5u;
// A zero direction (snorm16 -32768 twice, which packing never makes).
// Must match `ZERO_DIRECTION` in shader.wgsl and renderer/posing.rs.
const ZERO_DIRECTION: u32 = 0x80008000u;
// Threads a row of workgroups covers (65,535 workgroups), at each size.
const SHAPES_ROW: u32 = 65535u * 256u;
const POSE_ROW: u32 = 65535u * 64u;

// The last job whose first move is at or before `at`: the one that move is
// in (jobs with no moves share the next one's start and come before it).
fn job_of_move(at: u32) -> u32 {
    var low = 0u;
    var high = totals.jobs - 1u;
    while low < high {
        let middle = (low + high + 1u) / 2u;
        if jobs[middle].moves_start <= at {
            low = middle;
        } else {
            high = middle - 1u;
        }
    }
    return low;
}

// The job posed corner `at` belongs to.
fn job_of_corner(at: u32) -> u32 {
    var low = 0u;
    var high = totals.jobs - 1u;
    while low < high {
        let middle = (low + high + 1u) / 2u;
        if jobs[middle].posed <= at {
            low = middle;
        } else {
            high = middle - 1u;
        }
    }
    return low;
}

fn add_fixed(at: u32, value: vec3<f32>) {
    let steps = vec3<i32>(round(clamp(value, vec3<f32>(-MOST_MOVE), vec3<f32>(MOST_MOVE)) * FIXED));
    if steps.x != 0 {
        atomicAdd(&sums[at], steps.x);
    }
    if steps.y != 0 {
        atomicAdd(&sums[at + 1u], steps.y);
    }
    if steps.z != 0 {
        atomicAdd(&sums[at + 2u], steps.z);
    }
}

@compute @workgroup_size(256)
fn cs_shapes(@builtin(global_invocation_id) id: vec3<u32>) {
    let at = id.x + id.y * SHAPES_ROW;
    if at >= totals.moves {
        return;
    }
    let job = jobs[job_of_move(at)];
    let local = at - job.moves_start;
    // The active shape `local` falls in: the last one starting at or
    // before it.
    var low = job.active_first;
    var high = job.active_first + job.active_count - 1u;
    while low < high {
        let middle = (low + high + 1u) / 2u;
        if shapes[middle].start <= local {
            low = middle;
        } else {
            high = middle - 1u;
        }
    }
    let shape = shapes[low];
    let one = moves[shape.first + local - shape.start];
    let corner = (job.sums + one.corner) * 6u;
    add_fixed(corner, one.position * shape.weight);
    add_fixed(corner + 3u, one.normal * shape.weight);
}

fn unit_or_zero(v: vec3<f32>) -> vec3<f32> {
    let length_squared = dot(v, v);
    return select(vec3<f32>(0.0), v * inverseSqrt(length_squared), length_squared > 0.0);
}

// `v` (of unit length, or zero) as one word: octahedral, two snorm16s.
fn octahedral(v: vec3<f32>) -> u32 {
    let sum = abs(v.x) + abs(v.y) + abs(v.z);
    if sum <= 0.0 {
        return ZERO_DIRECTION;
    }
    var p = v.xy / sum;
    if v.z < 0.0 {
        let sign_of = select(vec2<f32>(-1.0), vec2<f32>(1.0), p >= vec2<f32>(0.0));
        p = (vec2<f32>(1.0) - abs(p.yx)) * sign_of;
    }
    return pack2x16snorm(p);
}

fn take_sum(at: u32) -> f32 {
    return f32(atomicExchange(&sums[at], 0)) / FIXED;
}

@compute @workgroup_size(64)
fn cs_pose(@builtin(global_invocation_id) id: vec3<u32>) {
    let out_corner = id.x + id.y * POSE_ROW;
    if out_corner >= totals.corners {
        return;
    }
    let job = jobs[job_of_corner(out_corner)];
    let corner = out_corner - job.posed;
    let at = (job.vertices + corner) * totals.stride;
    var position = vec3<f32>(vertices[at], vertices[at + 1u], vertices[at + 2u]);
    var normal = vec3<f32>(vertices[at + 3u], vertices[at + 4u], vertices[at + 5u]);
    // After the position, normal and texture coordinates.
    var tangent = vec3<f32>(vertices[at + 8u], vertices[at + 9u], vertices[at + 10u]);
    if job.sums != NONE {
        let sum = (job.sums + corner) * 6u;
        let moved = vec3<f32>(take_sum(sum), take_sum(sum + 1u), take_sum(sum + 2u));
        let turned = vec3<f32>(take_sum(sum + 3u), take_sum(sum + 4u), take_sum(sum + 5u));
        position += moved;
        if any(turned != vec3<f32>(0.0)) {
            normal = unit_or_zero(normal + turned);
        }
    }
    if job.joint_count > 0u && job.skin != NONE {
        let skin = skin_corners[job.skin + corner];
        var matrix = mat4x4<f32>();
        var normal_matrix = mat3x3<f32>();
        var total = 0.0;
        for (var i = 0u; i < 4u; i += 1u) {
            let weight = skin.weights[i];
            let joint = skin.joints[i];
            // Joints past the skin's are left out, as `Posed::skin` does.
            if weight <= 0.0 || joint >= job.joint_count {
                continue;
            }
            let moved = joints[job.joints + joint];
            matrix += moved.position * weight;
            normal_matrix += moved.normal * weight;
            total += weight;
        }
        // A corner weighted to nothing stays where it is.
        if total > 1e-6 {
            position = (matrix * vec4<f32>(position, 1.0)).xyz / total;
            normal = normal_matrix * normal;
            tangent = (matrix * vec4<f32>(tangent, 0.0)).xyz;
        }
    }
    let out = out_corner * POSED_WORDS;
    posed[out] = bitcast<u32>(position.x);
    posed[out + 1u] = bitcast<u32>(position.y);
    posed[out + 2u] = bitcast<u32>(position.z);
    posed[out + 3u] = octahedral(unit_or_zero(normal));
    posed[out + 4u] = octahedral(unit_or_zero(tangent));
}
