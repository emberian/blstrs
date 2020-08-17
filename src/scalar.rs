//! An implementation of the BLS12-381 scalar field $\mathbb{F}_q$
//! where `q = 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001`

use core::{
    convert::TryInto,
    fmt,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use blst::*;

/// Represents an element of the scalar field $\mathbb{F}_q$ of the BLS12-381 elliptic
/// curve construction.
///
/// The inner representation is stored in Montgomery form.
#[derive(Default, Clone, Copy)]
pub struct Scalar(blst_fr);

/// Representation of a `Scalar`, in regular coordinates.
#[derive(Default, Clone, Copy)]
pub struct ScalarRepr(blst_scalar);

impl AsRef<[u64]> for ScalarRepr {
    fn as_ref(&self) -> &[u64] {
        &self.0.l
    }
}

impl AsMut<[u64]> for ScalarRepr {
    fn as_mut(&mut self) -> &mut [u64] {
        &mut self.0.l
    }
}

const LIMBS: usize = 4;
const LIMB_BITS: usize = 64;

impl fmt::Debug for ScalarRepr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x")?;
        for &b in self.0.l.iter().rev() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl fmt::Display for ScalarRepr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x")?;
        for &b in self.0.l.iter().rev() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl From<u32> for ScalarRepr {
    fn from(val: u32) -> ScalarRepr {
        let mut raw = blst_scalar::default();

        unsafe { blst_scalar_from_uint32(&mut raw as *mut _, val as *const _) };

        ScalarRepr(raw)
    }
}

impl From<u64> for ScalarRepr {
    fn from(val: u64) -> ScalarRepr {
        let mut raw = blst_scalar::default();

        unsafe { blst_scalar_from_uint64(&mut raw as *mut _, val as *const _) };

        ScalarRepr(raw)
    }
}

impl Ord for ScalarRepr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        for (a, b) in self.0.l.iter().rev().zip(other.0.l.iter().rev()) {
            if a < b {
                return std::cmp::Ordering::Less;
            } else if a > b {
                return std::cmp::Ordering::Greater;
            }
        }

        std::cmp::Ordering::Equal
    }
}

impl PartialOrd for ScalarRepr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ScalarRepr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.l == other.0.l
    }
}
impl Eq for ScalarRepr {}

impl fff::PrimeFieldRepr for ScalarRepr {
    fn sub_noborrow(&mut self, other: &Self) {
        let mut borrow = 0;

        for (a, b) in self.0.l.iter_mut().zip(other.0.l.iter()) {
            *a = fff::sbb(*a, *b, &mut borrow);
        }
    }

    fn add_nocarry(&mut self, other: &Self) {
        let mut carry = 0;

        for (a, b) in self.0.l.iter_mut().zip(other.0.l.iter()) {
            *a = fff::adc(*a, *b, &mut carry);
        }
    }

    fn num_bits(&self) -> u32 {
        let mut ret = (LIMBS as u32) * LIMB_BITS as u32;
        for i in self.0.l.iter().rev() {
            let leading = i.leading_zeros();
            ret -= leading;
            if leading != LIMB_BITS as u32 {
                break;
            }
        }

        ret
    }

    fn is_zero(&self) -> bool {
        self.0.l.iter().all(|&e| e == 0)
    }

    fn is_odd(&self) -> bool {
        self.0.l[0] & 1 == 1
    }

    fn is_even(&self) -> bool {
        !self.is_odd()
    }

    fn div2(&mut self) {
        let mut t = 0;
        for i in self.0.l.iter_mut().rev() {
            let t2 = *i << 63;
            *i >>= 1;
            *i |= t;
            t = t2;
        }
    }

    fn shr(&mut self, mut n: u32) {
        if n as usize >= LIMB_BITS * LIMBS {
            *self = Self::from(0u32);
            return;
        }

        while n >= LIMB_BITS as u32 {
            let mut t = 0;
            for i in self.0.l.iter_mut().rev() {
                std::mem::swap(&mut t, i);
            }
            n -= LIMB_BITS as u32;
        }

        if n > 0 {
            let mut t = 0;
            for i in self.0.l.iter_mut().rev() {
                let t2 = *i << (LIMB_BITS as u32 - n);
                *i >>= n;
                *i |= t;
                t = t2;
            }
        }
    }

    fn mul2(&mut self) {
        let mut last = 0;
        for i in &mut self.0.l {
            let tmp = *i >> 63;
            *i <<= 1;
            *i |= last;
            last = tmp;
        }
    }

    fn shl(&mut self, mut n: u32) {
        if n as usize >= LIMB_BITS * LIMBS {
            *self = Self::from(0u32);
            return;
        }

        while n >= LIMB_BITS as u32 {
            let mut t = 0;
            for i in &mut self.0.l {
                std::mem::swap(&mut t, i);
            }
            n -= LIMB_BITS as u32;
        }

        if n > 0 {
            let mut t = 0;
            for i in &mut self.0.l {
                let t2 = *i >> (LIMB_BITS as u32 - n);
                *i <<= n;
                *i |= t;
                t = t2;
            }
        }
    }
}

pub const S: u32 = 32;

impl fmt::Debug for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tmp = self.to_bytes_le();
        write!(f, "0x")?;
        for &b in tmp.iter().rev() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tmp = self.to_bytes_le();
        write!(f, "0x")?;
        for &b in tmp.iter().rev() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl PartialEq for Scalar {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.l == other.0.l
    }
}
impl Eq for Scalar {}

#[derive(Debug, Clone)]
pub struct NotInFieldError;

impl fmt::Display for NotInFieldError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Not in field")
    }
}

impl std::error::Error for NotInFieldError {}

impl TryInto<Scalar> for blst_scalar {
    type Error = NotInFieldError;

    fn try_into(self) -> Result<Scalar, Self::Error> {
        if !unsafe { blst_scalar_fr_check(&self as _) } {
            return Err(NotInFieldError);
        }

        // Safe because valid fr check was just made above.
        let fr: blst_fr = unsafe { std::mem::transmute(self) };

        Ok(Scalar(fr))
    }
}

impl Into<blst_scalar> for &Scalar {
    fn into(self) -> blst_scalar {
        let mut out = blst_fr::default();
        unsafe {
            // transform out of montgomery space
            blst_fr_from(&mut out, &self.0)
        };

        unsafe { std::mem::transmute(out) }
    }
}

impl From<Scalar> for ScalarRepr {
    fn from(val: Scalar) -> Self {
        let raw: blst_scalar = (&val).into();
        ScalarRepr(raw)
    }
}

impl From<u32> for Scalar {
    fn from(val: u32) -> Scalar {
        let mut raw = blst_scalar::default();

        unsafe { blst_scalar_from_uint32(&mut raw as *mut _, val as *const _) };

        raw.try_into().expect("u32 is always inside the field")
    }
}

impl From<u64> for Scalar {
    fn from(val: u64) -> Scalar {
        let mut raw = blst_scalar::default();

        unsafe { blst_scalar_from_uint64(&mut raw as *mut _, val as *const _) };

        raw.try_into().expect("u64 is always inside the field")
    }
}

impl<'a> Neg for &'a Scalar {
    type Output = Scalar;

    #[inline]
    fn neg(self) -> Scalar {
        let mut out = blst_fr::default();

        const FLAG: usize = 0x1;

        unsafe { blst_fr_cneg(&mut out as _, &self.0 as _, FLAG) };

        Scalar(out)
    }
}

impl Neg for Scalar {
    type Output = Scalar;

    #[inline]
    fn neg(self) -> Scalar {
        -&self
    }
}

impl<'a, 'b> Sub<&'b Scalar> for &'a Scalar {
    type Output = Scalar;

    #[inline]
    fn sub(self, rhs: &'b Scalar) -> Scalar {
        self.sub(rhs)
    }
}

impl<'a, 'b> Add<&'b Scalar> for &'a Scalar {
    type Output = Scalar;

    #[inline]
    fn add(self, rhs: &'b Scalar) -> Scalar {
        self.add(rhs)
    }
}

impl<'a, 'b> Mul<&'b Scalar> for &'a Scalar {
    type Output = Scalar;

    #[inline]
    fn mul(self, rhs: &'b Scalar) -> Scalar {
        self.mul(rhs)
    }
}

impl_binops_additive!(Scalar, Scalar);
impl_binops_multiplicative!(Scalar, Scalar);

impl fff::Field for Scalar {
    fn random<R: rand_core::RngCore>(rng: &mut R) -> Self {
        use fff::PrimeField;

        // The number of bits we should "shave" from a randomly sampled reputation.
        const REPR_SHAVE_BITS: usize = 256 - Scalar::NUM_BITS as usize;

        loop {
            let mut raw = blst_scalar::default();
            for i in 0..4 {
                raw.l[i] = rng.next_u64();
            }

            // Mask away the unused most-significant bits.
            raw.l[3] &= 0xffffffffffffffff >> REPR_SHAVE_BITS;

            if let Ok(valid_el) = raw.try_into() {
                return valid_el;
            }
        }
    }

    fn zero() -> Self {
        Scalar::from_raw_unchecked([0, 0, 0, 0])
    }

    fn one() -> Self {
        Scalar::from_raw_unchecked([1, 0, 0, 0])
    }

    fn is_zero(&self) -> bool {
        self == &Self::zero()
    }

    fn square(&mut self) {
        let mut raw = blst_fr::default();
        unsafe { blst_fr_sqr(&mut raw as _, &self.0 as _) }

        self.0 = raw;
    }

    fn double(&mut self) {
        *self += *self;
    }

    fn negate(&mut self) {
        *self = -&*self;
    }
    fn add_assign(&mut self, other: &Self) {
        *self += other;
    }

    fn sub_assign(&mut self, other: &Self) {
        *self -= other;
    }

    fn mul_assign(&mut self, other: &Self) {
        *self *= other;
    }

    fn inverse(&self) -> Option<Self> {
        todo!()
    }

    fn frobenius_map(&mut self, power: usize) {
        todo!()
    }
}

impl fff::PrimeField for Scalar {
    type Repr = ScalarRepr;

    const NUM_BITS: u32 = 256;
    const CAPACITY: u32 = Self::NUM_BITS - 1;
    const S: u32 = S;

    fn from_repr(_: Self::Repr) -> Result<Self, fff::PrimeFieldDecodingError> {
        todo!()
    }

    /// Convert a biginteger representation into a prime field element, if
    /// the number is an element of the field.
    fn into_repr(&self) -> Self::Repr {
        todo!()
    }

    fn char() -> Self::Repr {
        ScalarRepr(blst_scalar {
            l: [
                0xffffffff00000001,
                0x53bda402fffe5bfe,
                0x3339d80809a1d805,
                0x73eda753299d7d48,
            ],
        })
    }

    fn multiplicative_generator() -> Self {
        todo!()
    }

    fn root_of_unity() -> Self {
        todo!()
    }
}

impl fff::SqrtField for Scalar {
    fn legendre(&self) -> fff::LegendreSymbol {
        todo!()
    }

    fn sqrt(&self) -> Option<Self> {
        todo!()
    }
}

impl Scalar {
    /// Attempts to convert a little-endian byte representation of
    /// a scalar into a `Scalar`, failing if the input is not canonical.
    pub fn from_bytes_le(bytes: &[u8; 32]) -> Option<Scalar> {
        // TODO: figure out if there is a way to avoid this heap allocation
        let mut in_v = bytes.to_vec();
        let mut raw = blst_scalar::default();

        unsafe {
            blst_scalar_from_lendian(&mut raw as _, in_v.as_mut_ptr());
        }

        raw.try_into().ok()
    }

    /// Attempts to convert a big-endian byte representation of
    /// a scalar into a `Scalar`, failing if the input is not canonical.
    pub fn from_bytes_be(bytes: &[u8; 32]) -> Option<Scalar> {
        // TODO: figure out if there is a way to avoid this heap allocation
        let mut in_v = bytes.to_vec();
        let mut raw = blst_scalar::default();

        unsafe {
            blst_scalar_from_bendian(&mut raw as _, in_v.as_mut_ptr());
        }

        raw.try_into().ok()
    }

    /// Converts from an integer represented in little endian
    /// into its (congruent) `Scalar` representation.
    pub fn from_raw(val: [u64; 4]) -> Self {
        let mut original = blst_fr::default();
        original.l.copy_from_slice(&val);

        let mut raw = blst_fr::default();
        // Convert to montgomery form
        unsafe { blst_fr_to(&mut raw as _, &original as _) }

        Scalar(raw)
    }

    /// Converts from an integer represented in little endian, in Montgomery form, into a `Scalar`,
    /// without any checks
    pub fn from_raw_unchecked(val: [u64; 4]) -> Self {
        let mut raw = blst_fr::default();
        raw.l.copy_from_slice(&val);

        Scalar(raw)
    }

    /// Converts an element of `Scalar` into a byte representation in
    /// little-endian byte order.
    pub fn to_bytes_le(&self) -> [u8; 32] {
        // TODO: figure out if there is a way to avoid this heap allocation
        let mut out_v = vec![0u8; 32];
        // Safe because any valid blst_fr is also a valid blst_scalar.
        let scalar: blst_scalar = unsafe { std::mem::transmute(self.0) };

        unsafe {
            blst_lendian_from_scalar(out_v.as_mut_ptr(), &scalar);
        }

        let mut out = [0u8; 32];
        out.copy_from_slice(&out_v);

        out
    }

    /// Converts an element of `Scalar` into a byte representation in
    /// big-endian byte order.
    pub fn to_bytes_be(&self) -> [u8; 32] {
        // TODO: figure out if there is a way to avoid this heap allocation
        let mut out_v = vec![0u8; 32];
        // Safe because any valid blst_fr is also a valid blst_scalar.
        let scalar: blst_scalar = unsafe { std::mem::transmute(self.0) };
        unsafe {
            blst_bendian_from_scalar(out_v.as_mut_ptr(), &scalar);
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&out_v);

        out
    }

    /// Multiplies `rhs` by `self`, returning the result.
    #[inline]
    pub fn mul(&self, rhs: &Self) -> Self {
        let mut out = blst_fr::default();

        unsafe { blst_fr_mul(&mut out as _, &self.0 as _, &rhs.0 as _) };

        Scalar(out)
    }

    /// Subtracts `rhs` from `self`, returning the result.
    #[inline]
    pub fn sub(&self, rhs: &Self) -> Self {
        let mut out = blst_fr::default();

        unsafe { blst_fr_sub(&mut out as _, &self.0 as _, &rhs.0 as _) };

        Scalar(out)
    }

    /// Adds `rhs` to `self`, returning the result.
    #[inline]
    pub fn add(&self, rhs: &Self) -> Self {
        let mut out = blst_fr::default();

        unsafe { blst_fr_add(&mut out as _, &self.0 as _, &rhs.0 as _) };

        Scalar(out)
    }

    /// Returns true if this element is zero.
    pub fn is_zero(&self) -> bool {
        self.0.l.iter().all(|&e| e == 0)
    }

    /// Returns true if this element is a valid field element.
    pub fn is_valid(&self) -> bool {
        // Safe because all blst_fr are valid blst_scalar
        let scalar: &blst_scalar = unsafe { std::mem::transmute(&self.0) };

        unsafe { blst_scalar_fr_check(scalar as _) }
    }

    /// Multiplies `self` with `3`, returning the result.
    pub fn mul3(&self) -> Self {
        let mut out = blst_fr::default();

        unsafe { blst_fr_mul_by_3(&mut out as _, &self.0 as _) };

        Scalar(out)
    }

    /// Left shift `self` by `count`, returning the result.
    pub fn shl(&self, count: usize) -> Self {
        let mut out = blst_fr::default();

        unsafe { blst_fr_lshift(&mut out as _, &self.0 as _, count) };

        Scalar(out)
    }

    /// Right shift `self` by `count`, returning the result.
    pub fn shr(&self, count: usize) -> Self {
        let mut out = blst_fr::default();

        unsafe { blst_fr_rshift(&mut out as _, &self.0 as _, count) };

        Scalar(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use fff::{Field, SqrtField};
    use rand_core::SeedableRng;
    use rand_xorshift::XorShiftRng;

    /// INV = -(q^{-1} mod 2^64) mod 2^64
    const INV: u64 = 0xfffffffeffffffff;

    /// Constant representing the modulus
    /// q = 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001
    fn MODULUS() -> Scalar {
        Scalar::from_raw_unchecked([
            0xffffffff00000001,
            0x53bda402fffe5bfe,
            0x3339d80809a1d805,
            0x73eda753299d7d48,
        ])
    }

    /// R = 2^256 mod q
    fn R() -> Scalar {
        Scalar::from_raw_unchecked([
            0x00000001fffffffe,
            0x5884b7fa00034802,
            0x998c4fefecbc4ff5,
            0x1824b159acc5056f,
        ])
    }

    /// R^2 = 2^512 mod q
    fn R2() -> Scalar {
        Scalar::from_raw_unchecked([
            0xc999e990f3f29c6d,
            0x2b6cedcb87925c23,
            0x05d314967254398f,
            0x0748d9d99f59ff11,
        ])
    }

    fn LARGEST() -> Scalar {
        Scalar::from_raw_unchecked([
            0xffffffff00000000,
            0x53bda402fffe5bfe,
            0x3339d80809a1d805,
            0x73eda753299d7d48,
        ])
    }

    #[test]
    fn test_inv() {
        // Compute -(q^{-1} mod 2^64) mod 2^64 by exponentiating
        // by totient(2**64) - 1

        let mut inv = 1u64;
        for _ in 0..63 {
            inv = inv.wrapping_mul(inv);
            inv = inv.wrapping_mul(MODULUS().0.l[0]);
        }
        inv = inv.wrapping_neg();

        assert_eq!(inv, INV);
    }

    #[test]
    fn test_debug() {
        assert_eq!(
            format!("{:?}", Scalar::zero()),
            "0x0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(
            format!("{:?}", Scalar::one()),
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );
        assert_eq!(
            format!("{:?}", R()),
            "0x1824b159acc5056f998c4fefecbc4ff55884b7fa0003480200000001fffffffe"
        );
    }

    #[test]
    fn test_equality() {
        assert_eq!(Scalar::zero(), Scalar::zero());
        assert_eq!(Scalar::one(), Scalar::one());
        assert_eq!(R2(), R2());

        assert!(Scalar::zero() != Scalar::one());
        assert!(Scalar::one() != R2());
    }

    #[test]
    fn test_to_bytes() {
        assert_eq!(
            Scalar::zero().to_bytes_le(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );

        assert_eq!(
            Scalar::one().to_bytes_le(),
            [
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );

        assert_eq!(
            R().to_bytes_le(),
            [
                254, 255, 255, 255, 1, 0, 0, 0, 2, 72, 3, 0, 250, 183, 132, 88, 245, 79, 188, 236,
                239, 79, 140, 153, 111, 5, 197, 172, 89, 177, 36, 24
            ]
        );

        assert_eq!(
            (-&Scalar::one()).to_bytes_le(),
            [
                0, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9,
                8, 216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 115
            ]
        );
    }

    #[test]
    fn test_from_bytes() {
        assert_eq!(
            Scalar::from_bytes_le(&[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ])
            .unwrap(),
            Scalar::zero()
        );

        assert_eq!(
            Scalar::from_bytes_le(&[
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ])
            .unwrap(),
            Scalar::one()
        );

        assert_eq!(
            Scalar::from_bytes_le(&[
                254, 255, 255, 255, 1, 0, 0, 0, 2, 72, 3, 0, 250, 183, 132, 88, 245, 79, 188, 236,
                239, 79, 140, 153, 111, 5, 197, 172, 89, 177, 36, 24
            ])
            .unwrap(),
            R()
        );

        // -1 should work
        assert!(Scalar::from_bytes_le(&[
            0, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8,
            216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 115
        ])
        .is_some());

        // modulus is invalid
        assert!(Scalar::from_bytes_le(&[
            1, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8,
            216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 115
        ])
        .is_none());

        // Anything larger than the modulus is invalid
        assert!(Scalar::from_bytes_le(&[
            2, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8,
            216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 115
        ])
        .is_none());
        assert!(Scalar::from_bytes_le(&[
            1, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8,
            216, 58, 51, 72, 125, 157, 41, 83, 167, 237, 115
        ])
        .is_none());
        assert!(Scalar::from_bytes_le(&[
            1, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8,
            216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 116
        ])
        .is_none());
    }

    #[test]
    fn test_zero() {
        assert_eq!(Scalar::zero(), -&Scalar::zero());
        assert_eq!(Scalar::zero(), Scalar::zero() + Scalar::zero());
        assert_eq!(Scalar::zero(), Scalar::zero() - Scalar::zero());
        assert_eq!(Scalar::zero(), Scalar::zero() * Scalar::zero());
    }

    #[test]
    fn test_addition() {
        let mut tmp = LARGEST();
        tmp += &LARGEST();

        assert_eq!(
            tmp,
            Scalar::from_raw_unchecked([
                0xfffffffeffffffff,
                0x53bda402fffe5bfe,
                0x3339d80809a1d805,
                0x73eda753299d7d48
            ])
        );

        let mut tmp = LARGEST();
        tmp += &Scalar::from_raw_unchecked([1, 0, 0, 0]);

        assert_eq!(tmp, Scalar::zero());
    }

    #[test]
    fn test_negation() {
        let tmp = -&LARGEST();

        assert_eq!(tmp, Scalar::from_raw_unchecked([1, 0, 0, 0]));

        let tmp = -&Scalar::zero();
        assert_eq!(tmp, Scalar::zero());
        let tmp = -&Scalar::from_raw_unchecked([1, 0, 0, 0]);
        assert_eq!(tmp, LARGEST());

        {
            let mut a = Scalar::zero();
            a = a.neg();

            assert!(a.is_zero());
        }

        let mut rng = XorShiftRng::from_seed([
            0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06,
            0xbc, 0xe5,
        ]);

        for _ in 0..1000 {
            // Ensure (a - (-a)) = 0.
            let mut a = Scalar::random(&mut rng);
            let mut b = a;
            b = b.neg();
            a += &b;

            assert!(a.is_zero());
        }
    }

    #[test]
    fn test_subtraction() {
        let mut tmp = LARGEST();
        tmp -= &LARGEST();

        assert_eq!(tmp, Scalar::zero());

        let mut tmp = Scalar::zero();
        tmp -= &LARGEST();

        let mut tmp2 = MODULUS();
        tmp2 -= &LARGEST();

        assert_eq!(tmp, tmp2);
    }

    #[test]
    fn test_multiplication() {
        let mut tmp = Scalar::from_raw_unchecked([
            0x6b7e9b8faeefc81a,
            0xe30a8463f348ba42,
            0xeff3cb67a8279c9c,
            0x3d303651bd7c774d,
        ]);
        tmp *= &Scalar::from_raw_unchecked([
            0x13ae28e3bc35ebeb,
            0xa10f4488075cae2c,
            0x8160e95a853c3b5d,
            0x5ae3f03b561a841d,
        ]);
        assert!(
            tmp == Scalar::from_raw_unchecked([
                0x23717213ce710f71,
                0xdbee1fe53a16e1af,
                0xf565d3e1c2a48000,
                0x4426507ee75df9d7
            ])
        );

        let mut rng = XorShiftRng::from_seed([
            0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06,
            0xbc, 0xe5,
        ]);

        for _ in 0..1000000 {
            // Ensure that (a * b) * c = a * (b * c)
            let a = Scalar::random(&mut rng);
            let b = Scalar::random(&mut rng);
            let c = Scalar::random(&mut rng);

            let mut tmp1 = a;
            tmp1 *= &b;
            tmp1 *= &c;

            let mut tmp2 = b;
            tmp2 *= &c;
            tmp2 *= &a;

            assert_eq!(tmp1, tmp2);
        }

        for _ in 0..1000000 {
            // Ensure that r * (a + b + c) = r*a + r*b + r*c

            let r = Scalar::random(&mut rng);
            let mut a = Scalar::random(&mut rng);
            let mut b = Scalar::random(&mut rng);
            let mut c = Scalar::random(&mut rng);

            let mut tmp1 = a;
            tmp1 += &b;
            tmp1 += &c;
            tmp1 *= &r;

            a *= &r;
            b *= &r;
            c *= &r;

            a += &b;
            a += &c;

            assert_eq!(tmp1, a);
        }
    }

    #[test]
    fn test_squaring() {
        // FIXME: why does this fail?
        // let a = Scalar::from_raw_unchecked([
        //         0xffffffffffffffff,
        //         0xffffffffffffffff,
        //         0xffffffffffffffff,
        //         0x73eda753299d7d47,
        //     ],
        // );
        // assert!(a.is_valid());
        // assert_eq!(
        //     a.square(),
        //     Scalar::from_raw_unchecked([
        //             0xc0d698e7bde077b8,
        //             0xb79a310579e76ec2,
        //             0xac1da8d0a9af4e5f,
        //             0x13f629c49bf23e97
        //         ]
        //     )
        // );

        let mut rng = XorShiftRng::from_seed([
            0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06,
            0xbc, 0xe5,
        ]);

        for _ in 0..1000000 {
            // Ensure that (a * a) = a^2
            let a = Scalar::random(&mut rng);

            let mut tmp = a;
            tmp.square();

            let mut tmp2 = a;
            tmp2 *= &a;

            assert_eq!(tmp, tmp2);
        }
    }

    #[test]
    fn test_inversion() {
        assert!(Scalar::zero().inverse().is_none());
        assert_eq!(Scalar::one().inverse().unwrap(), Scalar::one());
        assert_eq!((-&Scalar::one()).inverse().unwrap(), -&Scalar::one());

        let mut tmp = R2();

        for _ in 0..100 {
            let mut tmp2 = tmp.inverse().unwrap();
            tmp2 *= &tmp;

            assert_eq!(tmp2, Scalar::one());

            tmp += &R2();
        }
    }

    #[test]
    fn test_inverse_is_pow() {
        let q_minus_2 = [
            0xfffffffeffffffff,
            0x53bda402fffe5bfe,
            0x3339d80809a1d805,
            0x73eda753299d7d48,
        ];

        let mut r1 = R();
        let mut r2 = R();

        for _ in 0..100 {
            r1 = r1.inverse().unwrap();
            r2 = r2.pow(&q_minus_2);

            assert_eq!(r1, r2);
            // Add R so we check something different next time around
            r1 += &R();
            r2 = r1;
        }
    }

    #[test]
    fn test_sqrt() {
        {
            assert_eq!(Scalar::zero().sqrt().unwrap(), Scalar::zero());
        }

        let mut square = Scalar::from_raw_unchecked([
            0x46cd85a5f273077e,
            0x1d30c47dd68fc735,
            0x77f656f60beca0eb,
            0x494aa01bdf32468d,
        ]);

        let mut none_count = 0;

        for _ in 0..100 {
            let square_root = square.sqrt();
            if square_root.is_none() {
                none_count += 1;
            } else {
                assert_eq!(square_root.unwrap() * square_root.unwrap(), square);
            }
            square -= Scalar::one();
        }

        assert_eq!(49, none_count);
    }

    #[test]
    fn test_from_raw() {
        assert_eq!(
            Scalar::from_raw([
                0x1fffffffd,
                0x5884b7fa00034802,
                0x998c4fefecbc4ff5,
                0x1824b159acc5056f
            ]),
            Scalar::from_raw([0xffffffffffffffff; 4])
        );

        assert_eq!(Scalar::from_raw(MODULUS().0.l), Scalar::zero());

        assert_eq!(Scalar::from_raw([1, 0, 0, 0]), R());
    }

    #[test]
    fn test_double() {
        let mut a = Scalar::from_raw([
            0x1fff3231233ffffd,
            0x4884b7fa00034802,
            0x998c4fefecbc4ff3,
            0x1824b159acc50562,
        ]);

        let mut b = a.clone();
        b.double();
        assert_eq!(b, a + a);
    }
}
