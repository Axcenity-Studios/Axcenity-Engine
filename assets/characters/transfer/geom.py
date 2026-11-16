"""Mesh maths for the transfer spike, on numpy arrays (no scipy: the linear
systems are solved by a small conjugate-gradient loop).

Faces are rows of vertex indices padded with -1 (quads and triangles).
"""

import numpy as np


def triangles(faces):
    """Each quad as two triangles (and each triangle as itself)."""
    faces = np.asarray(faces)
    if faces.shape[1] == 3:
        return faces
    quad = faces[:, 3] >= 0
    return np.concatenate([faces[:, :3], faces[quad][:, [0, 2, 3]]])


def edges(faces):
    """The unique edges (pairs of vertex indices, low first)."""
    faces = np.asarray(faces)
    out = []
    n = faces.shape[1]
    for k in range(n):
        a, b = faces[:, k], faces[:, (k + 1) % n]
        if n == 4:
            # A padded triangle's last corner joins back to its first.
            b = np.where(b < 0, faces[:, 0], b)
        ok = a >= 0
        out.append(np.stack([a[ok], b[ok]], axis=1))
    e = np.concatenate(out)
    e = e[e[:, 0] != e[:, 1]]
    e.sort(axis=1)
    return np.unique(e, axis=0)


def face_normals(V, tris):
    n = np.cross(V[tris[:, 1]] - V[tris[:, 0]], V[tris[:, 2]] - V[tris[:, 0]])
    return n


def vertex_normals(V, tris):
    """Area-weighted unit normals."""
    fn = face_normals(V, tris)
    out = np.zeros_like(V)
    for k in range(3):
        np.add.at(out, tris[:, k], fn)
    return out / np.maximum(np.linalg.norm(out, axis=1, keepdims=True), 1e-12)


def vertex_areas(V, tris):
    a = np.linalg.norm(face_normals(V, tris), axis=1) / 6.0
    out = np.zeros(len(V))
    for k in range(3):
        np.add.at(out, tris[:, k], a)
    return out


def volume(V, tris):
    a, b, c = V[tris[:, 0]], V[tris[:, 1]], V[tris[:, 2]]
    return float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)


class Laplacian:
    """The graph Laplacian of a mesh's edges: `apply(U)` is, per vertex,
    the sum over its edges of (u_i - u_j) * weight."""

    def __init__(self, n, e, weights=None):
        self.n = n
        self.e = e
        self.w = np.ones(len(e)) if weights is None else weights
        self.degree = np.bincount(e[:, 0], self.w, n) + np.bincount(e[:, 1], self.w, n)
        self.count = np.bincount(e[:, 0], minlength=n) + np.bincount(e[:, 1], minlength=n)

    def apply(self, U):
        d = (U[self.e[:, 0]] - U[self.e[:, 1]]) * self.w[:, None]
        out = np.zeros_like(U)
        for k in range(U.shape[1]):
            out[:, k] = np.bincount(self.e[:, 0], d[:, k], self.n) - np.bincount(self.e[:, 1], d[:, k], self.n)
        return out

    def mean(self, U):
        """Each vertex's neighbours' weighted mean."""
        out = np.zeros_like(U)
        for k in range(U.shape[1]):
            out[:, k] = (np.bincount(self.e[:, 0], U[self.e[:, 1], k] * self.w, self.n)
                         + np.bincount(self.e[:, 1], U[self.e[:, 0], k] * self.w, self.n))
        return out / np.maximum(self.degree, 1e-12)[:, None]


def solve(apply, B, diag, X0=None, iterations=600, tol=1e-6):
    """Conjugate gradients with a diagonal preconditioner for a symmetric
    positive system `apply(X) = B` (columns solved together)."""
    X = np.zeros_like(B) if X0 is None else X0.copy()
    R = B - apply(X)
    Z = R / diag[:, None]
    P = Z.copy()
    rz = np.einsum("ij,ij->j", R, Z)
    b2 = np.maximum(np.einsum("ij,ij->j", B, B), 1e-30)
    for _ in range(iterations):
        AP = apply(P)
        alpha = rz / np.maximum(np.einsum("ij,ij->j", P, AP), 1e-30)
        X += P * alpha
        R -= AP * alpha
        if np.all(np.einsum("ij,ij->j", R, R) < tol * tol * b2):
            break
        Z = R / diag[:, None]
        rz_new = np.einsum("ij,ij->j", R, Z)
        P = Z + P * (rz_new / np.maximum(rz, 1e-30))
        rz = rz_new
    return X


def smooth_fit(lap, weight, target, stiffness, X0=None, iterations=600):
    """The field U that minimises
    sum_i weight_i |u_i - target_i|^2 + stiffness * sum_edges w_e |u_i - u_j|^2."""
    diag = weight + stiffness * lap.degree
    return solve(lambda U: U * weight[:, None] + stiffness * lap.apply(U),
                 target * weight[:, None], diag, X0, iterations)


def harmonic_fill(lap, values, known, iterations=3000):
    """`values` with the rows where `known` is false replaced by the
    smoothest (harmonic) field that meets the known rows: each unknown
    row becomes the mean of its neighbours."""
    if known.all():
        return values
    out = np.where(known[:, None], values, 0.0)
    e, w = lap.e, lap.w
    a, b = e[:, 0], e[:, 1]
    free = ~known
    # What the known neighbours contribute to each unknown row.
    rhs = np.zeros_like(out)
    for k in range(out.shape[1]):
        rhs[:, k] = (np.bincount(a, out[b, k] * w * known[b], lap.n)
                     + np.bincount(b, out[a, k] * w * known[a], lap.n))
    inner = free[a] & free[b]
    ea, eb, ew = a[inner], b[inner], w[inner]

    def apply(U):
        U = U * free[:, None]
        res = U * lap.degree[:, None]
        for k in range(U.shape[1]):
            res[:, k] -= np.bincount(ea, U[eb, k] * ew, lap.n) + np.bincount(eb, U[ea, k] * ew, lap.n)
        # Known rows are held: identity there.
        return np.where(free[:, None], res, 0.0)

    X = solve(apply, rhs * free[:, None], np.maximum(lap.degree, 1e-12), None, iterations, 1e-9)
    return np.where(known[:, None], out, X)


def local_frames(P0, P1, e, rings=2):
    """Per vertex, the rotation and the scale that best carry its
    neighbourhood's edges in `P0` to those in `P1` (Kabsch, over the
    vertex's edges, widened `rings - 1` times by averaging the covariance
    over neighbours). Returns (R (n, 3, 3), scale (n,))."""
    n = len(P0)
    d0 = P0[e[:, 1]] - P0[e[:, 0]]
    d1 = P1[e[:, 1]] - P1[e[:, 0]]
    cov = np.zeros((n, 3, 3))
    outer = np.einsum("ei,ej->eij", d1, d0)
    np.add.at(cov, e[:, 0], outer)
    np.add.at(cov, e[:, 1], outer)
    s0 = np.zeros(n)
    s1 = np.zeros(n)
    for a in (0, 1):
        np.add.at(s0, e[:, a], np.einsum("ei,ei->e", d0, d0))
        np.add.at(s1, e[:, a], np.einsum("ei,ei->e", d1, d1))
    for _ in range(rings - 1):
        wide = cov.copy()
        np.add.at(wide, e[:, 0], cov[e[:, 1]])
        np.add.at(wide, e[:, 1], cov[e[:, 0]])
        cov = wide
        w0, w1 = s0.copy(), s1.copy()
        np.add.at(w0, e[:, 0], s0[e[:, 1]])
        np.add.at(w0, e[:, 1], s0[e[:, 0]])
        np.add.at(w1, e[:, 0], s1[e[:, 1]])
        np.add.at(w1, e[:, 1], s1[e[:, 0]])
        s0, s1 = w0, w1
    U, _, Vt = np.linalg.svd(cov)
    det = np.linalg.det(U @ Vt)
    D = np.zeros((n, 3, 3))
    D[:, 0, 0] = D[:, 1, 1] = 1.0
    D[:, 2, 2] = det
    R = U @ D @ Vt
    return R, np.sqrt(s1 / np.maximum(s0, 1e-18))


def rbf_warp(src, dst, X):
    """`X` moved by the smooth space warp that carries the points `src`
    onto `dst`: an affine part plus a biharmonic kernel (phi(r) = r), which
    interpolates the control points exactly."""
    n = len(src)
    K = np.linalg.norm(src[:, None] - src[None], axis=2)
    Pm = np.c_[np.ones(n), src]
    A = np.zeros((n + 4, n + 4))
    A[:n, :n] = K + np.eye(n) * 1e-9
    A[:n, n:] = Pm
    A[n:, :n] = Pm.T
    rhs = np.zeros((n + 4, 3))
    rhs[:n] = dst - src
    coef = np.linalg.solve(A, rhs)
    out = np.empty_like(X)
    for s in range(0, len(X), 4096):
        x = X[s:s + 4096]
        k = np.linalg.norm(x[:, None] - src[None], axis=2)
        out[s:s + 4096] = x + k @ coef[:n] + np.c_[np.ones(len(x)), x] @ coef[n:]
    return out


def mirror_map(shapes, tol=0.01):
    """Each vertex's mirror vertex across x = 0 (a vertex on the middle
    line is its own), for a mesh given in one or more shapes (the two
    frames): the vertex nearest its mirrored position, summed over the
    shapes, since a body that isn't quite symmetric can mislead alone.
    Checked to pair the vertices up."""
    n = len(shapes[0])
    flip = np.array([-1.0, 1.0, 1.0])
    idx = np.empty(n, dtype=np.int64)
    worst = 0.0
    for s in range(0, n, 2048):
        d = sum(np.linalg.norm((P * flip)[s:s + 2048, None] - P[None], axis=2) for P in shapes)
        idx[s:s + 2048] = d.argmin(axis=1)
        worst = max(worst, float(d.min(axis=1).max()) / len(shapes))
    assert worst < tol, f"the mesh isn't mirror symmetric ({worst * 1000:.1f} mm)"
    assert np.array_equal(idx[idx], np.arange(n)), "the mirror map doesn't pair the vertices up"
    return idx
