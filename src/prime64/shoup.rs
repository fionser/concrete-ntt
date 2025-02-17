use super::RECURSION_THRESHOLD;
#[allow(unused_imports)]
use crate::Butterfly;
use core::iter::zip;
use pulp::cast;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use pulp::x86::*;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[cfg(feature = "nightly")]
pub(crate) fn fwd_breadth_first_avx512<Simd: crate::SupersetOfV4>(
    simd: Simd,
    p: u64,
    data: &mut [u64],
    twid: &[u64],
    twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<Simd, u64x8>,
    last_butterfly: impl Butterfly<Simd, u64x8>,
) {
    struct Impl<'a, Simd, B, LB> {
        simd: Simd,
        p: u64,
        data: &'a mut [u64],
        twid: &'a [u64],
        twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<Simd: crate::SupersetOfV4, B: Butterfly<Simd, u64x8>, LB: Butterfly<Simd, u64x8>>
        pulp::NullaryFnOnce for Impl<'_, Simd, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                twid,
                twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;
            let v4 = simd.get_v4();
            let n = data.len();
            debug_assert!(n.is_power_of_two());

            let mut t = n;
            let mut m = 1;
            let mut w_idx = (m << recursion_depth) + recursion_half * m;

            let neg_p = v4.splat_u64x8(p.wrapping_neg());
            let two_p = v4.splat_u64x8(2 * p);
            let p = v4.splat_u64x8(p);

            while m < n / 8 {
                t /= 2;

                let w = &twid[w_idx..];
                let w_shoup = &twid_shoup[w_idx..];

                for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                    let (z0, z1) = data.split_at_mut(t);
                    let z0 = pulp::as_arrays_mut::<8, _>(z0).0;
                    let z1 = pulp::as_arrays_mut::<8, _>(z1).0;
                    let w = v4.splat_u64x8(w);
                    let w_shoup = v4.splat_u64x8(w_shoup);

                    for (z0_, z1_) in zip(z0, z1) {
                        let mut z0 = cast(*z0_);
                        let mut z1 = cast(*z1_);
                        (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                        *z0_ = cast(z0);
                        *z1_ = cast(z1);
                    }
                }

                m *= 2;
                w_idx *= 2;
            }

            // m = n / 8
            // t = 4
            {
                let w = pulp::as_arrays::<2, _>(&twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<2, _>(&twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<8, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z0z0z0z1z1z1z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = v4.permute4_u64x8(*w);
                    let w_shoup = v4.permute4_u64x8(*w_shoup);
                    let [mut z0, mut z1] = v4.interleave4_u64x8(cast(*z0z0z0z0z1z1z1z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z0z0z0z1z1z1z1 = cast(v4.interleave4_u64x8([z0, z1]));
                }

                w_idx *= 2;
            }

            // m = n / 4
            // t = 2
            {
                let w = pulp::as_arrays::<4, _>(&twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<4, _>(&twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<8, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z0z1z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = v4.permute2_u64x8(*w);
                    let w_shoup = v4.permute2_u64x8(*w_shoup);
                    let [mut z0, mut z1] = v4.interleave2_u64x8(cast(*z0z0z1z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z0z1z1 = cast(v4.interleave2_u64x8([z0, z1]));
                }

                w_idx *= 2;
            }

            // m = n / 2
            // t = 1
            {
                let w = pulp::as_arrays::<8, _>(&twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<8, _>(&twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<8, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = v4.permute1_u64x8(*w);
                    let w_shoup = v4.permute1_u64x8(*w_shoup);
                    let [mut z0, mut z1] = v4.interleave1_u64x8(cast(*z0z1));
                    (z0, z1) = last_butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z1 = cast(v4.interleave1_u64x8([z0, z1]));
                }
            }
        }
    }

    simd.vectorize(Impl {
        simd,
        p,
        data,
        twid,
        twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[cfg(feature = "nightly")]
pub(crate) fn fwd_depth_first_avx512<Simd: crate::SupersetOfV4>(
    simd: Simd,
    p: u64,
    data: &mut [u64],
    twid: &[u64],
    twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<Simd, u64x8>,
    last_butterfly: impl Butterfly<Simd, u64x8>,
) {
    struct Impl<'a, Simd, B, LB> {
        simd: Simd,
        p: u64,
        data: &'a mut [u64],
        twid: &'a [u64],
        twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<Simd: crate::SupersetOfV4, B: Butterfly<Simd, u64x8>, LB: Butterfly<Simd, u64x8>>
        pulp::NullaryFnOnce for Impl<'_, Simd, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                twid,
                twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;

            let v4 = simd.get_v4();
            let n = data.len();
            debug_assert!(n.is_power_of_two());

            if n <= RECURSION_THRESHOLD {
                fwd_breadth_first_avx512(
                    simd,
                    p,
                    data,
                    twid,
                    twid_shoup,
                    recursion_depth,
                    recursion_half,
                    butterfly,
                    last_butterfly,
                );
            } else {
                let t = n / 2;
                let m = 1;
                let w_idx = (m << recursion_depth) + m * recursion_half;

                let w = &twid[w_idx..];
                let w_shoup = &twid_shoup[w_idx..];

                {
                    let neg_p = v4.splat_u64x8(p.wrapping_neg());
                    let two_p = v4.splat_u64x8(2 * p);
                    let p = v4.splat_u64x8(p);

                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<8, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<8, _>(z1).0;
                        let w = v4.splat_u64x8(w);
                        let w_shoup = v4.splat_u64x8(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                }

                let (data0, data1) = data.split_at_mut(n / 2);
                fwd_depth_first_avx512(
                    simd,
                    p,
                    data0,
                    twid,
                    twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2,
                    butterfly,
                    last_butterfly,
                );
                fwd_depth_first_avx512(
                    simd,
                    p,
                    data1,
                    twid,
                    twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2 + 1,
                    butterfly,
                    last_butterfly,
                );
            }
        }
    }

    simd.vectorize(Impl {
        simd,
        p,
        data,
        twid,
        twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) fn fwd_breadth_first_avx2(
    simd: crate::V3,
    p: u64,
    data: &mut [u64],
    twid: &[u64],
    twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<crate::V3, u64x4>,
    last_butterfly: impl Butterfly<crate::V3, u64x4>,
) {
    struct Impl<'a, B, LB> {
        simd: crate::V3,
        p: u64,
        data: &'a mut [u64],
        twid: &'a [u64],
        twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<B: Butterfly<crate::V3, u64x4>, LB: Butterfly<crate::V3, u64x4>> pulp::NullaryFnOnce
        for Impl<'_, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                twid,
                twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;
            let n = data.len();
            debug_assert!(n.is_power_of_two());

            let mut t = n;
            let mut m = 1;
            let mut w_idx = (m << recursion_depth) + recursion_half * m;

            let neg_p = simd.splat_u64x4(p.wrapping_neg());
            let two_p = simd.splat_u64x4(2 * p);
            let p = simd.splat_u64x4(p);

            while m < n / 4 {
                t /= 2;

                let w = &twid[w_idx..];
                let w_shoup = &twid_shoup[w_idx..];

                for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                    let (z0, z1) = data.split_at_mut(t);
                    let z0 = pulp::as_arrays_mut::<4, _>(z0).0;
                    let z1 = pulp::as_arrays_mut::<4, _>(z1).0;
                    let w = simd.splat_u64x4(w);
                    let w_shoup = simd.splat_u64x4(w_shoup);

                    for (z0_, z1_) in zip(z0, z1) {
                        let mut z0 = cast(*z0_);
                        let mut z1 = cast(*z1_);
                        (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                        *z0_ = cast(z0);
                        *z1_ = cast(z1);
                    }
                }

                m *= 2;
                w_idx *= 2;
            }

            // m = n / 4
            // t = 2
            {
                let w = pulp::as_arrays::<2, _>(&twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<2, _>(&twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<4, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z0z1z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = simd.permute2_u64x4(*w);
                    let w_shoup = simd.permute2_u64x4(*w_shoup);
                    let [mut z0, mut z1] = simd.interleave2_u64x4(cast(*z0z0z1z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z0z1z1 = cast(simd.interleave2_u64x4([z0, z1]));
                }

                w_idx *= 2;
            }

            // m = n / 2
            // t = 1
            {
                let w = pulp::as_arrays::<4, _>(&twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<4, _>(&twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<4, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = simd.permute1_u64x4(*w);
                    let w_shoup = simd.permute1_u64x4(*w_shoup);
                    let [mut z0, mut z1] = simd.interleave1_u64x4(cast(*z0z1));
                    (z0, z1) = last_butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z1 = cast(simd.interleave1_u64x4([z0, z1]));
                }
            }
        }
    }
    simd.vectorize(Impl {
        simd,
        p,
        data,
        twid,
        twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) fn fwd_depth_first_avx2(
    simd: crate::V3,
    p: u64,
    data: &mut [u64],
    twid: &[u64],
    twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<crate::V3, u64x4>,
    last_butterfly: impl Butterfly<crate::V3, u64x4>,
) {
    struct Impl<'a, B, LB> {
        simd: crate::V3,
        p: u64,
        data: &'a mut [u64],
        twid: &'a [u64],
        twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<B: Butterfly<crate::V3, u64x4>, LB: Butterfly<crate::V3, u64x4>> pulp::NullaryFnOnce
        for Impl<'_, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                twid,
                twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;

            let n = data.len();
            debug_assert!(n.is_power_of_two());

            if n <= RECURSION_THRESHOLD {
                fwd_breadth_first_avx2(
                    simd,
                    p,
                    data,
                    twid,
                    twid_shoup,
                    recursion_depth,
                    recursion_half,
                    butterfly,
                    last_butterfly,
                );
            } else {
                let t = n / 2;
                let m = 1;
                let w_idx = (m << recursion_depth) + m * recursion_half;

                let w = &twid[w_idx..];
                let w_shoup = &twid_shoup[w_idx..];

                {
                    let neg_p = simd.splat_u64x4(p.wrapping_neg());
                    let two_p = simd.splat_u64x4(2 * p);
                    let p = simd.splat_u64x4(p);

                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<4, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<4, _>(z1).0;
                        let w = simd.splat_u64x4(w);
                        let w_shoup = simd.splat_u64x4(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                }

                let (data0, data1) = data.split_at_mut(n / 2);
                fwd_depth_first_avx2(
                    simd,
                    p,
                    data0,
                    twid,
                    twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2,
                    butterfly,
                    last_butterfly,
                );
                fwd_depth_first_avx2(
                    simd,
                    p,
                    data1,
                    twid,
                    twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2 + 1,
                    butterfly,
                    last_butterfly,
                );
            }
        }
    }

    simd.vectorize(Impl {
        simd,
        p,
        data,
        twid,
        twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

pub fn fwd_breadth_first_scalar(
    p: u64,
    data: &mut [u64],
    twid: &[u64],
    twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
    last_butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());

    let mut t = n;
    let mut m = 1;
    let mut w_idx = (m << recursion_depth) + recursion_half * m;

    let neg_p = p.wrapping_neg();
    let two_p = 2 * p;

    while m < n {
        t /= 2;

        let w = &twid[w_idx..];
        let w_shoup = &twid_shoup[w_idx..];

        if t == 1 {
            for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                let (z0, z1) = data.split_at_mut(t);
                for (z0_, z1_) in zip(z0, z1) {
                    let mut z0 = *z0_;
                    let mut z1 = *z1_;
                    (z0, z1) = last_butterfly(z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0_ = z0;
                    *z1_ = z1;
                }
            }
        } else {
            for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                let (z0, z1) = data.split_at_mut(t);
                for (z0_, z1_) in zip(z0, z1) {
                    let mut z0 = *z0_;
                    let mut z1 = *z1_;
                    (z0, z1) = butterfly(z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0_ = z0;
                    *z1_ = z1;
                }
            }
        }

        m *= 2;
        w_idx *= 2;
    }
}

pub fn fwd_depth_first_scalar(
    p: u64,
    data: &mut [u64],
    twid: &[u64],
    twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
    last_butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());

    if n <= RECURSION_THRESHOLD {
        fwd_breadth_first_scalar(
            p,
            data,
            twid,
            twid_shoup,
            recursion_depth,
            recursion_half,
            butterfly,
            last_butterfly,
        );
    } else {
        let t = n / 2;
        let m = 1;
        let w_idx = (m << recursion_depth) + m * recursion_half;

        let w = &twid[w_idx..];
        let w_shoup = &twid_shoup[w_idx..];

        {
            let neg_p = p.wrapping_neg();
            let two_p = 2 * p;

            for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                let (z0, z1) = data.split_at_mut(t);

                for (z0_, z1_) in zip(z0, z1) {
                    let mut z0 = cast(*z0_);
                    let mut z1 = cast(*z1_);
                    (z0, z1) = butterfly(z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0_ = cast(z0);
                    *z1_ = cast(z1);
                }
            }
        }

        let (data0, data1) = data.split_at_mut(n / 2);
        fwd_depth_first_scalar(
            p,
            data0,
            twid,
            twid_shoup,
            recursion_depth + 1,
            recursion_half * 2,
            butterfly,
            last_butterfly,
        );
        fwd_depth_first_scalar(
            p,
            data1,
            twid,
            twid_shoup,
            recursion_depth + 1,
            recursion_half * 2 + 1,
            butterfly,
            last_butterfly,
        );
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[cfg(feature = "nightly")]
pub(crate) fn inv_breadth_first_avx512<Simd: crate::SupersetOfV4>(
    simd: Simd,
    p: u64,
    data: &mut [u64],
    inv_twid: &[u64],
    inv_twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<Simd, u64x8>,
    last_butterfly: impl Butterfly<Simd, u64x8>,
) {
    struct Impl<'a, Simd, B, LB> {
        simd: Simd,
        p: u64,
        data: &'a mut [u64],
        inv_twid: &'a [u64],
        inv_twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<Simd: crate::SupersetOfV4, B: Butterfly<Simd, u64x8>, LB: Butterfly<Simd, u64x8>>
        pulp::NullaryFnOnce for Impl<'_, Simd, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                inv_twid,
                inv_twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;

            let v4 = simd.get_v4();
            let n = data.len();
            debug_assert!(n.is_power_of_two());

            let mut t = 1;
            let mut m = n;
            let mut w_idx = (m << recursion_depth) + recursion_half * m;

            let neg_p = v4.splat_u64x8(p.wrapping_neg());
            let two_p = v4.splat_u64x8(2 * p);
            let p = v4.splat_u64x8(p);

            // m = n / 2
            // t = 1
            {
                m /= 2;
                w_idx /= 2;

                let w = pulp::as_arrays::<8, _>(&inv_twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<8, _>(&inv_twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<8, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = v4.permute1_u64x8(*w);
                    let w_shoup = v4.permute1_u64x8(*w_shoup);
                    let [mut z0, mut z1] = v4.interleave1_u64x8(cast(*z0z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z1 = cast(v4.interleave1_u64x8([z0, z1]));
                }

                t *= 2;
            }

            // m = n / 4
            // t = 2
            {
                m /= 2;
                w_idx /= 2;

                let w = pulp::as_arrays::<4, _>(&inv_twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<4, _>(&inv_twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<8, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z0z1z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = v4.permute2_u64x8(*w);
                    let w_shoup = v4.permute2_u64x8(*w_shoup);
                    let [mut z0, mut z1] = v4.interleave2_u64x8(cast(*z0z0z1z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z0z1z1 = cast(v4.interleave2_u64x8([z0, z1]));
                }

                t *= 2;
            }

            // m = n / 8
            // t = 4
            {
                m /= 2;
                w_idx /= 2;

                let w = pulp::as_arrays::<2, _>(&inv_twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<2, _>(&inv_twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<8, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z0z0z0z1z1z1z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = v4.permute4_u64x8(*w);
                    let w_shoup = v4.permute4_u64x8(*w_shoup);
                    let [mut z0, mut z1] = v4.interleave4_u64x8(cast(*z0z0z0z0z1z1z1z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z0z0z0z1z1z1z1 = cast(v4.interleave4_u64x8([z0, z1]));
                }

                t *= 2;
            }

            while m > 1 {
                m /= 2;
                w_idx /= 2;

                let w = &inv_twid[w_idx..];
                let w_shoup = &inv_twid_shoup[w_idx..];

                if m == 1 {
                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<8, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<8, _>(z1).0;
                        let w = v4.splat_u64x8(w);
                        let w_shoup = v4.splat_u64x8(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = last_butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                } else {
                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<8, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<8, _>(z1).0;
                        let w = v4.splat_u64x8(w);
                        let w_shoup = v4.splat_u64x8(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                }

                t *= 2;
            }
        }
    }

    simd.vectorize(Impl {
        simd,
        p,
        data,
        inv_twid,
        inv_twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[cfg(feature = "nightly")]
pub(crate) fn inv_depth_first_avx512<Simd: crate::SupersetOfV4>(
    simd: Simd,
    p: u64,
    data: &mut [u64],
    inv_twid: &[u64],
    inv_twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<Simd, u64x8>,
    last_butterfly: impl Butterfly<Simd, u64x8>,
) {
    struct Impl<'a, Simd, B, LB> {
        simd: Simd,
        p: u64,
        data: &'a mut [u64],
        inv_twid: &'a [u64],
        inv_twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<Simd: crate::SupersetOfV4, B: Butterfly<Simd, u64x8>, LB: Butterfly<Simd, u64x8>>
        pulp::NullaryFnOnce for Impl<'_, Simd, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                inv_twid,
                inv_twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;

            let v4 = simd.get_v4();
            let n = data.len();
            debug_assert!(n.is_power_of_two());

            if n <= RECURSION_THRESHOLD {
                inv_breadth_first_avx512(
                    simd,
                    p,
                    data,
                    inv_twid,
                    inv_twid_shoup,
                    recursion_depth,
                    recursion_half,
                    butterfly,
                    last_butterfly,
                );
            } else {
                let (data0, data1) = data.split_at_mut(n / 2);
                inv_depth_first_avx512(
                    simd,
                    p,
                    data0,
                    inv_twid,
                    inv_twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2,
                    butterfly,
                    butterfly,
                );
                inv_depth_first_avx512(
                    simd,
                    p,
                    data1,
                    inv_twid,
                    inv_twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2 + 1,
                    butterfly,
                    butterfly,
                );

                let t = n / 2;
                let m = 1;
                let w_idx = (m << recursion_depth) + m * recursion_half;

                let w = &inv_twid[w_idx..];
                let w_shoup = &inv_twid_shoup[w_idx..];

                {
                    let neg_p = v4.splat_u64x8(p.wrapping_neg());
                    let two_p = v4.splat_u64x8(2 * p);
                    let p = v4.splat_u64x8(p);

                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<8, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<8, _>(z1).0;
                        let w = v4.splat_u64x8(w);
                        let w_shoup = v4.splat_u64x8(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = last_butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                }
            }
        }
    }

    simd.vectorize(Impl {
        simd,
        p,
        data,
        inv_twid,
        inv_twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) fn inv_breadth_first_avx2(
    simd: crate::V3,
    p: u64,
    data: &mut [u64],
    inv_twid: &[u64],
    inv_twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<crate::V3, u64x4>,
    last_butterfly: impl Butterfly<crate::V3, u64x4>,
) {
    struct Impl<'a, B, LB> {
        simd: crate::V3,
        p: u64,
        data: &'a mut [u64],
        inv_twid: &'a [u64],
        inv_twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<B: Butterfly<crate::V3, u64x4>, LB: Butterfly<crate::V3, u64x4>> pulp::NullaryFnOnce
        for Impl<'_, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                inv_twid,
                inv_twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;

            let n = data.len();
            debug_assert!(n.is_power_of_two());

            let mut t = 1;
            let mut m = n;
            let mut w_idx = (m << recursion_depth) + recursion_half * m;

            let neg_p = simd.splat_u64x4(p.wrapping_neg());
            let two_p = simd.splat_u64x4(2 * p);
            let p = simd.splat_u64x4(p);

            // m = n / 2
            // t = 1
            {
                m /= 2;
                w_idx /= 2;

                let w = pulp::as_arrays::<4, _>(&inv_twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<4, _>(&inv_twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<4, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = simd.permute1_u64x4(*w);
                    let w_shoup = simd.permute1_u64x4(*w_shoup);
                    let [mut z0, mut z1] = simd.interleave1_u64x4(cast(*z0z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z1 = cast(simd.interleave1_u64x4([z0, z1]));
                }

                t *= 2;
            }

            // m = n / 4
            // t = 2
            {
                m /= 2;
                w_idx /= 2;

                let w = pulp::as_arrays::<2, _>(&inv_twid[w_idx..]).0;
                let w_shoup = pulp::as_arrays::<2, _>(&inv_twid_shoup[w_idx..]).0;
                let data = pulp::as_arrays_mut::<4, _>(data).0;
                let data = pulp::as_arrays_mut::<2, _>(data).0;

                for (z0z0z1z1, (w, w_shoup)) in zip(data, zip(w, w_shoup)) {
                    let w = simd.permute2_u64x4(*w);
                    let w_shoup = simd.permute2_u64x4(*w_shoup);
                    let [mut z0, mut z1] = simd.interleave2_u64x4(cast(*z0z0z1z1));
                    (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0z0z1z1 = cast(simd.interleave2_u64x4([z0, z1]));
                }

                t *= 2;
            }

            while m > 1 {
                m /= 2;
                w_idx /= 2;

                let w = &inv_twid[w_idx..];
                let w_shoup = &inv_twid_shoup[w_idx..];

                if m == 1 {
                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<4, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<4, _>(z1).0;
                        let w = simd.splat_u64x4(w);
                        let w_shoup = simd.splat_u64x4(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = last_butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                } else {
                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<4, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<4, _>(z1).0;
                        let w = simd.splat_u64x4(w);
                        let w_shoup = simd.splat_u64x4(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                }

                t *= 2;
            }
        }
    }

    simd.vectorize(Impl {
        simd,
        p,
        data,
        inv_twid,
        inv_twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) fn inv_depth_first_avx2(
    simd: crate::V3,
    p: u64,
    data: &mut [u64],
    inv_twid: &[u64],
    inv_twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Butterfly<crate::V3, u64x4>,
    last_butterfly: impl Butterfly<crate::V3, u64x4>,
) {
    struct Impl<'a, B, LB> {
        simd: crate::V3,
        p: u64,
        data: &'a mut [u64],
        inv_twid: &'a [u64],
        inv_twid_shoup: &'a [u64],
        recursion_depth: usize,
        recursion_half: usize,
        butterfly: B,
        last_butterfly: LB,
    }
    impl<B: Butterfly<crate::V3, u64x4>, LB: Butterfly<crate::V3, u64x4>> pulp::NullaryFnOnce
        for Impl<'_, B, LB>
    {
        type Output = ();

        #[inline(always)]
        fn call(self) -> Self::Output {
            let Self {
                simd,
                p,
                data,
                inv_twid,
                inv_twid_shoup,
                recursion_depth,
                recursion_half,
                butterfly,
                last_butterfly,
            } = self;

            let n = data.len();
            debug_assert!(n.is_power_of_two());

            if n <= RECURSION_THRESHOLD {
                inv_breadth_first_avx2(
                    simd,
                    p,
                    data,
                    inv_twid,
                    inv_twid_shoup,
                    recursion_depth,
                    recursion_half,
                    butterfly,
                    last_butterfly,
                );
            } else {
                let (data0, data1) = data.split_at_mut(n / 2);
                inv_depth_first_avx2(
                    simd,
                    p,
                    data0,
                    inv_twid,
                    inv_twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2,
                    butterfly,
                    butterfly,
                );
                inv_depth_first_avx2(
                    simd,
                    p,
                    data1,
                    inv_twid,
                    inv_twid_shoup,
                    recursion_depth + 1,
                    recursion_half * 2 + 1,
                    butterfly,
                    butterfly,
                );

                let t = n / 2;
                let m = 1;
                let w_idx = (m << recursion_depth) + m * recursion_half;

                let w = &inv_twid[w_idx..];
                let w_shoup = &inv_twid_shoup[w_idx..];

                {
                    let neg_p = simd.splat_u64x4(p.wrapping_neg());
                    let two_p = simd.splat_u64x4(2 * p);
                    let p = simd.splat_u64x4(p);

                    for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup))
                    {
                        let (z0, z1) = data.split_at_mut(t);
                        let z0 = pulp::as_arrays_mut::<4, _>(z0).0;
                        let z1 = pulp::as_arrays_mut::<4, _>(z1).0;
                        let w = simd.splat_u64x4(w);
                        let w_shoup = simd.splat_u64x4(w_shoup);

                        for (z0_, z1_) in zip(z0, z1) {
                            let mut z0 = cast(*z0_);
                            let mut z1 = cast(*z1_);
                            (z0, z1) = last_butterfly(simd, z0, z1, w, w_shoup, p, neg_p, two_p);
                            *z0_ = cast(z0);
                            *z1_ = cast(z1);
                        }
                    }
                }
            }
        }
    }

    simd.vectorize(Impl {
        simd,
        p,
        data,
        inv_twid,
        inv_twid_shoup,
        recursion_depth,
        recursion_half,
        butterfly,
        last_butterfly,
    });
}

pub fn inv_breadth_first_scalar(
    p: u64,
    data: &mut [u64],
    inv_twid: &[u64],
    inv_twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
    last_butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());

    let mut t = 1;
    let mut m = n;
    let mut w_idx = (m << recursion_depth) + recursion_half * m;

    let neg_p = p.wrapping_neg();
    let two_p = 2 * p;

    while m > 1 {
        m /= 2;
        w_idx /= 2;

        let w = &inv_twid[w_idx..];
        let w_shoup = &inv_twid_shoup[w_idx..];

        if m == 1 {
            for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                let (z0, z1) = data.split_at_mut(t);
                for (z0_, z1_) in zip(z0, z1) {
                    let mut z0 = *z0_;
                    let mut z1 = *z1_;
                    (z0, z1) = last_butterfly(z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0_ = z0;
                    *z1_ = z1;
                }
            }
        } else {
            for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                let (z0, z1) = data.split_at_mut(t);
                for (z0_, z1_) in zip(z0, z1) {
                    let mut z0 = *z0_;
                    let mut z1 = *z1_;
                    (z0, z1) = butterfly(z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0_ = z0;
                    *z1_ = z1;
                }
            }
        }

        t *= 2;
    }
}

pub fn inv_depth_first_scalar(
    p: u64,
    data: &mut [u64],
    inv_twid: &[u64],
    inv_twid_shoup: &[u64],
    recursion_depth: usize,
    recursion_half: usize,
    butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
    last_butterfly: impl Copy
        + Fn(
            /* z0 */ u64,
            /* z1 */ u64,
            /* w */ u64,
            /* w_shoup */ u64,
            /* p */ u64,
            /* neg_p */ u64,
            /* two_p */ u64,
        ) -> (u64, u64),
) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());

    if n <= RECURSION_THRESHOLD {
        inv_breadth_first_scalar(
            p,
            data,
            inv_twid,
            inv_twid_shoup,
            recursion_depth,
            recursion_half,
            butterfly,
            last_butterfly,
        );
    } else {
        let (data0, data1) = data.split_at_mut(n / 2);
        inv_depth_first_scalar(
            p,
            data0,
            inv_twid,
            inv_twid_shoup,
            recursion_depth + 1,
            recursion_half * 2,
            butterfly,
            butterfly,
        );
        inv_depth_first_scalar(
            p,
            data1,
            inv_twid,
            inv_twid_shoup,
            recursion_depth + 1,
            recursion_half * 2 + 1,
            butterfly,
            butterfly,
        );

        let t = n / 2;
        let m = 1;
        let w_idx = (m << recursion_depth) + m * recursion_half;

        let w = &inv_twid[w_idx..];
        let w_shoup = &inv_twid_shoup[w_idx..];

        {
            let neg_p = p.wrapping_neg();
            let two_p = 2 * p;

            for (data, (&w, &w_shoup)) in zip(data.chunks_exact_mut(2 * t), zip(w, w_shoup)) {
                let (z0, z1) = data.split_at_mut(t);

                for (z0_, z1_) in zip(z0, z1) {
                    let mut z0 = cast(*z0_);
                    let mut z1 = cast(*z1_);
                    (z0, z1) = last_butterfly(z0, z1, w, w_shoup, p, neg_p, two_p);
                    *z0_ = cast(z0);
                    *z1_ = cast(z1);
                }
            }
        }
    }
}
