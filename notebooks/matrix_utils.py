# Copyright contributors to the Bicycle Architecture Compiler project

import numpy as np
from typing import Optional, Union, Iterator


def row_echelon(A: np.ndarray, reduced=False) -> tuple[np.ndarray, np.ndarray]:
    """Finds invertible X such that XA is in row echelon form. Returns XA, X."""
    n = A.shape[0]  # num rows
    m = A.shape[1]  # num cols
    A = np.copy(A)

    X = np.eye(n, dtype=int)

    r = 0  # row of corresponding 1
    for c in range(m):
        # make A[r,c]=1 if possible, and if so make A[r+1:n,c]=1

        # rest of column is empty
        if not any(A[r:n, c] % 2):
            continue

        if not A[r, c] % 2:
            # another row with a 1 in column c
            rp = next(rp for rp in range(r, n) if A[rp, c] % 2 == 1)

            A[r, :] += A[rp, :]
            X[r, :] += X[rp, :]

        # use A[r,:] to delete the 1s in the other rows in the c position

        if reduced:
            for rp in range(r):
                if A[rp, c] % 2:
                    A[rp, :] += A[r, :]
                    X[rp, :] += X[r, :]

        for rp in range(r + 1, n):
            if A[rp, c] % 2:
                A[rp, :] += A[r, :]
                X[rp, :] += X[r, :]
        r += 1

    return A % 2, X % 2


def decompose_row_vector(
    v: np.ndarray, XA: np.ndarray, X: Optional[np.ndarray] = None
) -> tuple[np.ndarray, np.ndarray]:
    """If XA is the row echelon form of a matrix A, decompose the vector v into a linear combination of rows of XA.
    If X is not provided, returns the 'leftover vector' of bits that are not in the rowspace of XA.
    If X is provided, gives both the leftover vector and a vector h such that hA = v + leftovers.
    """
    v = np.copy(v)
    if len(v.shape) == 2:
        v = v[0, :]

    w = np.zeros((1, XA.shape[0]), dtype=int)

    r = 0
    for c in range(XA.shape[1]):
        if XA[r, c] % 2 == 0:
            continue
        if v[c] % 2 == 1:
            v += XA[r, :]
            w[0, r] = 1
        r += 1
        if r >= XA.shape[0]:
            break

    if X is None:
        return v % 2, np.zeros(
            XA.shape[0], dtype=int
        )  # no X provided: give only leftovers - the part of v outside of rowspan of XA
    return (
        v % 2,
        (w @ X) % 2,
    )  # x provided: give both leftovers and a vector h such that hA = v + leftovers


def get_row_nullspace(A: np.ndarray) -> np.ndarray:
    """Outputs a matrix whose rows generate the space of vectors that are orthogonal to all the rows in A."""
    nullspace = np.eye(A.shape[1], dtype=int)

    for i in range(A.shape[0]):
        new_nullspace = []
        non_orthogonal = []
        for j in range(nullspace.shape[0]):
            if np.dot(nullspace[j, :], A[i, :]) % 2 == 1:
                non_orthogonal.append(nullspace[j : j + 1, :])
            else:
                new_nullspace.append(nullspace[j : j + 1, :])

        for j in range(1, len(non_orthogonal)):
            new_nullspace.append(non_orthogonal[j] + non_orthogonal[0])
        if len(new_nullspace) == 0:
            return np.zeros((0, 0))
        nullspace = np.concatenate(new_nullspace)

    return np.concatenate(
        [
            nullspace[i : i + 1, :] % 2
            for i in range(nullspace.shape[0])
            if np.sum(nullspace[i, :] % 2) != 0
        ]
    )


def iter_rowspace(A: np.ndarray) -> Iterator[np.ndarray]:
    """Generator that iterates over linear combinations of rows of A. Skips the all-zero vector."""
    for i in range(1, 2 ** A.shape[0]):
        br = np.binary_repr(i, width=A.shape[0])
        it = sum([A[i : i + 1, :] for i in range(A.shape[0]) if br[i] == "1"]) % 2
        if isinstance(it, int):
            yield np.zeros((1, A.shape[1]))
        else:
            yield it
