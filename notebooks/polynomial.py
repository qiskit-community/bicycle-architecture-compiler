# Copyright contributors to the Bicycle Architecture Compiler project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

from __future__ import annotations
from typing import Iterator, Tuple, Literal, List, Optional, Any, cast
import itertools
from collections import Counter
import copy
import re
import numpy as np

Monomial = Tuple[int, int]
Order = Tuple[int, int]
CastsToPolynomial = List[Monomial] | Monomial | Literal[0, 1]


class Polynomial:
    """Bivariate Polynomials GF(2)[x,y]/<x^l-1,y^m-1>

    Polynomials class used for defining Bicycle Codes (Univariate and Bivariate)
    and manipulating a set of coordinates on such codes. Supports algebraic
    operations and converting to and from vectors and matrices.

    Polynomials are incoded as lists of elements of the Free Group/Group
    Alebgra GF(2)[Z_l x Z_m]. Thus as monomials

    1 = (0,0), x = (1,0), y = (0,1)

    and as polynomials:

    0 = [], 1 =  [(0,0)], x = [(1,0)], y = [(0,1)]

    The zero polynomial is only part of ring and not the multiplive group of monomials
    and so it is best to represent the zero polynomial as [] and not represent the
    zero monomial as it is not needed when using the multiplicative group alone.

    A non-monomial x^a1y^b_1 + ... + a^ty^bt is thus represented by

    [(a1,b1), ..., (at,bt)]

    The value of the order can be accessed via:

    A.l = A.ell = A.d1 -> l , A.m = A.d2 -> m

    A.order -> (l,m) = (d1,d2)

    Note: As the ordering of the monomials is irrelevant one could use a set container instead
    of a list container. The choice really depends on if speed becomes an issue and what
    computations we are doing. Sets are faster as inclusion test (e.g. (1,1) in [(2,1),...]
    for example. See https://wiki.python.org/moin/TimeComplexity. This decision should
    only be made if we need to fully optimize this class at some point.

    How we use this class can slow things down if not used with care. Initialization can be
    expensive due to the calculating the canonical representation. If you know that you are
    creating a non-zero monomial then set the nzmono flag to True to speed things up.
    """

    def __init__(
        self,
        terms: Polynomial | CastsToPolynomial | np.ndarray,
        *,
        order: tuple[int, int],
        nzmono: bool = False,
    ) -> None:
        """Initialization of the Polynomial class.

        Args:
            terms (Polynomial|CastsToPolynomialnomial|np.ndarray):  terms of the polynomial
            order (Tuple[int,int]): order of the polynomial (l,m) or (d1,d2) etc
            nzmono (bool): if the terms represent a non-zero monomial as a tuple or if the
                first element of a list is top be used to create a monomial set this flag
                to increase speed. This is NOT required to create a monomial polynomial.
                Default: False

        Example:

            >>>A = Polynomial([], order=(4,7))
            >>>B = Polynomial([(2,1), (1,0)], order=(3,2))
            >>>C = Polynomial([0,5,(2,3),(7,4),(3,2),(2,3),1], order=(4,7))
            >>>D = Polynomial([(1,0)], order=(2,2))
            >>>E = Polynomial((3,4), order=(12,4))
            >>>F = Polynomial(1, order=(12,6))

        """

        self.d1 = order[0]
        self.d2 = order[1]

        if nzmono is True:
            if not isinstance(terms, tuple):
                raise TypeError("nzmono can only be used with terms of type tuple")
            self.terms: List[Tuple[int, int]] = [
                (terms[0] % self.d1, terms[1] % self.d2)
            ]  # CastsToPolynomial : ignore
        else:
            self.terms = self._make_polynomial_canonical(terms)

    ##-- Properties --##

    def dim(self):
        return self.d1 * self.d2

    def is_zero_poly(self) -> bool:
        if not self.terms:
            return True
        return False

    def is_nonzero(self) -> bool:
        if self.terms:
            return True
        return False

    def is_monomial(self) -> bool:
        """The polynomial '0' is often considered a monomial. But here we do not,
        for consistency with the Monomial class which cannot represent 0."""
        if len(self.terms) == 1:
            return True
        return False

    @property
    def order(self) -> Order:
        """Return the order of the Polynomial"""
        return (self.d1, self.d2)

    @property
    def l(self) -> int:
        return self.d1

    @property
    def ell(self) -> int:
        return self.d1

    @property
    def m(self) -> int:
        return self.d2

    @property
    def T(self) -> Polynomial:
        """Alias for pointwise inverse of the polynomial. Abusing notation this is called T"""
        return self.pointwise_inverse

    @property
    def pointwise_inverse(self) -> Polynomial:
        return Polynomial(
            [Polynomial.m_pow(term, -1) for term in self.terms], order=self.order
        )

    @property
    def inverse(self) -> Polynomial:
        assert self.is_monomial()
        return self.pointwise_inverse

    @property
    def mon(self) -> Monomial:
        if not len(self.terms) == 1:
            raise ValueError(f"Attempted to truncate {self} to a monomial.")
        return self.terms[0]

    ##-- Dunder Methods --##
    def __abs__(self) -> int:
        return len(self.terms)

    def __add__(self, other_in: CastsToPolynomial | Polynomial) -> Polynomial:

        other: Polynomial = Polynomial(other_in, order=self.order)
        if self.is_zero_poly():
            return copy.copy(other)
        terms = self.terms + other.terms

        return Polynomial(terms, order=self.order)

    def __contains__(self, term: CastsToPolynomial | Polynomial) -> bool:
        other = Polynomial(term, order=self.order)
        assert other.is_monomial()
        return other.terms[0] in self.terms

    def __copy__(self) -> Polynomial:
        """Shallow Copy for Polynomial class"""
        return Polynomial(self.terms, order=self.order)

    def __eq__(self, other: Any) -> bool:
        if not isinstance(other, Polynomial):
            other = Polynomial(other, order=self.order)

        if self.order != other.order:
            return False

        return set(self.terms) == set(other.terms)

    def __hash__(self) -> int:
        return hash(str(self))

    def __ne__(self, other: Any) -> bool:
        if not isinstance(other, Polynomial):
            other = Polynomial(other, order=self.order)

        if set(self.terms) != set(other.terms):
            return True

        if self.order != other.order:
            return True

        return False

    def __iter__(self) -> Iterator[Polynomial]:
        for elem in self.terms:
            yield Polynomial(elem, order=self.order)

    def __getitem__(self, key: int) -> Polynomial:
        return Polynomial(self.terms[key], order=self.order)

    def __len__(self) -> int:
        return len(self.terms)

    def __mul__(self, other_in: CastsToPolynomial | Polynomial) -> Polynomial:
        """Multiplication of Polynomials"""
        if isinstance(other_in, Polynomial):
            other = other_in
        else:
            other = Polynomial(other_in, order=self.order)

        terms = []
        for elem1 in self.terms:
            for elem2 in other.terms:
                terms += [Polynomial.m_mul(elem1, elem2)]

        # Canonicalization is done during initilization of Polynomial
        return Polynomial(terms, order=self.order)

    def __pow__(self, other: int) -> Polynomial:
        if other == 0:
            return Polynomial(1, order=self.order)

        if other < 0:
            if self.is_zero_poly():
                raise ZeroDivisionError
            if not self.is_monomial():
                raise ValueError("Division by non-monomials is not supported")
            return self.inverse ** abs(other)

        return self * self ** (other - 1)

    def __radd__(self, other: CastsToPolynomial) -> Polynomial:
        # As addition is commutative
        return self.__add__(other)

    def __repr__(self) -> str:
        """String representation for Polynomial class object."""
        return (
            "Polynomial(["
            + ",".join([str(term) for term in self.terms])
            + "], order="
            + str(self.order)
            + ")"
        )

    def __str__(self, labels: str = "xy", power_symbol: str = "**") -> str:
        """String representation of the Polynomial"""
        if self.is_zero_poly():
            return "0"
        self.terms = self._make_polynomial_canonical(self.terms)
        out_str = " + ".join(
            [self.order_to_mon_str(term, labels, power_symbol) for term in self.terms]
        )
        return out_str

    def __rmul__(self, other: CastsToPolynomial | Polynomial) -> Polynomial:
        return self.__mul__(other)

    def __truediv__(self, other: CastsToPolynomial | Polynomial) -> Polynomial:
        """Division of Monomials only: self / other"""

        if not isinstance(other, Polynomial):
            other = Polynomial(other, order=self.order)
        if other.is_zero_poly():
            raise ZeroDivisionError
        if not other.is_monomial():
            raise ValueError("Division by non-monomials is not supported")
        if self.is_zero_poly():
            return copy.copy(self)
        m_term = other.terms[0]

        return Polynomial(
            [Polynomial.m_div(term, m_term) for term in self.terms], order=self.order
        )

    def __rtruediv__(self, other: CastsToPolynomial | Polynomial) -> Polynomial:
        """Division of Monomials: other / self"""

        if self.is_zero_poly():
            raise ZeroDivisionError
        if not self.is_monomial():
            raise ValueError("Division by non-monomials is not supported")
        other = Polynomial(other, order=self.order)
        if other.is_zero_poly():
            return copy.copy(other)
        m_term = self.terms[0]

        return Polynomial(
            [Polynomial.m_div(term, m_term) for term in other.terms], order=other.order
        )

    def _int_to_exponent_list(self, value: int) -> List[Tuple[int, int]]:
        """Converts input integer into binomial exponent list List[Tuple[int,int]] over GF(2)"""
        if value % 2 == 0:
            return []
        return [(0, 0)]

    ##-- Iterators --##

    @staticmethod
    def iter_monomials(*, order: Order) -> Iterator[Polynomial]:
        """Iterates over all monomials in the order

        Consistent with self.idx() with the group order specified

        Args:
            order: Tuple[int,int]: order of the group order = (d1, d2)
        """
        for m in itertools.product(range(order[0]), range(order[1])):
            yield Polynomial(m, order=order)

    def monomials(self) -> Iterator[Polynomial]:
        """Iterates over all monomials in the order

        Consistent with self.idx() using the instance group order."""
        yield from self.iter_monomials(order=self.order)

    ##-- Public Utilities --##

    @staticmethod
    def m_idx(value: Monomial, *, order: Order) -> int:
        """Convert Monomial into an index: (a,b) -> a.d2 + b"""
        return value[0] * order[1] + value[1]

    @staticmethod
    def m_pow(base: Monomial, power: int) -> Monomial:
        return (base[0] * power, base[1] * power)

    @staticmethod
    def m_mul(left: Monomial, right: Monomial) -> Monomial:
        """Monomial multiplication left * right represented as exponents"""
        return (left[0] + right[0], left[1] + right[1])

    @staticmethod
    def m_div(left: Monomial, right: Monomial) -> Monomial:
        """Monomial division left/right represented as exponents"""
        return (left[0] - right[0], left[1] - right[1])

    def mat(self) -> np.ndarray:
        def recur_mat(shape, term):
            if len(shape) == 0:
                return 1
            l = shape[0]
            a = term[0]
            S = np.zeros((l, l), dtype=int)
            for i in range(l):
                S[i, (i + a) % l] = 1
            return np.kron(S, recur_mat(shape[1:], term[1:]))

        out = np.zeros((self.dim(), self.dim()), dtype=int)
        for term in self.terms:
            out += recur_mat((self.d1, self.d2), term)
        return out % 2

    def vec(self) -> np.ndarray:  # as a row vector
        out = np.zeros((1, self.dim()), dtype=int)
        for m in self.terms:
            out[0, self.m_idx(m, order=self.order)] = 1
        return out

    ##-- Private Utilities --##

    def _make_mon_canonical(
        self, value: Monomial, order: Optional[Order] = None
    ) -> Monomial:
        """Return the canomical representation of the non zero tuple

        0 <= xexp < d1, 0 <= yexp < d2"""
        if order is None:
            order = self.order
        return (value[0] % order[0], value[1] % order[1])

    def _from_np_ndarray(
        self, array: np.ndarray, order: Optional[Order] = None
    ) -> Polynomial:
        """Convert numpy vector representation of a Polynomial to a Polynomial

        Args:
            array: (np.ndarray) : A vector of length d1*d2 described by a numpy array. The array
                could be a one dimensional vector of shape (d1*d2,) or a two dimensional vector
                of shape (1,d1*d2) or (d1*d2,1). If a general two dimensional array is provided
                then it must of the shape (d1*d2, *) and the first row will be used to defined the
                polynomial.
            order: (int,int) : A tuple describing the order: (d1, d2). Default is None. A default
                value of None will mean that the order of the underlying polynomial will be used.

        A vector representation of a Polynomial is relative to the underlying polynomial's idx
        function that indexes the monomials of a given order (d1, d2). That is
        idx(x^my^n) -> index. The ordering defined by this index can then be used to defined a
        vector over GF(2) where a 1 in position `index`indicates the presence of the monomial with
        that index.

        Eg. If idx(x^my^n) = m*d2+n the the polnomial
        Polynomial([(2, 1),(1, 0)], order=(3, 2)) = x**2*y + x
        has the following vector representation:

            [0, 0, 1, 0, 0, 1]

        """
        if order is None:
            order = self.order

        dim = order[0] * order[1]

        assert array.ndim in [1, 2]

        if len(array.shape) == 2:
            if array.shape[0] == 1:
                array = array[0, :]  # row vector
            elif array.shape[1] == 1:
                array = array[:, 0]  # column vector
            else:
                # matrix representation. want first row
                assert array.shape[1] == dim
                array = array[0, :]
        assert array.shape[0] == dim
        return Polynomial(
            [
                alpha.terms[0]
                for i, alpha in enumerate(self.iter_monomials(order=order))
                if array[i] % 2 == 1
            ],
            order=order,
        )

    def _make_polynomial_canonical(
        self,
        terms: CastsToPolynomial | Polynomial | np.ndarray,
        order: Optional[Order] = None,
    ) -> List[Tuple[int, int]]:
        """ """

        mod_terms: CastsToPolynomial
        if order is None:
            order = self.order
        if isinstance(terms, Polynomial):
            mod_terms = terms.terms
        elif isinstance(terms, np.ndarray):
            mod_terms = self._from_np_ndarray(terms, order=order).terms
        elif isinstance(terms, tuple):
            cast(Tuple, terms)
            mod_terms = [terms]
        elif isinstance(terms, int):
            if terms == 0:
                mod_terms = []
            elif terms == 1:
                mod_terms = [(0, 0)]
            else:
                raise ValueError(f"Cannot initialize Polynomial from integer {terms}.")
        else:
            mod_terms = terms

        if not isinstance(mod_terms, list):
            raise ValueError(
                "Input provided for Polynomial class not fully supported: {terms}"
            )

        if not mod_terms:
            return []

        # Find canonical representations for individual terms
        canon = []
        for term in mod_terms:
            if isinstance(term, int):
                canon += self._int_to_exponent_list(term)
            else:
                canon += [self._make_mon_canonical(term, order=order)]

        # Find canonical representation for Polynomial (i.e. remove duplicates)
        count_list = Counter(canon)
        return [item for item in canon if count_list[item] % 2 == 1]

    @staticmethod
    def order_to_mon_str(
        value: Tuple[int, int], labels: str = "xy", power: str = "**"
    ) -> str:
        """Convert an order to an algebraic string"""
        if value == (0, 0):
            return "1"

        return "*".join(
            [
                labels[i] + power + str(term) if term > 1 else labels[i]
                for i, term in enumerate(value)
                if term != 0
            ]
        )

    def mon_idx(self):
        """Assert that the polynomial has just one term, and get its m_idx."""
        return self.m_idx(self.mon, order=self.order)

    def __lt__(self, other: Polynomial) -> bool:
        return self.mon_idx() < other.mon_idx()

    def __gt__(self, other: Polynomial) -> bool:
        return self.mon_idx() > other.mon_idx()

    @staticmethod
    def mon_str_to_order(expr: str, power="**", variables="xy") -> Order:
        """Extract the x and y powers form a str representation of a monomial

            x**n * y**m -> (n,m)

            This includes simplifications on monomials like x -> (1,0) etc
            The multiplication symbol '*' does not need to be present but no
            other symbol is allowed (i.e. not x**3.y**9)

        Args:
            expr: (str) A string expression represenating a monomial in terms
                of variables given by <variables> and power symbol given by <power>
            power: (str) Default is '**'. String used for the power operation.
                '**' (x**2) or  '^' (x^2)
            variables: (str) Default is 'xy'. Variables that the monomial is written
                in terms of: x**2*y**23
        """
        if expr == "1":
            return (0, 0)
        pattern = (
            rf"({variables[0]}(?:\{power}\(?(-?\d+)\)?)?)?"
            r"\s*\*?\s*"
            rf"({variables[1]}(?:\{power}\(?(-?\d+)\)?)?)?"
        )

        match = re.match(pattern, expr)
        if match:
            x_value = (
                int(match.group(2)) if match.group(2) else 1
            )  # Default x exponent to 1
            y_value = (
                int(match.group(4)) if match.group(4) else 1
            )  # Default y exponent to 1
            if match.group(1) and match.group(3):  # Case for both x and y found
                return x_value, y_value
            if match.group(1):  # Only x found
                return x_value, 0  # Return 0 for y
            if match.group(3):  # Only y found
                return 0, y_value  # Return 0 for x
        raise ValueError("Polynomial string not in correct format.")

    @staticmethod
    def poly_str_to_mon_list(string, power="**", variables="xy"):
        string = string.replace(" ", "")
        monomials = string.split("+")
        terms = []
        for mon in monomials:
            terms += [
                Polynomial.mon_str_to_order(mon, power=power, variables=variables)
            ]
        return terms

    @staticmethod
    def poly_str_to_polynomial(
        poly_str: str, order: Order, power="**", variables="xy"
    ) -> Polynomial:
        """Convert a polynomial represented as a string to a Polynomial Class

        Args:
            poly_str (str): String representing a polynomial is at most two variables.
            order (Order): Order if the resulting polynomial object
            power (str): Default is '**'. Symbol used for power
            variables (str): Default is 'xy'. String of two characters representing the two variables.
        """
        return Polynomial(
            Polynomial.poly_str_to_mon_list(poly_str, power=power, variables=variables),
            order=order,
        )
